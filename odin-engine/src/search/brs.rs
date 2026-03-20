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
// Player::prev() on side_to_move without storing it in MoveUndo. Manual
// set_side_to_move() between make and unmake corrupts restoration.

use std::sync::Arc;
use std::time::Instant;

use crate::board::{Board, PieceType, Player};
use crate::eval::{Evaluator, PIECE_EVAL_VALUES};
use crate::eval::nnue::accumulator::AccumulatorStack;
use crate::eval::nnue::{forward_pass, weights::NnueWeights};
use crate::gamestate::{GameState, PlayerStatus};
use arrayvec::ArrayVec;
use crate::movegen::{
    generate_legal, generate_legal_captures_into, generate_legal_into, is_in_check,
    is_square_attacked_by, make_move, unmake_move, Move, MAX_MOVES,
};

use super::board_scanner::{scan_board, select_hybrid_reply, BoardContext};
use super::mcts::HistoryTable;
use super::tt::{TranspositionTable, TT_DEFAULT_ENTRIES, TT_EXACT, TT_LOWER, TT_UPPER};
use super::{SearchBudget, SearchResult, Searcher};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Hard cap on search depth.
const MAX_DEPTH: usize = 64;

/// Total squares on the 14×14 board (196). Used to size history/counter-move tables.
const TOTAL_SQUARES: usize = 196;

/// Number of piece types (Pawn, Knight, Bishop, Rook, Queen, King, PromotedQueen).
/// Matches the range of `PieceType::index()`.
const PIECE_TYPE_COUNT: usize = 7;

/// Number of players. Used to size the history table.
const PLAYER_COUNT: usize = 4;

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

/// Maximum displayable score from BRS search. Raw mate-range scores are clamped
/// to this value in info lines and SearchResult to prevent false mate display.
/// BRS single-reply model produces phantom mates that wouldn't survive full
/// multi-reply search. Internal alpha-beta uses unclamped scores for correctness.
const BRS_SCORE_CAP: i16 = 9_999;

/// Check time/node budget every this many nodes.
const TIME_CHECK_INTERVAL: u64 = 1_024;

// ---------------------------------------------------------------------------
// BrsSearcher
// ---------------------------------------------------------------------------

/// Implements the `Searcher` trait using Best-Reply Search with alpha-beta.
pub struct BrsSearcher {
    evaluator: Box<dyn Evaluator>,
    info_cb: Option<Box<dyn FnMut(String)>>,
    /// Transposition table — persists across iterative deepening depths and
    /// between moves. Not reset between `search()` calls.
    tt: TranspositionTable,
    /// History table extracted from the last completed search (Stage 11).
    /// Contains accumulated move ordering scores for progressive history handoff.
    last_history: Option<Box<HistoryTable>>,
    /// Root move scores from the last completed depth of the last search (Stage 11).
    /// Each entry is (move, clamped_score) for all root moves that were searched.
    last_root_move_scores: Option<Vec<(Move, i16)>>,
    // --- NNUE (Stage 16) ---
    /// Accumulator stack for incremental NNUE updates through the search tree.
    /// None when using BootstrapEvaluator (no NNUE weights loaded).
    acc_stack: Option<AccumulatorStack>,
    /// Shared NNUE weights (read-only during search). None = bootstrap eval.
    nnue_weights: Option<Arc<NnueWeights>>,
    /// Defense-aware move ordering weight. 0.0 = disabled. Default: 0.5.
    defense_weight: f32,
}

impl BrsSearcher {
    /// Create a new BrsSearcher with the given evaluator and optional NNUE weights.
    pub fn new(evaluator: Box<dyn Evaluator>, nnue_weights: Option<Arc<NnueWeights>>) -> Self {
        let acc_stack = if nnue_weights.is_some() {
            Some(AccumulatorStack::new())
        } else {
            None
        };
        Self {
            evaluator,
            info_cb: None,
            tt: TranspositionTable::new(TT_DEFAULT_ENTRIES),
            last_history: None,
            last_root_move_scores: None,
            acc_stack,
            nnue_weights,
            defense_weight: 0.75,
        }
    }

    /// Create a new BrsSearcher with a real-time info callback.
    ///
    /// The callback receives a formatted `info` string after each completed
    /// iterative deepening depth. The protocol can use this to emit progress.
    pub fn with_info_callback(
        evaluator: Box<dyn Evaluator>,
        cb: Box<dyn FnMut(String)>,
        nnue_weights: Option<Arc<NnueWeights>>,
    ) -> Self {
        let mut searcher = Self::new(evaluator, nnue_weights);
        searcher.info_cb = Some(cb);
        searcher
    }

    /// Replace the info callback. Called before each `search()` when the
    /// searcher is persisted across `go` commands, so each search gets its
    /// own output buffer.
    pub fn set_info_callback(&mut self, cb: Box<dyn FnMut(String)>) {
        self.info_cb = Some(cb);
    }

