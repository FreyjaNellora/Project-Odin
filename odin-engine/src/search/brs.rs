// BRS search — Stage 7
//
// Implements Best-Reply Search (BRS) with alpha-beta pruning, iterative
// deepening, quiescence search, aspiration windows, null move pruning,
// late move reductions, and PV tracking.
//
// BRS tree structure (4-player, natural turn order R→B→Y→G→R→...):
//
//   MAX node (root player's turn):
//     Explore ALL legal moves. Standard alpha-beta. Updates alpha on improvement.
//     Prunes remaining moves when alpha >= beta (beta cutoff).
//
//   MIN node (each opponent's turn):
//     Select the SINGLE strongest reply for this opponent (the move that
//     minimizes root player's static eval). Play it. Recurse once. No branching.
//     Alpha/beta constraints pass through unchanged to the child MAX node.
//
// All scores are always from root_player's perspective (not negamax).
// Large positive = good for root player. Large negative = bad for root player.
//
// ADR-012: Natural turn order chosen over the MASTERPLAN's alternating
// MAX-MIN-MAX-MIN model because unmake_move() derives the previous player from
// prev_player(side_to_move) without storing it in MoveUndo. Manual
// set_side_to_move() between make and unmake corrupts restoration.

use std::time::Instant;

use crate::board::{PieceType, Player};
use crate::eval::Evaluator;
use crate::gamestate::GameState;
use crate::movegen::{generate_legal, is_in_check, make_move, unmake_move, Move};

use super::{SearchBudget, SearchResult, Searcher};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Hard cap on search depth.
const MAX_DEPTH: usize = 64;

/// Aspiration window initial half-width in centipawns.
const ASPIRATION_WINDOW: i16 = 50;

/// Null move reduction factor (R). Null move searches at depth - 1 - R.
const NULL_MOVE_REDUCTION: u8 = 2;

/// Minimum depth required to apply null move pruning.
const NULL_MOVE_MIN_DEPTH: u8 = NULL_MOVE_REDUCTION + 1;

/// Maximum extra plies in quiescence search.
const MAX_QSEARCH_DEPTH: u8 = 8;

/// Minimum depth at which late move reductions apply.
const LMR_MIN_DEPTH: u8 = 3;

/// Number of moves tried at full depth before LMR starts reducing.
const LMR_MOVE_THRESHOLD: usize = 3;

/// Score representing a forced mate (found, not given away).
/// Adjusted by ply to prefer shorter mates.
const MATE_SCORE: i16 = 20_000;

/// Lower bound for alpha (used as -infinity). Avoids i16::MIN negation overflow.
const NEG_INF: i16 = -30_000;

/// Upper bound for beta (used as +infinity).
const POS_INF: i16 = 30_000;

/// Check time/node budget every this many nodes.
const TIME_CHECK_INTERVAL: u64 = 1_024;

// ---------------------------------------------------------------------------
// BrsSearcher
// ---------------------------------------------------------------------------

/// Implements the `Searcher` trait using Best-Reply Search with alpha-beta.
pub struct BrsSearcher {
    evaluator: Box<dyn Evaluator>,
    info_cb: Option<Box<dyn FnMut(String)>>,
}

impl BrsSearcher {
    /// Create a new BrsSearcher with the given evaluator.
    pub fn new(evaluator: Box<dyn Evaluator>) -> Self {
        Self {
            evaluator,
            info_cb: None,
        }
    }

    /// Create a new BrsSearcher with a real-time info callback.
    ///
    /// The callback receives a formatted `info` string after each completed
    /// iterative deepening depth. The protocol can use this to emit progress.
    pub fn with_info_callback(
        evaluator: Box<dyn Evaluator>,
        cb: Box<dyn FnMut(String)>,
    ) -> Self {
        Self {
            evaluator,
            info_cb: Some(cb),
        }
    }
}

impl Searcher for BrsSearcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult {
        let mut ctx = BrsContext::new(position, self.evaluator.as_ref(), &budget);
        ctx.iterative_deepening(&mut self.info_cb)
    }
}

// ---------------------------------------------------------------------------
// BrsContext — per-search mutable state
// ---------------------------------------------------------------------------

