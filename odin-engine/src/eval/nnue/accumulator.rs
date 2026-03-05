// NNUE Accumulator — Stage 14
//
// Per-perspective accumulators (4 × 256 int16), stack for make/unmake,
// incremental delta updates and full refresh.

use crate::board::{Board, PieceType, Player};
use crate::movegen::Move;

use super::features::{
    active_features, feature_index, relative_owner, FEATURES_PER_PERSPECTIVE, FT_OUT,
    MAX_STACK_DEPTH,
};
use super::weights::NnueWeights;

// ---------------------------------------------------------------------------
// Accumulator
// ---------------------------------------------------------------------------

/// Full accumulator: 4 perspectives × 256 int16 values.
#[derive(Clone)]
pub struct Accumulator {
    /// Per-perspective accumulator values.
    /// Index: `values[player.index()][neuron]`.
    pub values: [[i16; FT_OUT]; 4],

    /// Whether each perspective needs full recompute.
    pub needs_refresh: [bool; 4],
}

impl Accumulator {
    /// Create a zeroed accumulator.
    pub fn zeroed() -> Self {
        Self {
            values: [[0i16; FT_OUT]; 4],
            needs_refresh: [true; 4],
        }
    }

    /// Full recompute from scratch for all perspectives.
    pub fn compute_full(&mut self, board: &Board, weights: &NnueWeights) {
        for &perspective in &Player::ALL {
            self.compute_perspective(perspective, board, weights);
        }
    }

    /// Full recompute for a single perspective.
    pub fn compute_perspective(
        &mut self,
        perspective: Player,
        board: &Board,
        weights: &NnueWeights,
    ) {
        let pidx = perspective.index();
        let bias_offset = pidx * FT_OUT;

        // Start from biases (perspective-specific).
        self.values[pidx].copy_from_slice(&weights.ft_biases[bias_offset..bias_offset + FT_OUT]);

        // Add each active feature's weight column.
        let (features, count) = active_features(board, perspective);
        let ft_base = pidx * FEATURES_PER_PERSPECTIVE * FT_OUT;
        for &feat_raw in &features[..count] {
            let feat = feat_raw as usize;
            let col_offset = ft_base + feat * FT_OUT;
            super::simd::accumulator_add(
                &mut self.values[pidx],
                &weights.ft_weights[col_offset..col_offset + FT_OUT],
            );
        }

        self.needs_refresh[pidx] = false;
    }

    /// Incrementally add a feature (piece appeared on a square).
    #[inline]
    pub fn add_feature(
        &mut self,
        perspective: Player,
        feat_idx: u16,
        weights: &NnueWeights,
    ) {
        let pidx = perspective.index();
        let col_offset =
            pidx * FEATURES_PER_PERSPECTIVE * FT_OUT + feat_idx as usize * FT_OUT;
        super::simd::accumulator_add(
            &mut self.values[pidx],
            &weights.ft_weights[col_offset..col_offset + FT_OUT],
        );
    }

    /// Incrementally remove a feature (piece left a square).
    #[inline]
    pub fn sub_feature(
        &mut self,
        perspective: Player,
        feat_idx: u16,
        weights: &NnueWeights,
    ) {
        let pidx = perspective.index();
        let col_offset =
            pidx * FEATURES_PER_PERSPECTIVE * FT_OUT + feat_idx as usize * FT_OUT;
        super::simd::accumulator_sub(
            &mut self.values[pidx],
            &weights.ft_weights[col_offset..col_offset + FT_OUT],
        );
    }
}

// ---------------------------------------------------------------------------
// AccumulatorStack
// ---------------------------------------------------------------------------

/// Stack of accumulators mirroring the search tree.
///
/// Push copies the current accumulator forward and applies incremental updates.
/// Pop simply decrements the depth pointer (zero computation).
pub struct AccumulatorStack {
    stack: Vec<Accumulator>,
    current: usize,
}

impl Default for AccumulatorStack {
    fn default() -> Self {
        Self::new()
    }
}

impl AccumulatorStack {
    /// Create a new stack with pre-allocated entries.
    pub fn new() -> Self {
        let mut stack = Vec::with_capacity(MAX_STACK_DEPTH);
        for _ in 0..MAX_STACK_DEPTH {
            stack.push(Accumulator::zeroed());
        }
        Self { stack, current: 0 }
    }

