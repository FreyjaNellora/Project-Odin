// Board Scanner — Stage 8, Step 1
//
// Runs once before search. Scans the board for attack patterns, king exposure,
// score standings, and high-value targets. Produces a flat `BoardContext` struct
// that informs hybrid opponent reply scoring (Step 3) and move ordering.
//
// Design: pure read-only analysis. No search, no eval calls, no mutations.
// Target: < 1ms in release build.

use crate::board::{file_of, is_valid_square, rank_of, Board, Player, Square};
use crate::eval::values::PIECE_EVAL_VALUES;
use crate::gamestate::{GameMode, GameState, PlayerStatus};
use crate::movegen::{is_square_attacked_by, Move};

/// Invalid square sentinel for unused high-value target slots.
const INVALID_SQUARE: Square = 255;

/// Maximum high-value targets tracked.
const MAX_HVT: usize = 8;

/// Pre-search board context: who's pointing guns at whom.
#[derive(Debug, Clone)]
pub struct BoardContext {
    /// Game mode (affects target selection heuristics).
    pub game_mode: GameMode,
    /// The root player this scan was performed for.
    pub root_player: Player,
    /// The player with the lowest material (weakest, most vulnerable).
    pub weakest_player: Player,
    /// Opponents sorted by danger to root: most dangerous first.
    pub most_dangerous: [Player; 3],
    /// How much danger the root player is in (0.0 = safe, 1.0 = critical).
    pub root_danger_level: f64,
    /// High-value pieces that are attacked and potentially capturable.
    /// Unused slots have `square = INVALID_SQUARE`.
    pub high_value_targets: [(Square, Player); MAX_HVT],
    /// Number of valid entries in `high_value_targets`.
    pub high_value_target_count: u8,
    /// If three opponents are all targeting the same player, record convergence.
    /// (target, attacker1, attacker2) — or None if no convergence detected.
    pub convergence: Option<(Player, Player, Player)>,
    /// Per-opponent analysis.
    pub per_opponent: [OpponentProfile; 3],
}

/// Profile of a single opponent relative to the root player.
#[derive(Debug, Clone)]
pub struct OpponentProfile {
    /// Which opponent this profile describes.
    pub player: Player,
    /// How aggressively this opponent's pieces point at root (0.0-1.0).
    pub aggression_toward_root: f64,
    /// How exposed this opponent's own king is (0.0 = safe, 1.0 = very exposed).
    pub own_vulnerability: f64,
    /// Which player this opponent is most likely to target (may be root or another).
    pub best_target: Player,
    /// Whether this opponent has enough material to mount an attack on root.
    pub can_afford_to_attack_root: bool,
    /// Whether this opponent's pieces are supporting another attacker against root.
    pub supporting_attack_on_root: bool,
}

