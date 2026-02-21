// Evaluation — Stages 6, 14-16
//
// The Evaluator trait is the eval boundary. All search code calls through
// this trait, never a specific implementation. Bootstrap handcrafted eval
// (this stage) is replaced by NNUE in Stage 16.

mod king_safety;
mod material;
mod multi_player;
mod pst;
pub mod values;

use crate::board::Player;
use crate::gamestate::{GameState, PlayerStatus};

pub use values::{
    BISHOP_EVAL_VALUE, KING_EVAL_VALUE, KNIGHT_EVAL_VALUE, PAWN_EVAL_VALUE, PIECE_EVAL_VALUES,
    PROMOTED_QUEEN_EVAL_VALUE, QUEEN_EVAL_VALUE, ROOK_EVAL_VALUE,
};

/// Score assigned to eliminated players (extreme negative).
const ELIMINATED_SCORE: i16 = -30_000;

/// Sigmoid scaling constant for eval_4vec normalization.
/// At 0cp -> 0.5, +300cp -> ~0.68, +900cp -> ~0.90, -30000cp -> ~0.0.
const SIGMOID_K: f64 = 400.0;

/// The evaluation boundary trait. All search code calls through this trait.
///
/// Permanent contract: these signatures persist through the entire project.
/// Bootstrap handcrafted eval implements it in Stage 6. NNUE replaces it in
/// Stage 16. Nothing above the trait changes.
pub trait Evaluator {
    /// Centipawn evaluation from one player's perspective.
    /// Range: approximately -30000 to +30000.
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16;

    /// Evaluation from all 4 perspectives, normalized to \[0,1\] via sigmoid.
    /// Index by `Player::index()`: 0=Red, 1=Blue, 2=Yellow, 3=Green.
    fn eval_4vec(&self, position: &GameState) -> [f64; 4];
}

/// Bootstrap handcrafted evaluator. Implements the Evaluator trait.
///
/// Components: material counting, piece-square tables, king safety,
/// multi-player relative eval (lead penalty, threat penalty), FFA points.
///
/// This is temporary — replaced by NnueEvaluator in Stage 16.
pub struct BootstrapEvaluator;

impl BootstrapEvaluator {
    /// Create a new bootstrap evaluator (stateless).
    pub fn new() -> Self {
        Self
    }
}

impl Default for BootstrapEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator for BootstrapEvaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16 {
        eval_for_player(position, player)
    }

    fn eval_4vec(&self, position: &GameState) -> [f64; 4] {
        let mut raw = [0i16; 4];
        for &p in &Player::ALL {
            raw[p.index()] = eval_for_player(position, p);
        }
        normalize_4vec(&raw)
    }
}

/// Evaluate a position from one player's perspective. Returns centipawns.
///
/// Formula (per MASTERPLAN Stage 6 spec):
///   material + positional + king_safety - threat + lead_penalty + ffa_points
fn eval_for_player(position: &GameState, player: Player) -> i16 {
    if position.player_status(player) == PlayerStatus::Eliminated {
        return ELIMINATED_SCORE;
    }

    let board = position.board();
    let statuses = player_statuses(position);

    let mat = material::material_score(board, player);
    let pos = pst::positional_score(board, player);
    let king = king_safety::king_safety_score(board, player, &statuses);
    let threat = multi_player::threat_penalty(board, player, &statuses);
    let lead = multi_player::lead_penalty(
        player,
        &material::material_scores(board),
        &position.scores(),
        &statuses,
    );
    let ffa = multi_player::ffa_points_eval(position.score(player));

    // Combine with saturating arithmetic to avoid i16 overflow.
    mat.saturating_add(pos)
        .saturating_add(king)
        .saturating_sub(threat)
        .saturating_add(lead)
        .saturating_add(ffa)
        .clamp(-30_000, 30_000)
}

/// Extract player statuses into a fixed array for passing to eval components.
fn player_statuses(position: &GameState) -> [PlayerStatus; 4] {
    [
        position.player_status(Player::Red),
        position.player_status(Player::Blue),
        position.player_status(Player::Yellow),
        position.player_status(Player::Green),
    ]
}

/// Normalize raw centipawn scores to [0,1] via sigmoid.
/// Each player's value is independent (not softmax).
fn normalize_4vec(raw: &[i16; 4]) -> [f64; 4] {
    let mut result = [0.0f64; 4];
    for i in 0..4 {
        result[i] = 1.0 / (1.0 + (-f64::from(raw[i]) / SIGMOID_K).exp());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamestate::GameState;

    #[test]
    fn test_bootstrap_evaluator_implements_trait() {
        let evaluator = BootstrapEvaluator::new();
        let gs = GameState::new_standard_ffa();
        let _scalar = evaluator.eval_scalar(&gs, Player::Red);
        let _vec = evaluator.eval_4vec(&gs);
    }

    #[test]
    fn test_sigmoid_at_zero_is_half() {
        let raw = [0i16; 4];
        let result = normalize_4vec(&raw);
        for &v in &result {
            assert!((v - 0.5).abs() < 1e-10);
        }
    }

    #[test]
    fn test_sigmoid_monotonic() {
        let raw = [100, 0, -100, -500];
        let result = normalize_4vec(&raw);
        assert!(result[0] > result[1]);
        assert!(result[1] > result[2]);
        assert!(result[2] > result[3]);
    }

    #[test]
    fn test_sigmoid_bounded_01() {
        let raw = [30_000, -30_000, 0, 500];
        let result = normalize_4vec(&raw);
        for &v in &result {
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_default_evaluator() {
        let evaluator = BootstrapEvaluator::default();
        let gs = GameState::new_standard_ffa();
        let score = evaluator.eval_scalar(&gs, Player::Red);
        // Starting position should have a reasonable score.
        assert!(score > -30_000 && score < 30_000);
    }
}