/// All mutable state for a single search invocation.
///
/// Created inside `BrsSearcher::search()` and lives for the duration of one
/// `go` command. The `gs` field is a clone of the input position; the original
/// is never modified.
struct BrsContext<'a> {
    /// Working copy of the game state. Board is mutated in-place during search.
    gs: GameState,
    evaluator: &'a dyn Evaluator,
    /// The player who called `go` (the player we are searching for).
    root_player: Player,
    budget: &'a SearchBudget,
    /// Total nodes visited across all depths.
    nodes: u64,
    /// Wall-clock time when the search started.
    start: Instant,
    /// Set to true when a time or node limit is exceeded.
    stopped: bool,
    /// Triangular PV table: `pv_table[ply][i]` = i-th move in the PV from ply.
    pv_table: [[Option<Move>; MAX_DEPTH]; MAX_DEPTH],
    /// Number of valid entries in `pv_table[ply]`.
    pv_len: [usize; MAX_DEPTH],
    /// PV extracted from the last fully completed depth.
    best_pv: Vec<Move>,
    /// Score from the last fully completed depth.
    best_score: i16,
    /// Last fully completed depth.
    best_depth: u8,
}

impl<'a> BrsContext<'a> {
    fn new(position: &GameState, evaluator: &'a dyn Evaluator, budget: &'a SearchBudget) -> Self {
        Self {
            gs: position.clone(),
            evaluator,
            root_player: position.current_player(),
            budget,
            nodes: 0,
            start: Instant::now(),
            stopped: false,
            pv_table: [[None; MAX_DEPTH]; MAX_DEPTH],
            pv_len: [0; MAX_DEPTH],
            best_pv: Vec::new(),
            best_score: 0,
            best_depth: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Time / budget management
    // -----------------------------------------------------------------------

    fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    fn check_limits(&mut self) {
        if let Some(limit_ms) = self.budget.max_time_ms {
            if self.elapsed_ms() >= limit_ms {
                self.stopped = true;
                return;
            }
        }
        if let Some(limit_nodes) = self.budget.max_nodes {
            if self.nodes >= limit_nodes {
                self.stopped = true;
            }
        }
    }

    fn should_stop(&self) -> bool {
        self.stopped
    }

    // -----------------------------------------------------------------------
    // Iterative deepening
    // -----------------------------------------------------------------------

    fn iterative_deepening(
        &mut self,
        info_cb: &mut Option<Box<dyn FnMut(String)>>,
    ) -> SearchResult {
        let max_depth = self.budget.max_depth.unwrap_or(MAX_DEPTH as u8);

        // Seed with a legal move for the safety fallback (if search stops before
        // completing depth 1).
        let root_moves = generate_legal(self.gs.board_mut());
        assert!(
            !root_moves.is_empty(),
            "BrsSearcher::search called with no legal moves"
        );
        self.best_pv = vec![root_moves[0]];
        self.best_score = self.evaluator.eval_scalar(&self.gs, self.root_player);

        let mut prev_score = self.best_score;

        for depth in 1..=max_depth {
            if self.should_stop() {
                break;
            }

            // Aspiration windows for depth >= 2.
            let score = if depth >= 2 && prev_score.abs() < MATE_SCORE - MAX_DEPTH as i16 {
                let lo = prev_score.saturating_sub(ASPIRATION_WINDOW);
                let hi = prev_score.saturating_add(ASPIRATION_WINDOW);
                let s = self.alphabeta(depth, lo, hi, 0);
                if self.should_stop() {
                    break;
                }
                if s <= lo {
                    // Fail low: re-search with fully open lower bound.
                    self.alphabeta(depth, NEG_INF, hi, 0)
                } else if s >= hi {
                    // Fail high: re-search with fully open upper bound.
                    self.alphabeta(depth, lo, POS_INF, 0)
                } else {
                    s
                }
            } else {
                self.alphabeta(depth, NEG_INF, POS_INF, 0)
            };

            if self.should_stop() {
                break;
            }

            // Depth fully completed — commit results.
            self.best_score = score;
            self.best_depth = depth;
            self.best_pv = self.extract_pv();
            prev_score = score;

            // Emit info line.
            let elapsed = self.elapsed_ms();
            let nps = if elapsed > 0 {
                self.nodes * 1_000 / elapsed
            } else {
                0
            };
            let pv_str: Vec<String> =
                self.best_pv.iter().map(|m| m.to_algebraic()).collect();
            // Per-player evaluation at the root position for UI display.
            let v = [
                self.evaluator.eval_scalar(&self.gs, Player::Red) as i32,
                self.evaluator.eval_scalar(&self.gs, Player::Blue) as i32,
                self.evaluator.eval_scalar(&self.gs, Player::Yellow) as i32,
                self.evaluator.eval_scalar(&self.gs, Player::Green) as i32,
            ];
            let info_line = format!(
                "info depth {} score cp {} v1 {} v2 {} v3 {} v4 {} nodes {} nps {} time {} pv {} phase brs",
                depth,
                score,
                v[0], v[1], v[2], v[3],
                self.nodes,
                nps,
                elapsed,
                if pv_str.is_empty() {
                    "none".to_string()
                } else {
                    pv_str.join(" ")
                }
            );
            if let Some(cb) = info_cb.as_mut() {
                cb(info_line);
            }

            // Stop if time budget is exhausted.
            if let Some(limit_ms) = self.budget.max_time_ms {
                if self.elapsed_ms() >= limit_ms {
                    break;
                }
            }

            // Mate found — no point searching deeper.
            if score.abs() >= MATE_SCORE - MAX_DEPTH as i16 {
                break;
            }
        }

        let best_move = self
            .best_pv
            .first()
            .copied()
            .unwrap_or(root_moves[0]);

        SearchResult {
            best_move,
            score: self.best_score,
            depth: self.best_depth,
            nodes: self.nodes,
            pv: self.best_pv.clone(),
        }
    }

    // -----------------------------------------------------------------------
    // Alpha-beta dispatcher
    // -----------------------------------------------------------------------

    /// BRS alpha-beta search. Dispatches to max_node or min_node based on
    /// whose turn it is on the board.
    ///
    /// All scores are from `root_player`'s perspective (positive = good for root).
    /// `alpha` = minimum score root player is guaranteed elsewhere.
    /// `beta`  = maximum score the opponent allows (root exceeds this → cutoff).
    fn alphabeta(&mut self, depth: u8, alpha: i16, beta: i16, ply: usize) -> i16 {
        // Clear PV for this ply.
        self.pv_len[ply] = 0;

        // Periodic budget check.
        if self.nodes % TIME_CHECK_INTERVAL == 0 {
            self.check_limits();
        }
        if self.should_stop() {
            return 0;
        }

        // Leaf node: quiescence search.
        if depth == 0 {
            return self.quiescence(alpha, beta, MAX_QSEARCH_DEPTH);
        }

        let current = self.gs.board().side_to_move();
        let moves = generate_legal(self.gs.board_mut());

        // No legal moves: checkmate or stalemate.
        if moves.is_empty() {
            return if is_in_check(current, self.gs.board()) {
                // Checkmate. Penalise by ply to prefer shorter mates.
                if current == self.root_player {
                    -(MATE_SCORE - ply as i16) // root is mated
                } else {
                    MATE_SCORE - ply as i16 // opponent is mated
                }
            } else {
                // Stalemate. Approximate as neutral in search
                // (GameState awards 20 pts in FFA, but search uses 0 for simplicity).
                0
            };
        }

        if current == self.root_player {
            self.max_node(depth, alpha, beta, ply, moves)
        } else {
            self.min_node(depth, alpha, beta, ply, current, moves)
        }
    }

    // -----------------------------------------------------------------------
    // MAX node — root player explores all moves
    // -----------------------------------------------------------------------

    fn max_node(
        &mut self,
        depth: u8,
        mut alpha: i16,
        beta: i16,
        ply: usize,
        moves: Vec<Move>,
    ) -> i16 {
        // Null move pruning: skip root player's turn and check if the position
        // is still >= beta even with opponents getting priority.
        if depth >= NULL_MOVE_MIN_DEPTH
            && !is_in_check(self.root_player, self.gs.board())
            && has_non_pawn_material(self.gs.board(), self.root_player)
        {
            let saved = self.gs.board().side_to_move();
            // Skip root player: advance side_to_move to first opponent.
            self.gs.board_mut().set_side_to_move(self.root_player.next());
            let null_score =
                self.alphabeta(depth - 1 - NULL_MOVE_REDUCTION, alpha, beta, ply + 1);
            self.gs.board_mut().set_side_to_move(saved);

            if !self.should_stop() && null_score >= beta {
                return beta; // Prune: position is so good that passing still wins.
            }
        }

        // PV move from the previous depth, for move ordering.
        let pv_move = if ply < self.best_pv.len() {
            Some(self.best_pv[ply])
        } else {
            None
        };

        let ordered = order_moves(&moves, pv_move);
        let mut best = NEG_INF;

        for (move_idx, &mv) in ordered.iter().enumerate() {
            let undo = make_move(self.gs.board_mut(), mv);
            self.nodes += 1;

            // Late move reductions: search quiet late moves at reduced depth.
            let score = if move_idx >= LMR_MOVE_THRESHOLD
                && depth >= LMR_MIN_DEPTH
                && !mv.is_capture()
                && !mv.is_promotion()
            {
                // Narrow-window reduced-depth search.
                let s = self.alphabeta(depth - 2, alpha, alpha + 1, ply + 1);
                if !self.should_stop() && s > alpha {
                    // Promising move: re-search at full depth.
                    self.alphabeta(depth - 1, alpha, beta, ply + 1)
                } else {
                    s
                }
            } else {
                self.alphabeta(depth - 1, alpha, beta, ply + 1)
            };

            unmake_move(self.gs.board_mut(), mv, undo);

            if self.should_stop() {
                return best.max(NEG_INF);
            }

            if score > best {
                best = score;
                if best > alpha {
                    alpha = best;
                    self.update_pv(ply, mv);
                }
            }

            if alpha >= beta {
                // Beta cutoff: root player has found a move that is too good;
                // the opponent (parent MIN node or search bound) will not allow
                // this position.
                #[cfg(feature = "huginn")]
                {
                    // Gate: alpha_beta_prune (verbose level)
                    // Payload: depth, alpha, beta, score, move, node_type="max"
                    // Buffer plumbing deferred — Issue-Huginn-Gates-Unwired.
                    let _ = (depth, alpha, beta, score, mv);
                }
                break;
            }
        }

        best
    }

    // -----------------------------------------------------------------------
    // MIN node — opponent picks single best reply
    // -----------------------------------------------------------------------

    fn min_node(
        &mut self,
        depth: u8,
        alpha: i16,
        beta: i16,
        ply: usize,
        _opponent: Player,
        moves: Vec<Move>,
    ) -> i16 {
        // Select the single strongest reply for this opponent.
        // "Strongest" = the move that minimises root_player's static evaluation.
        let best_reply = select_best_opponent_reply(
            &mut self.gs,
            self.evaluator,
            self.root_player,
            &moves,
        );

        let Some(mv) = best_reply else {
            // Guard: should be caught by empty-move check in alphabeta, but
            // recurse safely if reached.
            return self.alphabeta(depth - 1, alpha, beta, ply + 1);
        };

        #[cfg(feature = "huginn")]
        {
            // Gate: brs_reply_selection (verbose level)
            // Payload: opponent, candidates=moves.len(), selected=mv
            // Buffer plumbing deferred — Issue-Huginn-Gates-Unwired.
            let _ = (_opponent, moves.len(), mv);
        }

        let undo = make_move(self.gs.board_mut(), mv);
        self.nodes += 1;
        let score = self.alphabeta(depth - 1, alpha, beta, ply + 1);
        unmake_move(self.gs.board_mut(), mv, undo);

        score
    }

    // -----------------------------------------------------------------------
    // Quiescence search
    // -----------------------------------------------------------------------

    /// Quiescence search: extend with captures to resolve tactical instability.
    ///
    /// At MAX (root player) nodes: stand-pat pruning + root player's captures.
    /// At MIN (opponent) nodes: opponent picks best capture if available, then
    /// returns min(stand_pat, score_after_capture).
    fn quiescence(&mut self, mut alpha: i16, beta: i16, qs_depth: u8) -> i16 {
        self.nodes += 1;

        if self.nodes % TIME_CHECK_INTERVAL == 0 {
            self.check_limits();
        }
        if self.should_stop() {
            return alpha;
        }

        let stand_pat = self.evaluator.eval_scalar(&self.gs, self.root_player);
        let current = self.gs.board().side_to_move();

        if current == self.root_player {
            // --- MAX quiescence node ---

            // Stand-pat: if static eval is already >= beta, no need to look further.
            if stand_pat >= beta {
                #[cfg(feature = "huginn")]
                {
                    // Gate: quiescence (verbose level) — stand-pat cutoff
                    let _ = (stand_pat, alpha, beta, qs_depth);
                }
                return beta;
            }
            if stand_pat > alpha {
                alpha = stand_pat;
            }

            // Depth cap: return stand-pat if max quiescence depth reached.
            if qs_depth == 0 {
                return alpha;
            }

            let all_moves = generate_legal(self.gs.board_mut());
            let captures: Vec<Move> = all_moves.into_iter().filter(|m| m.is_capture()).collect();

            for mv in captures {
                let undo = make_move(self.gs.board_mut(), mv);
                let score = self.quiescence(alpha, beta, qs_depth - 1);
                unmake_move(self.gs.board_mut(), mv, undo);

                if self.should_stop() {
                    return alpha;
                }
                if score >= beta {
                    return beta;
                }
                if score > alpha {
                    alpha = score;
                }
            }
        } else {
            // --- MIN quiescence node ---

            // If no quiescence depth remaining or no captures, return static eval.
            if qs_depth == 0 {
                return stand_pat;
            }

            let all_moves = generate_legal(self.gs.board_mut());
            let captures: Vec<Move> =
                all_moves.into_iter().filter(|m| m.is_capture()).collect();

            if captures.is_empty() {
                return stand_pat;
            }

            // Opponent picks the capture that most harms root player.
            let best_cap = select_best_opponent_reply(
                &mut self.gs,
                self.evaluator,
                self.root_player,
                &captures,
            );

            if let Some(mv) = best_cap {
                let undo = make_move(self.gs.board_mut(), mv);
                let score = self.quiescence(alpha, beta, qs_depth - 1);
                unmake_move(self.gs.board_mut(), mv, undo);

                if self.should_stop() {
                    return stand_pat;
                }
                // Opponent takes the worse outcome for root (min of stand_pat vs score).
                return score.min(stand_pat);
            }
        }

        #[cfg(feature = "huginn")]
        {
            // Gate: quiescence (verbose level) — exit
            let _ = (stand_pat, alpha, qs_depth);
        }

        alpha
    }

    // -----------------------------------------------------------------------
    // PV management
    // -----------------------------------------------------------------------

    /// Record `mv` as the best move at `ply` and copy the child PV below it.
    fn update_pv(&mut self, ply: usize, mv: Move) {
        self.pv_table[ply][0] = Some(mv);
        let child_len = if ply + 1 < MAX_DEPTH {
            self.pv_len[ply + 1]
        } else {
            0
        };
        for i in 0..child_len {
            self.pv_table[ply][i + 1] = self.pv_table[ply + 1][i];
        }
        self.pv_len[ply] = 1 + child_len;
    }

    /// Extract the principal variation from `pv_table[0]`.
    fn extract_pv(&self) -> Vec<Move> {
        (0..self.pv_len[0])
            .filter_map(|i| self.pv_table[0][i])
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

/// Select the move from `moves` that minimises `root_player`'s static eval.
///
/// This is the "best reply" for the opponent in plain BRS: the move that is
/// locally most harmful to the root player.
fn select_best_opponent_reply(
    gs: &mut GameState,
    evaluator: &dyn Evaluator,
    root_player: Player,
    moves: &[Move],
) -> Option<Move> {
    let mut best_move = None;
    let mut best_score = i16::MAX; // opponent minimises root's score

    for &mv in moves {
        let undo = make_move(gs.board_mut(), mv);
        let score = evaluator.eval_scalar(gs, root_player);
        unmake_move(gs.board_mut(), mv, undo);

        if score < best_score {
            best_score = score;
            best_move = Some(mv);
        }
    }

    best_move
}

/// Order moves for MAX node search: PV move first, then captures, then quiet.
fn order_moves(moves: &[Move], pv_move: Option<Move>) -> Vec<Move> {
    let mut ordered = Vec::with_capacity(moves.len());

    // PV move gets highest priority.
    if let Some(pv) = pv_move {
        if let Some(&m) = moves.iter().find(|&&m| m == pv) {
            ordered.push(m);
        }
    }

    // Captures before quiet moves.
    for &mv in moves {
        if Some(mv) == pv_move {
            continue;
        }
        if mv.is_capture() {
            ordered.push(mv);
        }
    }

    // Quiet moves last.
    for &mv in moves {
        if Some(mv) == pv_move {
            continue;
        }
        if !mv.is_capture() {
            ordered.push(mv);
        }
    }

    ordered
}

/// Return true if `player` has any piece other than pawns and king.
///
/// Used as a zugzwang guard for null move pruning: null moves are less reliable
/// in pure pawn / king endgames where passing loses tempo.
fn has_non_pawn_material(board: &crate::board::Board, player: Player) -> bool {
    board
        .piece_list(player)
        .iter()
        .any(|(pt, _)| !matches!(pt, PieceType::Pawn | PieceType::King))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::BootstrapEvaluator;
    use crate::gamestate::GameState;

    fn make_searcher() -> BrsSearcher {
        BrsSearcher::new(Box::new(BootstrapEvaluator::new()))
    }

    #[test]
    fn test_brs_searcher_returns_legal_move_from_start() {
        let gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let budget = SearchBudget {
            max_depth: Some(1),
            max_nodes: None,
            max_time_ms: None,
        };
        let result = searcher.search(&gs, budget);
        // Verify the returned move is legal.
        let mut check_gs = gs.clone();
        let legal = check_gs.legal_moves();
        assert!(
            legal.contains(&result.best_move),
            "returned move {:?} is not legal",
            result.best_move
        );
    }

    #[test]
    fn test_brs_search_result_score_in_valid_range() {
        let gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let budget = SearchBudget {
            max_depth: Some(3),
            max_nodes: None,
            max_time_ms: None,
        };
        let result = searcher.search(&gs, budget);
        assert!(
            result.score >= -30_000 && result.score <= 30_000,
            "score {} out of range",
            result.score
        );
    }

    #[test]
    fn test_brs_depth_limit_respected() {
        let gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let budget = SearchBudget {
            max_depth: Some(4),
            max_nodes: None,
            max_time_ms: None,
        };
        let result = searcher.search(&gs, budget);
        assert!(result.depth <= 4, "depth {} exceeded limit 4", result.depth);
        assert!(result.nodes > 0, "should have searched at least one node");
    }

    #[test]
    fn test_brs_pv_starts_with_best_move() {
        let gs = GameState::new_standard_ffa();
        let mut searcher = make_searcher();
        let budget = SearchBudget {
            max_depth: Some(3),
            max_nodes: None,
            max_time_ms: None,
        };
        let result = searcher.search(&gs, budget);
        assert!(
            !result.pv.is_empty(),
            "PV should not be empty after depth 3 search"
        );
        assert_eq!(
            result.pv[0], result.best_move,
            "PV first move should match best_move"
        );
    }

    #[test]
    fn test_brs_original_position_not_modified() {
        let gs = GameState::new_standard_ffa();
        let original_fen = gs.board().to_fen4();
        let mut searcher = make_searcher();
        let budget = SearchBudget {
            max_depth: Some(3),
            max_nodes: None,
            max_time_ms: None,
        };
        let _ = searcher.search(&gs, budget);
        assert_eq!(
            gs.board().to_fen4(),
            original_fen,
            "search must not modify the input position"
        );
    }
}