    /// Take the info callback out of the searcher (returns ownership).
    /// Used by HybridController to move the callback between sub-searchers.
    pub fn take_info_callback(&mut self) -> Option<Box<dyn FnMut(String)>> {
        self.info_cb.take()
    }

    /// History table from the last completed search. Returns None if no search
    /// has been performed yet.
    pub fn history_table(&self) -> Option<&HistoryTable> {
        self.last_history.as_deref()
    }

    /// Root move scores from the last completed depth of the last search.
    /// Each entry is (move, score_clamped_to_BRS_SCORE_CAP).
    pub fn root_move_scores(&self) -> Option<&[(Move, i16)]> {
        self.last_root_move_scores.as_deref()
    }

    /// Set the defense-aware move ordering weight.
    /// 0.0 = disabled (original behavior). Default: 0.5. Max: 2.0.
    pub fn set_defense_weight(&mut self, w: f32) {
        self.defense_weight = w.clamp(0.0, 2.0);
    }
}

impl Searcher for BrsSearcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult {
        self.tt.increment_generation();

        // Stage 16: Initialize accumulator from root position before search.
        if let (Some(stack), Some(w)) = (&mut self.acc_stack, &self.nnue_weights) {
            stack.init_from_board(position.board(), w);
        }

        // Split borrows: acc_stack and nnue_weights are passed as Option refs.
        let nnue_w = self.nnue_weights.as_deref();
        let acc = self.acc_stack.as_mut();
        let mut ctx = BrsContext::new(
            position,
            self.evaluator.as_ref(),
            &budget,
            &mut self.tt,
            acc,
            nnue_w,
            self.defense_weight,
        );
        let result = ctx.iterative_deepening(&mut self.info_cb);
        // Extract BRS knowledge for Stage 11 hybrid handoff.
        self.last_history = Some(Box::new(ctx.history));
        self.last_root_move_scores = if ctx.root_move_scores.is_empty() {
            None
        } else {
            Some(ctx.root_move_scores)
        };
        result
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
    /// Pre-search board context (Stage 8 hybrid scoring).
    board_ctx: BoardContext,
    /// Snapshot of position_history at search start. Used for repetition detection.
    /// Arc-shared with GameState — O(1) clone, no heap copy.
    game_history: Arc<Vec<u64>>,
    /// Path-local stack of Zobrist hashes pushed as we descend the tree.
    /// Combined with game_history to count repetitions without modifying GameState.
    rep_stack: Vec<u64>,
    /// Transposition table reference. Shared with BrsSearcher; persists across
    /// iterative deepening depths.
    tt: &'a mut TranspositionTable,
    /// Killer moves: up to 2 quiet moves per ply that caused a beta cutoff.
    /// Reset per search call. Used to try "refutation" moves early at the same ply.
    killers: [[Option<Move>; 2]; MAX_DEPTH],
    /// History heuristic: accumulated score for (player, piece_type, to_sq) moves
    /// that caused beta cutoffs. Higher = try this move earlier in quiet ordering.
    /// Indexed as `history[player_idx][piece_type_idx][to_sq]`. Reset per search.
    history: [[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT],
    /// Counter-move table: indexed by [from_sq * TOTAL_SQUARES + to_sq] of the
    /// most recent opponent move. Stores the quiet move that most recently caused
    /// a beta cutoff in response to that opponent move. Reset per search.
    countermoves: Vec<Option<Move>>,
    /// Most recent opponent move at each ply, set by min_node before recursing.
    /// Used by max_node to look up the counter-move for the current position.
    last_opp_move: [Option<Move>; MAX_DEPTH],
    /// Root move scores from the last fully completed iterative deepening depth.
    /// Used by HybridController (Stage 11) for survivor filtering.
    root_move_scores: Vec<(Move, i16)>,
    /// Temporary buffer for root move scores during the current depth iteration.
    /// Committed to `root_move_scores` when a depth completes without aborting.
    current_depth_root_scores: Vec<(Move, i16)>,
    // --- NNUE (Stage 16) ---
    /// Accumulator stack for incremental NNUE updates. None = bootstrap eval.
    acc_stack: Option<&'a mut AccumulatorStack>,
    /// NNUE weights reference. None = bootstrap eval.
    nnue_weights: Option<&'a NnueWeights>,
    // --- Defense-aware move ordering ---
    /// Defense bonus weight for move ordering. 0.0 = disabled.
    defense_weight: f32,
}

impl<'a> BrsContext<'a> {
    fn new(
        position: &GameState,
        evaluator: &'a dyn Evaluator,
        budget: &'a SearchBudget,
        tt: &'a mut TranspositionTable,
        acc_stack: Option<&'a mut AccumulatorStack>,
        nnue_weights: Option<&'a NnueWeights>,
        defense_weight: f32,
    ) -> Self {
        let root_player = position.current_player();
        let board_ctx = scan_board(position, root_player);

        Self {
            gs: position.clone(),
            evaluator,
            root_player,
            budget,
            nodes: 0,
            start: Instant::now(),
            stopped: false,
            pv_table: [[None; MAX_DEPTH]; MAX_DEPTH],
            pv_len: [0; MAX_DEPTH],
            best_pv: Vec::new(),
            best_score: 0,
            best_depth: 0,
            board_ctx,
            game_history: Arc::clone(position.position_history_arc()),
            rep_stack: Vec::with_capacity(64),
            tt,
            killers: [[None; 2]; MAX_DEPTH],
            history: [[[0; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT],
            countermoves: vec![None; TOTAL_SQUARES * TOTAL_SQUARES],
            last_opp_move: [None; MAX_DEPTH],
            root_move_scores: Vec::new(),
            current_depth_root_scores: Vec::new(),
            acc_stack,
            nnue_weights,
            defense_weight,
        }
    }

    /// NNUE-aware scalar eval. Uses the accumulator stack + forward pass when
    /// available, falls back to bootstrap evaluator otherwise.
    fn nnue_eval_scalar(&mut self, player: Player) -> i16 {
        if let (Some(ref mut stack), Some(weights)) = (&mut self.acc_stack, self.nnue_weights) {
            stack.refresh_if_needed(self.gs.board(), weights);
            let (brs_score, _) = forward_pass(stack.current(), weights, player);
            brs_score
        } else {
            self.evaluator.eval_scalar(&self.gs, player)
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
        self.best_score = self.nnue_eval_scalar(self.root_player);

        // Debug: compare NNUE vs bootstrap at root, flag large disagreements.
        #[cfg(debug_assertions)]
        if self.nnue_weights.is_some() {
            let bootstrap = self.evaluator.eval_scalar(&self.gs, self.root_player);
            let nnue = self.best_score;
            let diff = (nnue as i32 - bootstrap as i32).abs();
            if diff > 200 {
                eprintln!(
                    "info string [debug] NNUE vs bootstrap disagreement at root: nnue={} bootstrap={} diff={}",
                    nnue, bootstrap, diff
                );
            }
        }

        let mut prev_score = self.best_score;

        for depth in 1..=max_depth {
            if self.should_stop() {
                break;
            }

            // Clear temporary root score buffer for this depth iteration.
            self.current_depth_root_scores.clear();

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
            // Guard: aspiration re-search can leave PV table empty at ply 0
            // when all root moves fail low against the widened window. Keep
            // the previous depth's PV rather than emitting "pv none".
            let pv = self.extract_pv();
            if !pv.is_empty() {
                self.best_pv = pv;
            }
            prev_score = score;

            // Commit root move scores from this completed depth (Stage 11 handoff).
            if !self.current_depth_root_scores.is_empty() {
                self.root_move_scores = std::mem::take(&mut self.current_depth_root_scores);
            }

            // Emit info line.
            let elapsed = self.elapsed_ms();
            let nps = if elapsed > 0 {
                self.nodes * 1_000 / elapsed
            } else {
                0
            };
            let pv_str: Vec<String> = self.best_pv.iter().map(|m| m.to_algebraic()).collect();
            // Per-player evaluation at the root position for UI display.
            let v = [
                self.nnue_eval_scalar(Player::Red) as i32,
                self.nnue_eval_scalar(Player::Blue) as i32,
                self.nnue_eval_scalar(Player::Yellow) as i32,
                self.nnue_eval_scalar(Player::Green) as i32,
            ];
            // FFA game scores (capture points, checkmate bonuses, etc.)
            let s = self.gs.scores();
            // Clamp displayed score to prevent BRS phantom mates (19995cp etc.)
            // from showing as mate in the UI. Internal search keeps raw scores.
            let display_score = score.clamp(-BRS_SCORE_CAP, BRS_SCORE_CAP);
            let info_line = format!(
                "info depth {} score cp {} v1 {} v2 {} v3 {} v4 {} s1 {} s2 {} s3 {} s4 {} nodes {} nps {} time {} pv {} phase brs",
                depth,
                display_score,
                v[0], v[1], v[2], v[3],
                s[0], s[1], s[2], s[3],
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
            // In 4PC, depths below 8 (2 full rotations) can produce false mates
            // because BRS single-reply model leaves the last player without a
            // second response. Only trust mate-break at depth >= 8.
            if score.abs() >= MATE_SCORE - MAX_DEPTH as i16 && depth >= 8 {
                break;
            }
        }

        let best_move = self.best_pv.first().copied().unwrap_or(root_moves[0]);

        SearchResult {
            best_move,
            score: self.best_score.clamp(-BRS_SCORE_CAP, BRS_SCORE_CAP),
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
        if self.nodes.is_multiple_of(TIME_CHECK_INTERVAL) {
            self.check_limits();

            // Debug: every 1024 nodes, verify incremental accumulator matches full recompute.
            #[cfg(debug_assertions)]
            if let (Some(ref mut stack), Some(weights)) = (&mut self.acc_stack, self.nnue_weights) {
                use crate::eval::nnue::accumulator::Accumulator;
                // Verify stack depth is reasonable (should never exceed ~60 for depth 8).
                debug_assert!(
                    stack.depth() <= 64,
                    "[debug] acc_stack depth {} exceeds 64 at node {}",
                    stack.depth(), self.nodes
                );
                stack.refresh_if_needed(self.gs.board(), weights);
                let inc = stack.current();
                let mut full = Accumulator::zeroed();
                full.compute_full(self.gs.board(), weights);
                for p in 0..4 {
                    assert_eq!(
                        inc.values[p], full.values[p],
                        "[debug] accumulator divergence at perspective {} node {}",
                        p, self.nodes
                    );
                }
            }
        }
        if self.should_stop() {
            return 0;
        }

        // Compute Zobrist hash once — used for repetition detection (base hash)
        // and TT (player-aware hash).
        let hash = self.gs.board().zobrist();
        // TT hash includes the root player so entries from different root-player
        // searches never collide. Repetition detection uses the raw board hash.
        let tt_hash = hash
            ^ self
                .gs
                .board()
                .zobrist_keys()
                .root_player_key(self.root_player.index());

        // In-search repetition detection.
        // rep_stack contains hashes pushed by ancestor nodes on the current path.
        // The parent pushes the current hash *before* calling alphabeta, so
        // game_count + search_count >= 3 correctly identifies the 3rd occurrence.
        if ply > 0 {
            let game_count = self.game_history.iter().filter(|&&h| h == hash).count();
            let search_count = self.rep_stack.iter().filter(|&&h| h == hash).count();
            if game_count + search_count >= 3 {
                return 0; // Draw by repetition.
            }
        }

        // TT probe (depth > 0 only; quiescence search does not use TT).
        // Must come AFTER repetition check so draw scores are never bypassed.
        let orig_alpha = alpha;
        let mut alpha = alpha;
        let mut beta = beta;
        let mut compressed_tt_move: Option<u16> = None;
        if depth > 0 {
            if ply == 0 {
                // At the root, only use TT for move ordering hint — never for
                // score cutoffs or alpha/beta tightening. Aspiration re-searches
                // at the same depth would otherwise pick up TT_LOWER/TT_UPPER
                // entries from the initial narrow-window search, tightening alpha
                // to a value no move can beat and leaving the PV empty.
                let mut dummy_a = NEG_INF;
                let mut dummy_b = POS_INF;
                let probe = self.tt.probe(tt_hash, depth, &mut dummy_a, &mut dummy_b, 0);
                compressed_tt_move = probe.best_move;
            } else {
                let probe = self.tt.probe(tt_hash, depth, &mut alpha, &mut beta, ply as u8);
                if let Some(score) = probe.score {
                    return score;
                }
                compressed_tt_move = probe.best_move;
            }
        }

        // Leaf node: quiescence search.
        if depth == 0 {
            return self.quiescence(alpha, beta, MAX_QSEARCH_DEPTH);
        }

        let current = self.gs.board().side_to_move();

        // Skip eliminated (and DKW) players: make_move cycles turns via .next()
        // without checking PlayerStatus. An eliminated player's king has been removed
        // from the board — generating moves for them corrupts board state.
        // We skip by advancing side_to_move one step, recursing at the same depth,
        // then restoring. This is safe: no set_side_to_move is inserted between a
        // make_move and its matching unmake_move (ADR-012 constraint).
        if self.gs.player_status(current) != PlayerStatus::Active {
            let next = current.next();
            self.gs.board_mut().set_side_to_move(next);
            let score = self.alphabeta(depth, alpha, beta, ply);
            self.gs.board_mut().set_side_to_move(current);
            return score;
        }

        let mut moves = ArrayVec::<Move, MAX_MOVES>::new();
        generate_legal_into(self.gs.board_mut(), &mut moves);

        // No legal moves: checkmate or stalemate.
        if moves.is_empty() {
            let score = if is_in_check(current, self.gs.board()) {
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
            // Terminal nodes are always exact scores.
            self.tt.store(tt_hash, None, score, depth, TT_EXACT, ply as u8);
            return score;
        }

        // Decompress the TT best-move hint against this position's legal moves.
        // Used by max_node to try the TT move first in move ordering.
        let tt_move =
            compressed_tt_move.and_then(|c| TranspositionTable::decompress_move(c, &moves));

        let score = if current == self.root_player {
            self.max_node(depth, alpha, beta, ply, &moves, tt_move)
        } else {
            self.min_node(depth, alpha, beta, ply, current, &moves)
        };

        // TT store. Skip if search was aborted (score may be partial).
        if !self.should_stop() {
            let flag = if score <= orig_alpha {
                TT_UPPER // all moves failed to improve alpha (upper bound)
            } else if score >= beta {
                TT_LOWER // search failed high — beta cutoff (lower bound)
            } else {
                TT_EXACT // score is within the [orig_alpha, beta) window
            };
            // Best move: tracked by PV table at MAX nodes; None at MIN nodes.
            let best_move_compressed = if current == self.root_player {
                self.pv_table[ply][0].map(TranspositionTable::compress_move)
            } else {
                None
            };
            self.tt
                .store(tt_hash, best_move_compressed, score, depth, flag, ply as u8);
        }

        score
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
        moves: &[Move],
        tt_move: Option<Move>,
    ) -> i16 {
        // Null move pruning: skip root player's turn and check if the position
        // is still >= beta even with opponents getting priority.
        // Guard: ply > 0 prevents null move cutoff at root, which would prevent
        // root move score collection needed by Stage 11 hybrid survivor filter.
        if ply > 0
            && depth >= NULL_MOVE_MIN_DEPTH
            && !is_in_check(self.root_player, self.gs.board())
            && has_non_pawn_material(self.gs.board(), self.root_player)
        {
            let saved = self.gs.board().side_to_move();
            // Skip root player: advance side_to_move to first opponent.
            self.gs
                .board_mut()
                .set_side_to_move(self.root_player.next());
            let null_score = self.alphabeta(depth - 1 - NULL_MOVE_REDUCTION, alpha, beta, ply + 1);
            self.gs.board_mut().set_side_to_move(saved);

            if !self.should_stop() && null_score >= beta {
                return beta; // Prune: position is so good that passing still wins.
            }
        }

        // Priority move: TT hint takes precedence over PV; fall back to PV if none.
        let pv_move = if ply < self.best_pv.len() {
            Some(self.best_pv[ply])
        } else {
            None
        };
        let hint_move = tt_move.or(pv_move);

        // Counter-move: the quiet refutation of the opponent's last move at this ply.
        let countermove = self.last_opp_move[ply].and_then(|opp_mv| {
            let idx = opp_mv.from_sq() as usize * TOTAL_SQUARES + opp_mv.to_sq() as usize;
            self.countermoves[idx]
        });

        let ordered = order_moves(
            self.gs.board(),
            &moves,
            hint_move,
            &self.killers[ply],
            countermove,
            &self.history,
            self.root_player,
            self.defense_weight,
        );
        let mut best = NEG_INF;

        for (move_idx, &mv) in ordered.iter().enumerate() {
            if let (Some(ref mut s), Some(w)) = (&mut self.acc_stack, self.nnue_weights) {
                s.push(mv, self.gs.board(), w);
            }
            let undo = make_move(self.gs.board_mut(), mv);
            self.nodes += 1;
            self.rep_stack.push(self.gs.board().zobrist());

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

            self.rep_stack.pop();
            unmake_move(self.gs.board_mut(), mv, undo);
            if let Some(ref mut s) = self.acc_stack {
                s.pop();
            }

            if self.should_stop() {
                return best.max(NEG_INF);
            }

            // Record root move scores for Stage 11 hybrid survivor filter.
            if ply == 0 {
                let clamped = score.clamp(-BRS_SCORE_CAP, BRS_SCORE_CAP);
                self.current_depth_root_scores.push((mv, clamped));
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
                // this position. Update move ordering heuristics for quiet moves.
                if !mv.is_capture() && !mv.is_promotion() {
                    // Killer: shift existing killer down, insert new one at slot 0.
                    self.killers[ply][1] = self.killers[ply][0];
                    self.killers[ply][0] = Some(mv);
                    // History: reward depth^2 to prefer moves that cut off at deeper plies.
                    let p = self.root_player.index();
                    let pt = mv.piece_type().index();
                    let to = mv.to_sq() as usize;
                    self.history[p][pt][to] =
                        self.history[p][pt][to].saturating_add((depth as i32) * (depth as i32));
                    // Counter-move: record this response to the opponent's last move.
                    if let Some(opp_mv) = self.last_opp_move[ply] {
                        let idx =
                            opp_mv.from_sq() as usize * TOTAL_SQUARES + opp_mv.to_sq() as usize;
                        self.countermoves[idx] = Some(mv);
                    }
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
        opponent: Player,
        moves: &[Move],
    ) -> i16 {
        // Stage 8 hybrid reply selection: uses board context + move classifier
        // + progressive narrowing to pick the opponent reply that is both
        // harmful and realistic.
        let best_reply = select_hybrid_reply(
            &mut self.gs,
            self.evaluator,
            self.root_player,
            opponent,
            &moves,
            &self.board_ctx,
            depth,
        );

        let Some(mv) = best_reply else {
            // Guard: should be caught by empty-move check in alphabeta, but
            // recurse safely if reached.
            return self.alphabeta(depth - 1, alpha, beta, ply + 1);
        };

        if let (Some(ref mut s), Some(w)) = (&mut self.acc_stack, self.nnue_weights) {
            s.push(mv, self.gs.board(), w);
        }
        let undo = make_move(self.gs.board_mut(), mv);
        self.nodes += 1;
        self.rep_stack.push(self.gs.board().zobrist());
        // Record opponent's move so the child MAX node can look up its counter-move.
        if ply + 1 < MAX_DEPTH {
            self.last_opp_move[ply + 1] = Some(mv);
        }
        let score = self.alphabeta(depth - 1, alpha, beta, ply + 1);
        self.rep_stack.pop();
        unmake_move(self.gs.board_mut(), mv, undo);
        if let Some(ref mut s) = self.acc_stack {
            s.pop();
        }

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

        if self.nodes.is_multiple_of(TIME_CHECK_INTERVAL) {
            self.check_limits();
        }
        if self.should_stop() {
            return alpha;
        }

        let stand_pat = self.nnue_eval_scalar(self.root_player);
        let current = self.gs.board().side_to_move();

        // Skip eliminated players — same reasoning as in alphabeta.
        // generate_legal on a kingless board corrupts state; advance past them.
        if self.gs.player_status(current) != PlayerStatus::Active {
            let next = current.next();
            self.gs.board_mut().set_side_to_move(next);
            let score = self.quiescence(alpha, beta, qs_depth);
            self.gs.board_mut().set_side_to_move(current);
            return score;
        }

        if current == self.root_player {
            // --- MAX quiescence node ---

            // Stand-pat: if static eval is already >= beta, no need to look further.
            if stand_pat >= beta {
                return beta;
            }
            if stand_pat > alpha {
                alpha = stand_pat;
            }

            // Depth cap: return stand-pat if max quiescence depth reached.
            if qs_depth == 0 {
                return alpha;
            }

            let mut captures = ArrayVec::<Move, MAX_MOVES>::new();
            generate_legal_captures_into(self.gs.board_mut(), &mut captures);

            for mv in &captures {
                let mv = *mv;
                if let (Some(ref mut s), Some(w)) = (&mut self.acc_stack, self.nnue_weights) {
                    s.push(mv, self.gs.board(), w);
                }
                let undo = make_move(self.gs.board_mut(), mv);
                let score = self.quiescence(alpha, beta, qs_depth - 1);
                unmake_move(self.gs.board_mut(), mv, undo);
                if let Some(ref mut s) = self.acc_stack {
                    s.pop();
                }

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

            let mut captures = ArrayVec::<Move, MAX_MOVES>::new();
            generate_legal_captures_into(self.gs.board_mut(), &mut captures);

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
                if let (Some(ref mut s), Some(w)) = (&mut self.acc_stack, self.nnue_weights) {
                    s.push(mv, self.gs.board(), w);
                }
                let undo = make_move(self.gs.board_mut(), mv);
                let score = self.quiescence(alpha, beta, qs_depth - 1);
                unmake_move(self.gs.board_mut(), mv, undo);
                if let Some(ref mut s) = self.acc_stack {
                    s.pop();
                }

                if self.should_stop() {
                    return stand_pat;
                }
                // Opponent takes the worse outcome for root (min of stand_pat vs score).
                return score.min(stand_pat);
            }
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

/// Static Exchange Evaluation (SEE) — simplified for 4-player chess.
///
/// Returns true if the material exchange initiated by `mv` is expected to win
/// at least `threshold` centipawns for the moving side.
///
/// Simplified model: check only the immediate exchange (attacker value vs captured
/// value). A full recursive SEE with all 4-player recapture sequences is deferred
/// to Stage 19. This covers the most important case: detecting clearly winning
/// captures (pawn takes queen) and clearly losing ones (queen takes defended pawn).
///
/// Improvement over the Stage 9 baseline: checks whether the captured piece is
/// defended by any opponent before applying the attacker-value comparison. An
/// undefended piece is a free capture regardless of piece values — bishop×pawn
/// is a WINNING capture if the pawn is undefended, not a losing one.
///
/// Returns true if the capture gains at least `threshold` centipawns.
fn see(board: &Board, mv: Move, player: Player, threshold: i16) -> bool {
    if !mv.is_capture() {
        return threshold <= 0;
    }
    let captured_val = mv
        .captured()
        .map(|pt| PIECE_EVAL_VALUES[pt.index()])
        .unwrap_or(0);

    // Check whether any opponent of the capturer can recapture on the to-square.
    // If nobody can recapture, the capture is free (gains captured_val outright).
    let to_sq = mv.to_sq();
    let is_recapturable = Player::ALL
        .iter()
        .any(|&p| p != player && is_square_attacked_by(to_sq, p, board));

    if !is_recapturable {
        // Undefended piece: free capture.
        return captured_val >= threshold;
    }

    // Defended: simplified single-exchange SEE (attacker vs captured).
    let attacker_val = PIECE_EVAL_VALUES[mv.piece_type().index()];
    captured_val - attacker_val >= threshold
}

// ---------------------------------------------------------------------------
// Defense-aware move ordering
// ---------------------------------------------------------------------------

/// Identifies the player's pieces that are attacked but not defended (hanging).
/// Returns a list of (square, piece_value) for each hanging piece.
///
/// A piece is "hanging" if:
///   - At least one opponent attacks its square
///   - The owning player does NOT also defend that square
///
/// Kings are excluded (they have their own check-evasion logic).
/// Cost: one `is_square_attacked_by` per piece (~16 max), negligible vs movegen.
fn find_hanging_pieces(
    board: &Board,
    player: Player,
) -> ArrayVec<(u8, i16), 16> {
    let mut hanging = ArrayVec::new();
    for &(pt, sq) in board.piece_list(player) {
        let val = PIECE_EVAL_VALUES[pt.index()];
        if val == 0 {
            continue; // skip kings
        }

        // Is any opponent attacking this square?
        let attacked = Player::ALL
            .iter()
            .any(|&p| p != player && is_square_attacked_by(sq, p, board));
        if !attacked {
            continue;
        }

        // Is the piece defended by one of our own pieces?
        // We temporarily ignore the piece itself — `is_square_attacked_by` checks
        // if any *other* piece of ours covers this square. Since the piece is ON
        // the square, the function checks rays/jumps from other pieces to this sq.
        let defended = is_square_attacked_by(sq, player, board);
        if defended {
            continue; // has at least one defender — not hanging
        }

        hanging.push((sq, val));
    }
    hanging
}

/// Cheap geometric check: could `piece_type` on `from_sq` potentially
/// defend/attack `target_sq`?
///
/// For knights and kings this is exact. For sliders (bishop, rook, queen)
/// this is an approximation — it checks alignment but NOT obstructions.
/// False positives are harmless for move ordering (move gets tried earlier,
/// search discovers it doesn't actually help). Pawns are excluded (rarely
/// serve as critical defenders in 4PC).
fn could_defend(piece_type: PieceType, from_sq: u8, target_sq: u8) -> bool {
    use crate::board::{file_of, rank_of};
    let df = (file_of(from_sq) as i16 - file_of(target_sq) as i16).abs();
    let dr = (rank_of(from_sq) as i16 - rank_of(target_sq) as i16).abs();
    match piece_type {
        PieceType::Knight => (df == 1 && dr == 2) || (df == 2 && dr == 1),
        PieceType::King => df <= 1 && dr <= 1 && (df + dr) > 0,
        PieceType::Rook => df == 0 || dr == 0,
        PieceType::Bishop => df == dr && df > 0,
        PieceType::Queen | PieceType::PromotedQueen => df == 0 || dr == 0 || (df == dr && df > 0),
        _ => false, // pawns — skip
    }
}

/// Compute the defense bonus for a quiet move given the list of hanging pieces.
///
/// Two cases earn a bonus:
///   1. **Retreat**: The move evacuates a hanging piece to a safe square.
///   2. **Add defender**: The move places a piece where it could defend a
///      hanging square (geometric check, no obstruction verification).
///
/// The bonus is `defense_weight * hanging_piece_value` (as i32), which is
/// additive with the history heuristic score. This keeps defensive moves
/// within the quiet-move tier — they never override TT, captures, killers,
/// or counter-moves.
fn defense_bonus_for_move(
    mv: Move,
    hanging: &[(u8, i16)],
    board: &Board,
    player: Player,
    defense_weight: f32,
) -> i32 {
    if defense_weight <= 0.0 || hanging.is_empty() {
        return 0;
    }
    let mut bonus = 0i32;
    let from = mv.from_sq();
    let to = mv.to_sq();
    let pt = mv.piece_type();

    for &(hang_sq, piece_val) in hanging {
        if from == hang_sq {
            // Retreat: piece is hanging and moves away.
            // Only award bonus if the destination is not also attacked.
            let dest_attacked = Player::ALL
                .iter()
                .any(|&p| p != player && is_square_attacked_by(to, p, board));
            if !dest_attacked {
                bonus += (defense_weight * piece_val as f32) as i32;
            }
        } else if could_defend(pt, to, hang_sq) {
            // This move places a piece that could cover the hanging square.
            // Geometric approximation — slider obstructions not checked.
            bonus += (defense_weight * piece_val as f32) as i32;
        }
    }
    bonus
}

/// Order moves for MAX node search using the full Stage 9 pipeline,
/// extended with defense-aware bonus scoring for quiet moves.
///
/// Priority order (highest first):
///   1. TT/PV hint move — best move from previous search or TT probe
///   2. Winning captures (SEE >= 0), sorted descending by MVV-LVA score
///   3. Non-capture promotions
///   4. Killer moves (up to 2) — quiet moves that caused cutoffs at this ply
///   5. Counter-move — quiet move that refuted the opponent's last move
///   6. Remaining quiet moves, sorted descending by (history + defense bonus)
///   7. Losing captures (SEE < 0), sorted descending by MVV-LVA score
///
/// The defense bonus is additive with history — it does NOT promote quiet
/// moves above captures, killers, or counter-moves. It only reorders within
/// the quiet tier. Set defense_weight = 0.0 for original behavior.
///
/// All moves passed must be legal; the hint/killers/counter-move are validated
/// against the legal-move list before use.
#[allow(clippy::too_many_arguments)]
fn order_moves(
    board: &Board,
    moves: &[Move],
    hint_move: Option<Move>,
    killers: &[Option<Move>; 2],
    countermove: Option<Move>,
    history: &[[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT],
    player: Player,
    defense_weight: f32,
) -> ArrayVec<Move, MAX_MOVES> {
    let player_idx = player.index();
    let mut ordered = ArrayVec::<Move, MAX_MOVES>::new();
    // Track which moves have been placed to avoid duplicates (stack-allocated).
    let mut placed = [false; MAX_MOVES];

    // --- Defense scan: find hanging pieces (once per position) ---
    let hanging = if defense_weight > 0.0 {
        find_hanging_pieces(board, player)
    } else {
        ArrayVec::new()
    };

    // Helper: find the index of `mv` in `moves` and mark it placed.
    let find_and_mark = |mv: Move, placed: &mut [bool; MAX_MOVES]| -> Option<usize> {
        moves.iter().position(|&m| m == mv).inspect(|&i| {
            placed[i] = true;
        })
    };

    // --- 1. TT/PV hint move ---
    if let Some(hint) = hint_move {
        if let Some(i) = find_and_mark(hint, &mut placed) {
            ordered.push(moves[i]);
        }
    }

    // --- Classify remaining moves (captures vs quiet) ---
    let mut win_caps = ArrayVec::<(usize, i16), MAX_MOVES>::new();
    let mut lose_caps = ArrayVec::<(usize, i16), MAX_MOVES>::new();
    let mut promotions = ArrayVec::<usize, MAX_MOVES>::new();
    let mut quiets = ArrayVec::<(usize, i32), MAX_MOVES>::new();

    for (i, &mv) in moves.iter().enumerate() {
        if placed[i] {
            continue;
        }
        if mv.is_capture() {
            // Dead (DKW) pieces are worth minimal capture value — sort after alive captures.
            let is_dead = board
                .piece_at(mv.to_sq())
                .map(|p| p.status != crate::board::PieceStatus::Alive)
                .unwrap_or(false);
            let victim_val = if is_dead {
                1_i16
            } else {
                mv.captured()
                    .map(|pt| PIECE_EVAL_VALUES[pt.index()])
                    .unwrap_or(0)
            };
            let attacker_val = PIECE_EVAL_VALUES[mv.piece_type().index()];
            let mvv_lva = victim_val * 10 - attacker_val;
            if see(board, mv, player, 0) {
                win_caps.push((i, mvv_lva));
            } else {
                lose_caps.push((i, mvv_lva));
            }
        } else if mv.is_promotion() {
            promotions.push(i);
        } else {
            let pt = mv.piece_type().index();
            let to = mv.to_sq() as usize;
            let hist = history[player_idx][pt][to];
            // Add defense bonus: history + defense_bonus for quiet moves.
            let def_bonus = defense_bonus_for_move(mv, &hanging, board, player, defense_weight);
            quiets.push((i, hist + def_bonus));
        }
    }

    // --- 2. Winning captures (SEE >= 0), MVV-LVA descending ---
    win_caps.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    for (i, _) in &win_caps {
        placed[*i] = true;
        ordered.push(moves[*i]);
    }

    // --- 3. Non-capture promotions ---
    for i in &promotions {
        placed[*i] = true;
        ordered.push(moves[*i]);
    }

    // --- 4 & 5. Killers and counter-move (before sorted quiets) ---
    // Try each killer; skip if already placed (was a capture/TT move).
    for &killer_opt in killers {
        if let Some(killer) = killer_opt {
            if let Some(i) = moves.iter().position(|&m| m == killer) {
                if !placed[i] {
                    placed[i] = true;
                    // Remove from quiets list to avoid duplicate.
                    quiets.retain(|(qi, _)| *qi != i);
                    ordered.push(moves[i]);
                }
            }
        }
    }
    // Counter-move (skip if already placed).
    if let Some(cm) = countermove {
        if let Some(i) = moves.iter().position(|&m| m == cm) {
            if !placed[i] {
                // No need to set placed[i] — no more checks after this.
                quiets.retain(|(qi, _)| *qi != i);
                ordered.push(moves[i]);
            }
        }
    }

    // --- 6. Remaining quiet moves, history descending ---
    quiets.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    for (i, _) in &quiets {
        ordered.push(moves[*i]);
    }

    // --- 7. Losing captures (SEE < 0), MVV-LVA descending ---
    lose_caps.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    for (i, _) in &lose_caps {
        ordered.push(moves[*i]);
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
    use crate::eval::{BootstrapEvaluator, EvalProfile};
    use crate::gamestate::GameState;

    fn make_searcher() -> BrsSearcher {
        BrsSearcher::new(Box::new(BootstrapEvaluator::new(EvalProfile::Standard)), None)
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