    /// Initialize the base accumulator from a board position (full compute).
    pub fn init_from_board(&mut self, board: &Board, weights: &NnueWeights) {
        self.current = 0;
        self.stack[0].compute_full(board, weights);
    }

    /// Get the current accumulator.
    pub fn current(&self) -> &Accumulator {
        &self.stack[self.current]
    }

    /// Get the current accumulator mutably.
    pub fn current_mut(&mut self) -> &mut Accumulator {
        &mut self.stack[self.current]
    }

    /// Get the current depth.
    pub fn depth(&self) -> usize {
        self.current
    }

    /// Push: copy current accumulator forward, apply incremental update for a move.
    ///
    /// `board_before` is the board state BEFORE make_move is called.
    /// The move tells us what changed.
    pub fn push(&mut self, mv: Move, board_before: &Board, weights: &NnueWeights) {
        debug_assert!(
            self.current + 1 < MAX_STACK_DEPTH,
            "accumulator stack overflow"
        );

        let next = self.current + 1;

        // Copy current accumulator to next slot.
        self.stack[next] = self.stack[self.current].clone();

        let mover = board_before.side_to_move();
        let piece = mv.piece_type();
        let from_sq = mv.from_sq();
        let to_sq = mv.to_sq();

        // For each of the 4 perspectives, decide incremental vs refresh.
        for &perspective in &Player::ALL {
            let pidx = perspective.index();

            if self.stack[next].needs_refresh[pidx] {
                continue; // Already flagged, skip incremental.
            }

            // King move of the perspective's own king → mark refresh.
            // (Prepares for Phase 2 king bucketing; in Phase 1, could be
            // incremental but we keep the refresh infrastructure.)
            if piece == PieceType::King && mover == perspective {
                self.stack[next].needs_refresh[pidx] = true;
                continue;
            }

            // En passant: captured pawn is on a different square than to_sq.
            // Mark refresh rather than computing the EP capture square.
            if mv.is_en_passant() {
                self.stack[next].needs_refresh[pidx] = true;
                continue;
            }

            // Castling: king + rook both move. The moving player's perspective
            // already got refresh (king move above). For other perspectives,
            // mark refresh too (rook position is player-dependent, complex).
            if mv.is_castle() {
                self.stack[next].needs_refresh[pidx] = true;
                continue;
            }

            let mover_rel = relative_owner(perspective, mover);

            // Remove: piece was on from_sq.
            if let Some(old_feat) = feature_index(from_sq, piece, mover_rel) {
                self.stack[next].sub_feature(perspective, old_feat, weights);
            }

            // Add: piece is now on to_sq (possibly promoted).
            let new_piece = mv.promotion().unwrap_or(piece);
            if let Some(new_feat) = feature_index(to_sq, new_piece, mover_rel) {
                self.stack[next].add_feature(perspective, new_feat, weights);
            }

            // Capture: remove the captured piece's feature.
            if let Some(captured_type) = mv.captured() {
                // Normal capture: captured piece was on to_sq.
                // Get the captured piece's owner from the board.
                if let Some(captured_piece) = board_before.piece_at(to_sq) {
                    let cap_rel = relative_owner(perspective, captured_piece.owner);
                    if let Some(cap_feat) = feature_index(to_sq, captured_type, cap_rel) {
                        self.stack[next].sub_feature(perspective, cap_feat, weights);
                    }
                }
            }
        }

        self.current = next;
    }

    /// Pop: restore to previous depth. Zero computation (copy-on-push design).
    pub fn pop(&mut self) {
        debug_assert!(self.current > 0, "accumulator stack underflow");
        self.current -= 1;
    }

    /// Refresh any perspectives that need it on the current accumulator.
    /// Call before forward pass to ensure all accumulators are up-to-date.
    pub fn refresh_if_needed(&mut self, board: &Board, weights: &NnueWeights) {
        let acc = &mut self.stack[self.current];
        for &perspective in &Player::ALL {
            if acc.needs_refresh[perspective.index()] {
                acc.compute_perspective(perspective, board, weights);
            }
        }
    }
}
