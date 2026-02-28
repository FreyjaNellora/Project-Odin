// NNUE Evaluation — Stage 14
//
// Dual-head NNUE: BRS scalar head (centipawns) + MCTS value head (4-player sigmoid).
// Feature set: HalfKP-4 (4,480 features per perspective).
// Network: FT(4480→256) x4 → concat(1024) → hidden(32) → dual output.
// Quantized inference: int16 accumulator, int8 hidden, int32 output.
//
// Stage 14 scope: inference-only with random weights. No training (Stage 15),
// no search integration (Stage 16).

pub mod accumulator;
pub mod features;
pub mod weights;

use std::cell::RefCell;

use crate::board::Player;
use crate::eval::Evaluator;
use crate::gamestate::{GameState, PlayerStatus};

use accumulator::{Accumulator, AccumulatorStack};
use features::{FT_OUT, HIDDEN_SIZE, MCTS_OUTPUT, QA};
use weights::NnueWeights;

// Re-export key types.
pub use accumulator::AccumulatorStack as AccStack;
pub use features::{
    active_features, feature_index, relative_owner, FEATURES_PER_PERSPECTIVE, MAX_STACK_DEPTH,
};
pub use weights::{NnueLoadError, NnueWeights as Weights};

/// Eliminated player score in centipawns.
const ELIMINATED_SCORE: i16 = -30_000;

/// Sigmoid scaling constant. Must match eval::SIGMOID_K for compatibility
/// with the existing BootstrapEvaluator's normalization.
const SIGMOID_K: f64 = 4000.0;

/// Output rescaling divisor (raw int32 → centipawns for BRS head).
const OUTPUT_SCALE: i32 = features::OUTPUT_SCALE;

// ---------------------------------------------------------------------------
// Forward pass
// ---------------------------------------------------------------------------

/// SCReLU activation: clamp(x, 0, QA)^2.
/// Returns i32 to accommodate QA^2 = 65,025.
#[inline]
fn screlu(x: i16) -> i32 {
    let clamped = x.clamp(0, QA) as i32;
    clamped * clamped
}

/// Quantized forward pass through the network.
///
/// Takes the current accumulator (all 4 perspectives) and the root player,
/// returns (brs_cp, mcts_values).
///
/// Perspective ordering: `player` is index 0 (root), then CW turn order.
#[allow(clippy::needless_range_loop)]
pub fn forward_pass(
    acc: &Accumulator,
    weights: &NnueWeights,
    player: Player,
) -> (i16, [f64; 4]) {
    // Step 1: SCReLU activation on each perspective's accumulator.
    // Order: root player first, then CW opponents in turn order.
    let mut activated = [0i32; FT_OUT * 4];
    for p in 0..4 {
        let pidx = (player.index() + p) % 4;
        for j in 0..FT_OUT {
            activated[p * FT_OUT + j] = screlu(acc.values[pidx][j]);
        }
    }

    // Step 2: Hidden layer (1024 → 32).
    // int8 weights × (i32 activated / QA) → int32 accumulation + ClippedReLU.
    let qa = QA as i32;
    let mut hidden = [0i32; HIDDEN_SIZE];
    for h in 0..HIDDEN_SIZE {
        hidden[h] = weights.hidden_biases[h];
        for i in 0..(FT_OUT * 4) {
            let w = weights.hidden_weights[i * HIDDEN_SIZE + h] as i32;
            hidden[h] += w * (activated[i] / qa);
        }
        // ClippedReLU
        hidden[h] = hidden[h].max(0);
    }

    // Step 3: BRS scalar head (32 → 1).
    let mut brs_raw: i32 = weights.brs_bias;
    for h in 0..HIDDEN_SIZE {
        brs_raw += hidden[h] * weights.brs_weights[h] as i32;
    }
    let brs_cp = (brs_raw / OUTPUT_SCALE).clamp(-30_000, 30_000) as i16;

    // Step 4: MCTS value head (32 → 4, per-player sigmoid).
    let mut mcts_values = [0.0f64; 4];
    for v in 0..MCTS_OUTPUT {
        let mut raw: i32 = weights.mcts_biases[v];
        for h in 0..HIDDEN_SIZE {
            raw += hidden[h] * weights.mcts_weights[h * MCTS_OUTPUT + v] as i32;
        }
        // Per-player sigmoid: 1 / (1 + exp(-x / SIGMOID_K))
        let x = raw as f64 / OUTPUT_SCALE as f64;
        mcts_values[v] = 1.0 / (1.0 + (-x / SIGMOID_K).exp());
    }

    // Rotate MCTS values back to absolute player order.
    // Currently: mcts_values[0] = root player's value,
    //            mcts_values[1] = CW opponent, etc.
    let mut absolute = [0.0f64; 4];
    for v in 0..4 {
        let abs_idx = (player.index() + v) % 4;
        absolute[abs_idx] = mcts_values[v];
    }

    (brs_cp, absolute)
}

// ---------------------------------------------------------------------------
// NnueEvaluator
// ---------------------------------------------------------------------------

/// NNUE evaluator implementing the frozen Evaluator trait.
///
/// Uses `RefCell<AccumulatorStack>` for interior mutability because the
/// Evaluator trait requires `&self` (not `&mut self`).
///
/// In Stage 14 (inference-only, no search integration), eval_scalar and
/// eval_4vec do a full refresh every call. Incremental updates are tested
/// separately via AccumulatorStack. Search integration is Stage 16.
pub struct NnueEvaluator {
    weights: NnueWeights,
    accumulator_stack: RefCell<AccumulatorStack>,
}

impl NnueEvaluator {
    /// Create an evaluator with the given weights.
    pub fn new(weights: NnueWeights) -> Self {
        Self {
            weights,
            accumulator_stack: RefCell::new(AccumulatorStack::new()),
        }
    }

    /// Create an evaluator with random weights (for testing).
    pub fn with_random_weights(seed: u64) -> Self {
        Self::new(NnueWeights::random(seed))
    }

    /// Get a reference to the weights.
    pub fn weights(&self) -> &NnueWeights {
        &self.weights
    }
}

impl Evaluator for NnueEvaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16 {
        if position.player_status(player) == PlayerStatus::Eliminated {
            return ELIMINATED_SCORE;
        }

        let mut stack = self.accumulator_stack.borrow_mut();
        stack.init_from_board(position.board(), &self.weights);
        let acc = stack.current();
        let (brs_score, _) = forward_pass(acc, &self.weights, player);
        brs_score
    }

    fn eval_4vec(&self, position: &GameState) -> [f64; 4] {
        let mut stack = self.accumulator_stack.borrow_mut();
        stack.init_from_board(position.board(), &self.weights);
        let acc = stack.current();

        // Forward pass from side_to_move's perspective.
        let root = position.board().side_to_move();
        let (_, mcts_values) = forward_pass(acc, &self.weights, root);

        // Override eliminated players to small epsilon.
        let mut result = mcts_values;
        for &p in &Player::ALL {
            if position.player_status(p) == PlayerStatus::Eliminated {
                result[p.index()] = 0.001;
            }
        }
        result
    }
}