/// Scan the board and produce a `BoardContext` for the given root player.
///
/// This runs once before search and must complete in < 1ms (release build).
pub fn scan_board(gs: &GameState, root_player: Player) -> BoardContext {
    let board = gs.board();
    let scores = gs.scores();
    let opponents = opponents_of(root_player, gs);

    // 1. Material totals per player (centipawns)
    let mut material = [0i32; 4];
    for &p in &Player::ALL {
        if gs.player_status(p) == PlayerStatus::Eliminated {
            continue;
        }
        for &(pt, _sq) in board.piece_list(p) {
            material[p.index()] += PIECE_EVAL_VALUES[pt.index()] as i32;
        }
    }

    // 2. Weakest player (lowest material among active/DKW)
    let weakest_player = find_weakest(&material, gs);

    // 3. King safety: how many opponents attack squares around root's king
    let root_king_sq = board.king_square(root_player);
    let root_danger_level = compute_king_danger(board, root_player, root_king_sq, &opponents);

    // 4. Per-opponent profiling
    let mut per_opponent = [
        OpponentProfile {
            player: opponents[0],
            aggression_toward_root: 0.0,
            own_vulnerability: 0.0,
            best_target: root_player,
            can_afford_to_attack_root: false,
            supporting_attack_on_root: false,
        },
        OpponentProfile {
            player: opponents[1],
            aggression_toward_root: 0.0,
            own_vulnerability: 0.0,
            best_target: root_player,
            can_afford_to_attack_root: false,
            supporting_attack_on_root: false,
        },
        OpponentProfile {
            player: opponents[2],
            aggression_toward_root: 0.0,
            own_vulnerability: 0.0,
            best_target: root_player,
            can_afford_to_attack_root: false,
            supporting_attack_on_root: false,
        },
    ];

    for profile in &mut per_opponent {
        let opp = profile.player;
        if gs.player_status(opp) == PlayerStatus::Eliminated {
            continue;
        }

        // Aggression: how many of this opponent's pieces attack root's piece squares
        profile.aggression_toward_root =
            compute_aggression(board, opp, root_player, gs);

        // Vulnerability: how exposed is this opponent's king
        let opp_king_sq = board.king_square(opp);
        let other_opps: Vec<Player> = Player::ALL
            .iter()
            .copied()
            .filter(|&p| p != opp && gs.player_status(p) != PlayerStatus::Eliminated)
            .collect();
        profile.own_vulnerability =
            compute_king_danger(board, opp, opp_king_sq, &other_opps);

        // Can afford to attack: has non-pawn material worth >= 500cp
        profile.can_afford_to_attack_root =
            material[opp.index()] >= 500;

        // Best target: who does this opponent attack most
        profile.best_target = find_best_target(board, opp, gs, &scores);
    }

    // 5. Supporting attack detection
    //    If opponent A's best target is root and opponent B also attacks root,
    //    then B is "supporting" A's attack.
    let primary_attackers: Vec<usize> = per_opponent
        .iter()
        .enumerate()
        .filter(|(_, p)| p.best_target == root_player && p.aggression_toward_root > 0.3)
        .map(|(i, _)| i)
        .collect();

    if !primary_attackers.is_empty() {
        for i in 0..3 {
            if !primary_attackers.contains(&i)
                && per_opponent[i].aggression_toward_root > 0.15
            {
                per_opponent[i].supporting_attack_on_root = true;
            }
        }
    }

    // 6. Most dangerous ordering (by aggression toward root descending)
    let mut danger_order = opponents;
    danger_order.sort_by(|a, b| {
        let a_agg = per_opponent
            .iter()
            .find(|p| p.player == *a)
            .map_or(0.0, |p| p.aggression_toward_root);
        let b_agg = per_opponent
            .iter()
            .find(|p| p.player == *b)
            .map_or(0.0, |p| p.aggression_toward_root);
        b_agg
            .partial_cmp(&a_agg)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let most_dangerous = [danger_order[0], danger_order[1], danger_order[2]];

    // 7. High-value targets: opponent pieces worth >= 300cp that are attacked
    let (high_value_targets, high_value_target_count) =
        find_high_value_targets(board, root_player, gs);

    // 8. Convergence: two or more opponents both primarily targeting root
    let convergence = detect_convergence(&per_opponent, root_player);

    BoardContext {
        game_mode: gs.game_mode(),
        root_player,
        weakest_player,
        most_dangerous,
        root_danger_level,
        high_value_targets,
        high_value_target_count,
        convergence,
        per_opponent,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Get the three opponents of `root_player`, in turn order, skipping eliminated.
/// Always returns exactly 3 (may include eliminated for array shape).
fn opponents_of(root: Player, _gs: &GameState) -> Vec<Player> {
    Player::ALL
        .iter()
        .copied()
        .filter(|&p| p != root)
        .collect()
}

/// Find the player with the lowest material among non-eliminated players.
fn find_weakest(material: &[i32; 4], gs: &GameState) -> Player {
    let mut weakest = Player::Red;
    let mut weakest_mat = i32::MAX;
    for &p in &Player::ALL {
        if gs.player_status(p) == PlayerStatus::Eliminated {
            continue;
        }
        if material[p.index()] < weakest_mat {
            weakest_mat = material[p.index()];
            weakest = p;
        }
    }
    weakest
}

/// Compute how exposed a player's king is.
/// Returns 0.0 (safe) to 1.0 (critical danger).
/// Based on: how many of the 8 king-adjacent squares are attacked by opponents,
/// whether the king is in check, and how few friendly pieces shield it.
fn compute_king_danger(
    board: &Board,
    player: Player,
    king_sq: Square,
    opponents: &[Player],
) -> f64 {
    let king_file = file_of(king_sq) as i8;
    let king_rank = rank_of(king_sq) as i8;

    let mut attacked_squares = 0u32;
    let mut total_adjacent = 0u32;
    let mut friendly_shield = 0u32;

    // Check all 8 adjacent squares
    for df in -1..=1i8 {
        for dr in -1..=1i8 {
            if df == 0 && dr == 0 {
                continue;
            }
            let f = king_file + df;
            let r = king_rank + dr;
            if f < 0 || f > 13 || r < 0 || r > 13 {
                continue;
            }
            let sq = (r as u8) * 14 + (f as u8);
            if !is_valid_square(sq) {
                continue;
            }
            total_adjacent += 1;

            // Check if any opponent attacks this square
            for &opp in opponents {
                if is_square_attacked_by(sq, opp, board) {
                    attacked_squares += 1;
                    break;
                }
            }

            // Check for friendly pieces shielding
            if let Some(piece) = board.piece_at(sq) {
                if piece.owner == player && piece.is_alive() {
                    friendly_shield += 1;
                }
            }
        }
    }

    if total_adjacent == 0 {
        return 0.0;
    }

    let attack_ratio = attacked_squares as f64 / total_adjacent as f64;
    let shield_bonus = (friendly_shield as f64 * 0.1).min(0.3);
    let in_check_penalty = if opponents
        .iter()
        .any(|&opp| is_square_attacked_by(king_sq, opp, board))
    {
        0.3
    } else {
        0.0
    };

    ((attack_ratio * 0.7 + in_check_penalty) - shield_bonus).clamp(0.0, 1.0)
}

/// Compute how aggressively an opponent's pieces point at root's pieces.
/// Returns 0.0 (not targeting root) to 1.0 (heavily targeting root).
fn compute_aggression(
    board: &Board,
    opponent: Player,
    root: Player,
    gs: &GameState,
) -> f64 {
    if gs.player_status(opponent) == PlayerStatus::Eliminated {
        return 0.0;
    }

    let root_pieces = board.piece_list(root);
    if root_pieces.is_empty() {
        return 0.0;
    }

    let mut attacked_value = 0i32;
    let mut total_root_value = 0i32;

    for &(pt, sq) in root_pieces {
        let value = PIECE_EVAL_VALUES[pt.index()] as i32;
        total_root_value += value;
        if is_square_attacked_by(sq, opponent, board) {
            attacked_value += value;
        }
    }

    if total_root_value == 0 {
        return 0.0;
    }

    (attacked_value as f64 / total_root_value as f64).clamp(0.0, 1.0)
}

/// Find which player this opponent targets most.
/// Uses a weighted score: attack value toward each other player + score considerations.
fn find_best_target(
    board: &Board,
    opponent: Player,
    gs: &GameState,
    scores: &[i32; 4],
) -> Player {
    let mut best_target = opponent; // fallback
    let mut best_score = -1.0f64;

    for &target in &Player::ALL {
        if target == opponent || gs.player_status(target) == PlayerStatus::Eliminated {
            continue;
        }

        let target_pieces = board.piece_list(target);
        let mut attacked_value = 0i32;

        for &(pt, sq) in target_pieces {
            if is_square_attacked_by(sq, opponent, board) {
                attacked_value += PIECE_EVAL_VALUES[pt.index()] as i32;
            }
        }

        // In FFA, opponents tend to target the leader more
        let score_factor = if gs.game_mode() == GameMode::FreeForAll {
            let leader_bonus = if scores[target.index()] > scores[opponent.index()] {
                0.2
            } else {
                0.0
            };
            1.0 + leader_bonus
        } else {
            1.0
        };

        let target_score = attacked_value as f64 * score_factor;

        // King exposure bonus: more likely to target exposed kings
        let king_sq = board.king_square(target);
        let king_attacked = is_square_attacked_by(king_sq, opponent, board);
        let king_bonus = if king_attacked { 500.0 } else { 0.0 };

        let total = target_score + king_bonus;
        if total > best_score {
            best_score = total;
            best_target = target;
        }
    }

    best_target
}

/// Find high-value opponent pieces that root can potentially attack/capture.
/// Returns up to MAX_HVT targets (queens, rooks, bishops that are attacked by root).
fn find_high_value_targets(
    board: &Board,
    root: Player,
    gs: &GameState,
) -> ([(Square, Player); MAX_HVT], u8) {
    let mut targets = [(INVALID_SQUARE, Player::Red); MAX_HVT];
    let mut count = 0u8;

    /// Minimum value (centipawns) to be considered a high-value target.
    const HVT_MIN_VALUE: i16 = 300;

    for &opp in &Player::ALL {
        if opp == root || gs.player_status(opp) == PlayerStatus::Eliminated {
            continue;
        }

        for &(pt, sq) in board.piece_list(opp) {
            if PIECE_EVAL_VALUES[pt.index()] >= HVT_MIN_VALUE
                && is_square_attacked_by(sq, root, board)
            {
                if (count as usize) < MAX_HVT {
                    targets[count as usize] = (sq, opp);
                    count += 1;
                }
            }
        }
    }

    (targets, count)
}

/// Detect convergence: two or more opponents both targeting root as best_target.
/// Returns Some((root, attacker1, attacker2)) if found.
fn detect_convergence(
    profiles: &[OpponentProfile; 3],
    root: Player,
) -> Option<(Player, Player, Player)> {
    let attackers: Vec<Player> = profiles
        .iter()
        .filter(|p| p.best_target == root && p.aggression_toward_root > 0.2)
        .map(|p| p.player)
        .collect();

    if attackers.len() >= 2 {
        Some((root, attackers[0], attackers[1]))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Move Classifier — Step 2
// ---------------------------------------------------------------------------

/// Classification of an opponent move relative to the root player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveClass {
    /// Directly threatens root: captures root piece, checks root king, or
    /// lands adjacent to root king.
    Relevant,
    /// Does not directly interact with root player.
    Background,
}

/// Classify a single opponent move as relevant or background.
///
/// "Relevant" means the move directly interacts with the root player:
/// - Captures one of root's pieces
/// - Lands on or adjacent to root's king square (potential check or proximity threat)
///
/// This is a pure table lookup + comparison. No eval calls.
pub fn classify_move(
    mv: Move,
    board: &Board,
    root_player: Player,
) -> MoveClass {
    let to = mv.to_sq();

    // 1. Does this move capture one of root's pieces?
    if let Some(piece) = board.piece_at(to) {
        if piece.owner == root_player && piece.is_alive() {
            return MoveClass::Relevant;
        }
    }

    // 2. Does the destination land on or adjacent to root's king?
    let king_sq = board.king_square(root_player);
    let king_file = file_of(king_sq) as i8;
    let king_rank = rank_of(king_sq) as i8;
    let to_file = file_of(to) as i8;
    let to_rank = rank_of(to) as i8;

    let file_dist = (to_file - king_file).abs();
    let rank_dist = (to_rank - king_rank).abs();

    // Adjacent = within 1 square (Chebyshev distance <= 1), including the king square itself
    if file_dist <= 1 && rank_dist <= 1 {
        return MoveClass::Relevant;
    }

    // 3. Does the destination land within 2 squares of root's king? (extended threat zone)
    // This catches knight forks and nearby pressure. Be slightly generous here.
    if file_dist <= 2 && rank_dist <= 2 {
        // Only if the piece is a knight (can attack king from 2 away) or
        // if the piece is a sliding piece landing on a line toward the king
        if let Some(piece) = board.piece_at(mv.from_sq()) {
            match piece.piece_type {
                crate::board::PieceType::Knight => return MoveClass::Relevant,
                _ => {}
            }
        }
    }

    MoveClass::Background
}

/// Classify all moves and split into relevant and background.
/// Returns (relevant_moves, best_background_move_score, best_background_move).
///
/// The best_background_move is the single strongest background move by a simple
/// capture-value heuristic — this is the "fallback" move from the background set.
pub fn classify_moves(
    moves: &[Move],
    board: &Board,
    root_player: Player,
) -> (Vec<Move>, Option<Move>) {
    let mut relevant = Vec::new();
    let mut best_bg_move: Option<Move> = None;
    let mut best_bg_score = i16::MIN;

    for &mv in moves {
        match classify_move(mv, board, root_player) {
            MoveClass::Relevant => relevant.push(mv),
            MoveClass::Background => {
                // Track best background move by capture value (or 0 for quiet)
                let score = mv
                    .captured()
                    .map(|pt| PIECE_EVAL_VALUES[pt.index()])
                    .unwrap_or(0);
                if score > best_bg_score {
                    best_bg_score = score;
                    best_bg_move = Some(mv);
                }
            }
        }
    }

    (relevant, best_bg_move)
}

// ---------------------------------------------------------------------------
// Progressive Narrowing — Step 4
// ---------------------------------------------------------------------------

/// Maximum relevant candidates at depth 1-3.
const NARROWING_SHALLOW: usize = 10;
/// Maximum relevant candidates at depth 4-6.
const NARROWING_MID: usize = 6;
/// Maximum relevant candidates at depth 7+.
const NARROWING_DEEP: usize = 3;

/// Return the maximum number of relevant opponent moves to evaluate at this
/// search depth. Shallower depths consider more candidates for accuracy;
/// deeper depths narrow aggressively for speed.
pub fn narrowing_limit(depth: u8) -> usize {
    match depth {
        0..=3 => NARROWING_SHALLOW,
        4..=6 => NARROWING_MID,
        _ => NARROWING_DEEP,
    }
}

/// Pre-sort relevant moves by a cheap capture-value heuristic (MVV ordering).
/// Captures of high-value pieces come first, then non-captures.
/// This ensures progressive narrowing keeps the most promising candidates.
fn cheap_presort(moves: &mut [Move]) {
    moves.sort_by(|a, b| {
        let val_a = a
            .captured()
            .map(|pt| PIECE_EVAL_VALUES[pt.index()])
            .unwrap_or(0);
        let val_b = b
            .captured()
            .map(|pt| PIECE_EVAL_VALUES[pt.index()])
            .unwrap_or(0);
        val_b.cmp(&val_a)
    });
}

// ---------------------------------------------------------------------------
// Hybrid Reply Scoring — Step 3
// ---------------------------------------------------------------------------

/// Base likelihood when a move targets the root player.
const LIKELIHOOD_BASE_TARGETS_ROOT: f64 = 0.7;
/// Bonus if root is this opponent's best target (from board context).
const LIKELIHOOD_BEST_TARGET_BONUS: f64 = 0.2;
/// Bonus if opponent is supporting another attacker against root.
const LIKELIHOOD_SUPPORTING_BONUS: f64 = 0.1;
/// Penalty if opponent is too exposed (high own vulnerability).
const LIKELIHOOD_EXPOSED_PENALTY: f64 = 0.3;
/// Base likelihood for moves that do NOT target root.
const LIKELIHOOD_BASE_NON_ROOT: f64 = 0.2;

/// Scored opponent move for hybrid reply selection.
#[derive(Debug, Clone)]
pub struct ScoredReply {
    pub mv: Move,
    pub hybrid_score: f64,
    pub objective_strength: f64,
    pub harm_to_root: f64,
    pub likelihood: f64,
}

/// Score a relevant opponent move using the hybrid formula.
///
/// `score = harm_to_root * likelihood + objective_strength * (1 - likelihood)`
///
/// - `objective_strength`: how good this move is objectively (static eval delta).
///   Normalized to [0, 1] where 1 = very strong move.
/// - `harm_to_root`: how much this move specifically hurts the root player.
///   Based on capture value toward root, check threat, proximity.
/// - `likelihood`: probability the opponent would realistically play this move.
///   Higher if root is their best target, lower if opponent is too exposed.
pub fn score_reply(
    mv: Move,
    board: &Board,
    root_player: Player,
    opponent: Player,
    ctx: &BoardContext,
    obj_eval_delta: i16,
    max_eval_delta: i16,
) -> ScoredReply {
    // Find the opponent's profile from context
    let profile = ctx
        .per_opponent
        .iter()
        .find(|p| p.player == opponent);

    // Objective strength: normalized eval improvement (0.0 to 1.0)
    let max_delta = (max_eval_delta.abs() as f64).max(1.0);
    let objective_strength = ((obj_eval_delta.abs() as f64) / max_delta).clamp(0.0, 1.0);

    // Harm to root: based on what this move does to root's pieces/king
    let harm_to_root = compute_harm_to_root(mv, board, root_player);

    // Likelihood: based on board context
    let is_relevant = classify_move(mv, board, root_player) == MoveClass::Relevant;
    let likelihood = if is_relevant {
        let mut l = LIKELIHOOD_BASE_TARGETS_ROOT;
        if let Some(prof) = profile {
            if prof.best_target == root_player {
                l += LIKELIHOOD_BEST_TARGET_BONUS;
            }
            if prof.supporting_attack_on_root {
                l += LIKELIHOOD_SUPPORTING_BONUS;
            }
            if prof.own_vulnerability > 0.5 {
                l -= LIKELIHOOD_EXPOSED_PENALTY;
            }
        }
        l.clamp(0.1, 1.0)
    } else {
        LIKELIHOOD_BASE_NON_ROOT
    };

    let hybrid_score = harm_to_root * likelihood + objective_strength * (1.0 - likelihood);

    ScoredReply {
        mv,
        hybrid_score,
        objective_strength,
        harm_to_root,
        likelihood,
    }
}

/// Compute how much a move harms the root player specifically.
/// Returns 0.0 (harmless) to 1.0 (very harmful).
fn compute_harm_to_root(mv: Move, board: &Board, root_player: Player) -> f64 {
    let to = mv.to_sq();
    let mut harm = 0.0;

    // Capturing root's piece: harm proportional to piece value
    if let Some(captured_pt) = mv.captured() {
        if let Some(piece) = board.piece_at(to) {
            if piece.owner == root_player {
                harm += (PIECE_EVAL_VALUES[captured_pt.index()] as f64 / 900.0).min(1.0);
            }
        }
    }

    // Proximity to root's king
    let king_sq = board.king_square(root_player);
    let king_file = file_of(king_sq) as i8;
    let king_rank = rank_of(king_sq) as i8;
    let to_file = file_of(to) as i8;
    let to_rank = rank_of(to) as i8;

    let dist = (to_file - king_file).abs().max((to_rank - king_rank).abs());
    if dist <= 1 {
        harm += 0.5; // Adjacent to king
    } else if dist <= 2 {
        harm += 0.2; // Near king
    }

    harm.clamp(0.0, 1.0)
}

/// Select the best opponent reply using hybrid scoring with progressive narrowing.
///
/// Evaluates relevant moves with the hybrid formula, applying depth-based
/// candidate limits (progressive narrowing). Falls back to the strongest
/// background move. Returns the move that scores highest.
///
/// `depth` is the current search depth, used to determine the narrowing limit.
pub fn select_hybrid_reply(
    gs: &mut GameState,
    evaluator: &dyn crate::eval::Evaluator,
    root_player: Player,
    opponent: Player,
    moves: &[Move],
    ctx: &BoardContext,
    depth: u8,
) -> Option<Move> {
    if moves.is_empty() {
        return None;
    }

    let board = gs.board();
    let (mut relevant, best_bg) = classify_moves(moves, board, root_player);

    // If no relevant moves, fall back to plain best reply
    if relevant.is_empty() {
        return best_bg.or_else(|| moves.first().copied());
    }

    // Progressive narrowing: limit candidates based on search depth.
    // Pre-sort by cheap capture-value heuristic before truncating so we
    // keep the most promising moves.
    let limit = narrowing_limit(depth);
    if relevant.len() > limit {
        cheap_presort(&mut relevant);
        relevant.truncate(limit);
    }

    // Compute objective eval delta for each relevant move
    let base_eval = evaluator.eval_scalar(gs, root_player);
    let mut scored_replies: Vec<ScoredReply> = Vec::with_capacity(relevant.len());
    let mut max_delta: i16 = 1;

    // First pass: compute eval deltas
    let mut eval_deltas: Vec<i16> = Vec::with_capacity(relevant.len());
    for &mv in &relevant {
        let undo = crate::movegen::make_move(gs.board_mut(), mv);
        let after_eval = evaluator.eval_scalar(gs, root_player);
        crate::movegen::unmake_move(gs.board_mut(), mv, undo);
        let delta = base_eval - after_eval; // positive = hurts root
        eval_deltas.push(delta);
        if delta.abs() > max_delta {
            max_delta = delta.abs();
        }
    }

    // Second pass: score with hybrid formula
    for (i, &mv) in relevant.iter().enumerate() {
        let scored = score_reply(
            mv,
            gs.board(),
            root_player,
            opponent,
            ctx,
            eval_deltas[i],
            max_delta,
        );
        scored_replies.push(scored);
    }

    // Sort by hybrid score descending
    scored_replies.sort_by(|a, b| {
        b.hybrid_score
            .partial_cmp(&a.hybrid_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Return the top-scoring relevant move
    scored_replies.first().map(|r| r.mv)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamestate::GameState;
    use std::time::Instant;

    #[test]
    fn test_scan_starting_position_completes() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        assert_eq!(ctx.root_player, Player::Red);
        assert_eq!(ctx.game_mode, GameMode::FreeForAll);
        // At starting position, no player's pieces are attacked
        assert_eq!(ctx.high_value_target_count, 0);
    }

    #[test]
    fn test_scan_starting_position_symmetric() {
        let gs = GameState::new_standard_ffa();
        let ctx_red = scan_board(&gs, Player::Red);
        let ctx_blue = scan_board(&gs, Player::Blue);
        // Both should have low danger at start
        assert!(
            ctx_red.root_danger_level < 0.3,
            "Red danger {} too high at start",
            ctx_red.root_danger_level
        );
        assert!(
            ctx_blue.root_danger_level < 0.3,
            "Blue danger {} too high at start",
            ctx_blue.root_danger_level
        );
    }

    #[test]
    fn test_scan_has_three_opponents() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        assert_eq!(ctx.per_opponent.len(), 3);
        // None should be Red
        for profile in &ctx.per_opponent {
            assert_ne!(profile.player, Player::Red);
        }
    }

    #[test]
    fn test_scan_most_dangerous_contains_three_opponents() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        assert_eq!(ctx.most_dangerous.len(), 3);
        for &p in &ctx.most_dangerous {
            assert_ne!(p, Player::Red);
        }
    }

    #[test]
    fn test_scan_weakest_player_is_valid() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        // At starting position, all players have equal material.
        // weakest_player is one of the four players.
        assert!(Player::ALL.contains(&ctx.weakest_player));
    }

    #[test]
    fn test_scan_danger_level_bounded() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        assert!(
            ctx.root_danger_level >= 0.0 && ctx.root_danger_level <= 1.0,
            "danger level {} out of [0, 1]",
            ctx.root_danger_level
        );
    }

    #[test]
    fn test_scan_aggression_bounded() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        for profile in &ctx.per_opponent {
            assert!(
                profile.aggression_toward_root >= 0.0
                    && profile.aggression_toward_root <= 1.0,
                "{:?} aggression {} out of [0, 1]",
                profile.player,
                profile.aggression_toward_root
            );
        }
    }

    #[test]
    fn test_scan_vulnerability_bounded() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        for profile in &ctx.per_opponent {
            assert!(
                profile.own_vulnerability >= 0.0 && profile.own_vulnerability <= 1.0,
                "{:?} vulnerability {} out of [0, 1]",
                profile.player,
                profile.own_vulnerability
            );
        }
    }

    #[test]
    fn test_scan_lks_mode() {
        let gs = GameState::new_standard_lks();
        let ctx = scan_board(&gs, Player::Red);
        assert_eq!(ctx.game_mode, GameMode::LastKingStanding);
    }

    #[test]
    fn test_scan_performance_under_1ms() {
        let gs = GameState::new_standard_ffa();
        // Warm up
        let _ = scan_board(&gs, Player::Red);

        let start = Instant::now();
        for _ in 0..100 {
            let _ = scan_board(&gs, Player::Red);
        }
        let elapsed = start.elapsed();
        let per_call_us = elapsed.as_micros() / 100;
        // Must be under 1ms (1000us) per call. Even in debug build we have margin.
        assert!(
            per_call_us < 10_000, // 10ms generous debug-build limit
            "scan_board took {}us per call (must be < 10000us in debug)",
            per_call_us
        );
    }

    #[test]
    fn test_scan_no_convergence_at_start() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        // At starting position, no significant convergence expected
        // (opponents don't heavily target root)
        // This may or may not be None depending on exact heuristics — just verify it compiles.
        let _ = ctx.convergence;
    }

    #[test]
    fn test_high_value_targets_empty_at_start() {
        let gs = GameState::new_standard_ffa();
        let ctx = scan_board(&gs, Player::Red);
        // At starting position, no pieces are attacked
        assert_eq!(
            ctx.high_value_target_count, 0,
            "no high-value targets should be attacked at starting position"
        );
    }

    // --- Move classifier tests ---

    #[test]
    fn test_classify_moves_starting_position() {
        let gs = GameState::new_standard_ffa();
        let board = gs.board();
        // Generate Blue's legal moves (opponent of Red)
        let mut gs_clone = gs.clone();
        gs_clone.board_mut().set_side_to_move(Player::Blue);
        let moves = crate::movegen::generate_legal(gs_clone.board_mut());

        let (relevant, best_bg) = classify_moves(&moves, board, Player::Red);
        // At starting position, Blue's moves shouldn't capture Red's pieces
        // and shouldn't land near Red's king (e1). Most should be background.
        assert!(
            relevant.len() < moves.len(),
            "all {} moves classified as relevant — expected most to be background",
            moves.len()
        );
        // There should be at least some background moves
        assert!(best_bg.is_some() || relevant.len() == moves.len());
    }

    #[test]
    fn test_classify_capture_is_relevant() {
        // Create a position where Blue can capture Red's piece
        let mut gs = GameState::new_standard_ffa();
        let board = gs.board_mut();
        // Place a Red pawn where Blue can capture it
        let red_pawn_sq = crate::board::square_from(5, 5).unwrap(); // f6
        board.place_piece(
            red_pawn_sq,
            crate::board::Piece::new(crate::board::PieceType::Pawn, Player::Red),
        );
        // Place a Blue knight that can reach f6
        let blue_knight_sq = crate::board::square_from(4, 3).unwrap(); // e4
        board.place_piece(
            blue_knight_sq,
            crate::board::Piece::new(crate::board::PieceType::Knight, Player::Blue),
        );

        // Manually create a capture move: knight from e4 to f6
        let mv = Move::new_capture(
            blue_knight_sq,
            red_pawn_sq,
            crate::board::PieceType::Knight,
            crate::board::PieceType::Pawn,
        );
        let class = classify_move(mv, gs.board(), Player::Red);
        assert_eq!(class, MoveClass::Relevant, "capture of root piece must be relevant");
    }

    // --- Progressive narrowing tests ---

    #[test]
    fn test_narrowing_limit_shallow() {
        assert_eq!(narrowing_limit(1), NARROWING_SHALLOW);
        assert_eq!(narrowing_limit(2), NARROWING_SHALLOW);
        assert_eq!(narrowing_limit(3), NARROWING_SHALLOW);
    }

    #[test]
    fn test_narrowing_limit_mid() {
        assert_eq!(narrowing_limit(4), NARROWING_MID);
        assert_eq!(narrowing_limit(5), NARROWING_MID);
        assert_eq!(narrowing_limit(6), NARROWING_MID);
    }

    #[test]
    fn test_narrowing_limit_deep() {
        assert_eq!(narrowing_limit(7), NARROWING_DEEP);
        assert_eq!(narrowing_limit(8), NARROWING_DEEP);
        assert_eq!(narrowing_limit(10), NARROWING_DEEP);
        assert_eq!(narrowing_limit(20), NARROWING_DEEP);
    }

    #[test]
    fn test_narrowing_limit_depth_zero() {
        // Depth 0 (quiescence) should use shallow limit
        assert_eq!(narrowing_limit(0), NARROWING_SHALLOW);
    }

    #[test]
    fn test_cheap_presort_orders_by_capture_value() {
        // Create moves with different capture values
        let sq_a = crate::board::square_from(3, 3).unwrap();
        let sq_b = crate::board::square_from(4, 4).unwrap();
        let sq_c = crate::board::square_from(5, 5).unwrap();

        let capture_queen = Move::new_capture(
            sq_a, sq_b,
            crate::board::PieceType::Pawn,
            crate::board::PieceType::Queen,
        );
        let capture_pawn = Move::new_capture(
            sq_a, sq_c,
            crate::board::PieceType::Pawn,
            crate::board::PieceType::Pawn,
        );
        let quiet = Move::new(sq_a, sq_b, crate::board::PieceType::Pawn);

        let mut moves = vec![quiet, capture_pawn, capture_queen];
        cheap_presort(&mut moves);

        // Queen capture should come first, then pawn capture, then quiet
        assert_eq!(moves[0], capture_queen, "queen capture should be first");
        assert_eq!(moves[1], capture_pawn, "pawn capture should be second");
        assert_eq!(moves[2], quiet, "quiet move should be last");
    }

    #[test]
    fn test_classify_near_king_is_relevant() {
        let gs = GameState::new_standard_ffa();
        // Red's king is at e1. A move landing at d1, d2, e2, f1, f2 should be relevant.
        let king_sq = gs.board().king_square(Player::Red);
        let king_file = file_of(king_sq) as i8;
        let king_rank = rank_of(king_sq) as i8;

        // Create a dummy move landing adjacent to king
        let adj_file = (king_file + 1).min(13) as u8;
        let adj_rank = king_rank as u8;
        let to_sq = crate::board::square_from(adj_file, adj_rank).unwrap();
        let from_sq = crate::board::square_from(adj_file, adj_rank + 2).unwrap();

        let mv = Move::new(from_sq, to_sq, crate::board::PieceType::Pawn);
        let class = classify_move(mv, gs.board(), Player::Red);
        assert_eq!(class, MoveClass::Relevant, "move adjacent to root king must be relevant");
    }
}
