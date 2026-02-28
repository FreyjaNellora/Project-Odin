// Evaluation — Stages 6, 8, 14-16
//
// The Evaluator trait is the eval boundary. All search code calls through
// this trait, never a specific implementation. Bootstrap handcrafted eval
// (this stage) is replaced by NNUE in Stage 16.

mod development;
mod king_safety;
mod material;
mod multi_player;
mod pawn_structure;
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
/// Must match the inverse sigmoid constant in mcts::q_to_centipawns().
///
/// K=4000 keeps bootstrap eval's typical range (3000-5000cp) in the
/// sigmoid's discriminating region: 3000cp -> ~0.68, 4000cp -> ~0.73,
/// 5000cp -> ~0.78, -30000cp -> ~0.0.
///
/// K=400 (the original value) caused all positions to saturate at Q≈1.0,
/// making MCTS unable to distinguish between moves.
const SIGMOID_K: f64 = 4000.0;

// ---------------------------------------------------------------------------
// EvalProfile — evaluation personality (ADR-014)
// ---------------------------------------------------------------------------

/// Evaluation personality profile.
///
/// Controls how aggressively the engine evaluates positions. Independent of
/// `GameMode` (which controls rules). Default pairing: FFA → Aggressive,
/// LKS → Standard. Override via `setoption name EvalProfile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalProfile {
    /// Conservative: lead penalty active, moderate FFA point weight.
    /// Default for Last King Standing.
    Standard,
    /// FFA-optimized: no lead penalty, high FFA point weight.
    /// Default for Free For All.
    Aggressive,
}

/// Tunable eval weights derived from an `EvalProfile`.
#[derive(Debug, Clone, Copy)]
pub struct EvalWeights {
    /// Centipawns per FFA point.
    pub ffa_point_weight: i16,
    /// Whether the lead penalty is applied.
    pub lead_penalty_enabled: bool,
    /// Lead penalty scaling: penalty = lead / divisor.
    pub lead_penalty_divisor: i16,
    /// Maximum lead penalty in centipawns.
    pub max_lead_penalty: i16,
}

impl EvalProfile {
    /// Return the eval weights for this profile.
    pub fn weights(self) -> EvalWeights {
        match self {
            EvalProfile::Standard => EvalWeights {
                ffa_point_weight: 50,
                lead_penalty_enabled: true,
                lead_penalty_divisor: 4,
                max_lead_penalty: 150,
            },
            EvalProfile::Aggressive => EvalWeights {
                ffa_point_weight: 120,
                lead_penalty_enabled: false,
                lead_penalty_divisor: 4,
                max_lead_penalty: 0,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluator trait (permanent contract)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// BootstrapEvaluator
// ---------------------------------------------------------------------------

/// Bootstrap handcrafted evaluator. Implements the Evaluator trait.
///
/// Components: material counting, piece-square tables, king safety,
/// multi-player relative eval (lead penalty, threat penalty), FFA points.
///
/// This is temporary — replaced by NnueEvaluator in Stage 16.
pub struct BootstrapEvaluator {
    weights: EvalWeights,
}

impl BootstrapEvaluator {
    /// Create a new bootstrap evaluator with the given eval profile.
    pub fn new(profile: EvalProfile) -> Self {
        Self {
            weights: profile.weights(),
        }
    }
}

impl Evaluator for BootstrapEvaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16 {
        eval_for_player(position, player, &self.weights)
    }

    fn eval_4vec(&self, position: &GameState) -> [f64; 4] {
        let mut raw = [0i16; 4];
        for &p in &Player::ALL {
            raw[p.index()] = eval_for_player(position, p, &self.weights);
        }
        normalize_4vec(&raw)
    }
}

/// Evaluate a position from one player's perspective. Returns centipawns.
///
/// Formula:
///   material + positional + development + pawn_structure + king_safety - threat + lead_penalty + ffa_points + relative_material
fn eval_for_player(position: &GameState, player: Player, weights: &EvalWeights) -> i16 {
    if position.player_status(player) == PlayerStatus::Eliminated {
        return ELIMINATED_SCORE;
    }

    let board = position.board();
    let statuses = player_statuses(position);

    let mat = material::material_score(board, player);
    let pos = pst::positional_score(board, player);
    let dev = development::development_score(board, player);
    let pawn = pawn_structure::connected_pawn_score(board, player);
    let king = king_safety::king_safety_score(board, player, &statuses);
    let threat = multi_player::threat_penalty(board, player, &statuses);
    let lead = multi_player::lead_penalty(
        player,
        &material::material_scores(board),
        &position.scores(),
        &statuses,
        weights.lead_penalty_enabled,
        weights.lead_penalty_divisor,
        weights.max_lead_penalty,
        weights.ffa_point_weight,
    );
    let ffa = multi_player::ffa_points_eval(position.score(player), weights.ffa_point_weight);
    let rel_mat = material::relative_material_advantage(board, player, &statuses);

    // NOTE: hanging_piece_penalty was removed here — it double-counted capture
    // threats already handled by the search tree, causing the engine to retreat
    // developed pieces (Nf3→e1 regression in v0.4.3). The narrowing fix
    // (root-capture protection) addresses hanging pieces through search instead.

    // Combine with saturating arithmetic to avoid i16 overflow.
    mat.saturating_add(pos)
        .saturating_add(dev)
        .saturating_add(pawn)
        .saturating_add(king)
        .saturating_sub(threat)
        .saturating_add(lead)
        .saturating_add(ffa)
        .saturating_add(rel_mat)
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
        let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
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
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn test_standard_profile_matches_original_behavior() {
        // Standard profile should produce the same scores as the original evaluator.
        let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
        let gs = GameState::new_standard_ffa();
        let score = evaluator.eval_scalar(&gs, Player::Red);
        assert!(score > -30_000 && score < 30_000);
    }

    #[test]
    fn test_aggressive_profile_valid_scores() {
        let evaluator = BootstrapEvaluator::new(EvalProfile::Aggressive);
        let gs = GameState::new_standard_ffa();
        let score = evaluator.eval_scalar(&gs, Player::Red);
        assert!(score > -30_000 && score < 30_000);
    }

    #[test]
    fn test_aggressive_no_lead_penalty() {
        // With aggressive profile, being ahead should not reduce the score.
        let weights = EvalProfile::Aggressive.weights();
        assert!(!weights.lead_penalty_enabled);
        assert_eq!(weights.max_lead_penalty, 0);
    }

    #[test]
    fn test_standard_has_lead_penalty() {
        let weights = EvalProfile::Standard.weights();
        assert!(weights.lead_penalty_enabled);
        assert_eq!(weights.max_lead_penalty, 150);
        assert_eq!(weights.lead_penalty_divisor, 4);
    }

    #[test]
    fn test_aggressive_higher_ffa_weight() {
        let std_w = EvalProfile::Standard.weights();
        let agg_w = EvalProfile::Aggressive.weights();
        assert!(agg_w.ffa_point_weight > std_w.ffa_point_weight);
        assert_eq!(std_w.ffa_point_weight, 50);
        assert_eq!(agg_w.ffa_point_weight, 120);
    }

    #[test]
    fn test_eval_weights_debug() {
        // EvalWeights and EvalProfile should derive Debug.
        let w = EvalProfile::Standard.weights();
        let _ = format!("{:?}", w);
        let _ = format!("{:?}", EvalProfile::Aggressive);
    }
}
