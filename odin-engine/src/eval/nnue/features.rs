// HalfKP-4 Feature Indexing — Stage 14
//
// Per-perspective encoding: (piece_square, piece_type, relative_owner).
// 160 valid squares x 7 piece types x 4 relative owners = 4,480 features.
// Active per position: ~30-64 (very sparse).

use crate::board::{
    Board, PieceType, Player, Square, BOARD_SIZE, PIECE_TYPE_COUNT, PLAYER_COUNT, TOTAL_SQUARES,
    VALID_SQUARE_COUNT,
};

// ---------------------------------------------------------------------------
// Architecture constants
// ---------------------------------------------------------------------------

/// Relative owner count: Own(0), CW(1), Across(2), CCW(3).
pub const RELATIVE_OWNER_COUNT: usize = 4;

/// Total features per perspective: 160 * 7 * 4 = 4,480.
pub const FEATURES_PER_PERSPECTIVE: usize =
    VALID_SQUARE_COUNT * PIECE_TYPE_COUNT * RELATIVE_OWNER_COUNT;

/// Feature transformer output size (accumulator width).
pub const FT_OUT: usize = 256;

/// Hidden layer size.
pub const HIDDEN_SIZE: usize = 32;

/// BRS output count.
pub const BRS_OUTPUT: usize = 1;

/// MCTS output count.
pub const MCTS_OUTPUT: usize = 4;

/// SCReLU quantization scale.
pub const QA: i16 = 255;

/// Hidden layer quantization scale.
pub const QB: i32 = 64;

/// Maximum accumulator stack depth.
pub const MAX_STACK_DEPTH: usize = 128;

/// Output rescaling divisor (BRS head → centipawns).
pub const OUTPUT_SCALE: i32 = 400;

// ---------------------------------------------------------------------------
// Square-to-dense mapping
// ---------------------------------------------------------------------------

/// Maps raw square index (0..196) to dense index (0..159).
/// Invalid squares map to 255 (sentinel).
static SQUARE_TO_DENSE: [u8; TOTAL_SQUARES] = {
    let mut table = [255u8; TOTAL_SQUARES];
    let mut dense = 0u8;
    let mut sq = 0usize;
    while sq < TOTAL_SQUARES {
        let file = sq % BOARD_SIZE;
        let rank = sq / BOARD_SIZE;
        let in_corner = (file <= 2 && rank <= 2)
            || (file >= 11 && rank <= 2)
            || (file <= 2 && rank >= 11)
            || (file >= 11 && rank >= 11);
        if !in_corner {
            table[sq] = dense;
            dense += 1;
        }
        sq += 1;
    }
    table
};

/// Maps dense index (0..159) back to raw square index.
static DENSE_TO_SQUARE: [u8; VALID_SQUARE_COUNT] = {
    let mut table = [0u8; VALID_SQUARE_COUNT];
    let mut dense = 0usize;
    let mut sq = 0usize;
    while sq < TOTAL_SQUARES {
        let file = sq % BOARD_SIZE;
        let rank = sq / BOARD_SIZE;
        let in_corner = (file <= 2 && rank <= 2)
            || (file >= 11 && rank <= 2)
            || (file <= 2 && rank >= 11)
            || (file >= 11 && rank >= 11);
        if !in_corner {
            table[dense] = sq as u8;
            dense += 1;
        }
        sq += 1;
    }
    table
};

/// Convert raw square (0..195) to dense feature index (0..159).
/// Returns 255 for invalid squares.
#[inline]
pub fn square_to_dense(sq: Square) -> u8 {
    SQUARE_TO_DENSE[sq as usize]
}

/// Convert dense index (0..159) to raw square (0..195).
#[inline]
pub fn dense_to_square(dense: u8) -> Square {
    debug_assert!((dense as usize) < VALID_SQUARE_COUNT);
    DENSE_TO_SQUARE[dense as usize]
}

// ---------------------------------------------------------------------------
// Relative owner mapping
// ---------------------------------------------------------------------------

/// Compute relative owner index given perspective and piece owner.
///
/// Turn order: R→B→Y→G (ADR-012).
///   0 = Own (perspective player owns this piece)
///   1 = CW-opponent (next in turn order)
///   2 = Across-opponent (opposite)
///   3 = CCW-opponent (previous in turn order)
#[inline]
pub fn relative_owner(perspective: Player, piece_owner: Player) -> u8 {
    ((piece_owner.index() + PLAYER_COUNT - perspective.index()) % PLAYER_COUNT) as u8
}

// ---------------------------------------------------------------------------
// Feature index computation
// ---------------------------------------------------------------------------

/// Compute the feature index for a piece on a given square.
///
/// Formula: `square_dense * 28 + piece_type * 4 + relative_owner`
///
/// Returns `None` if the square is invalid (corner).
#[inline]
pub fn feature_index(sq: Square, piece_type: PieceType, rel_owner: u8) -> Option<u16> {
    let dense = SQUARE_TO_DENSE[sq as usize];
    if dense == 255 {
        return None;
    }
    Some(dense as u16 * 28 + piece_type.index() as u16 * 4 + rel_owner as u16)
}

/// Collect all active feature indices for a perspective from a board.
///
/// Iterates every player's piece list and maps each piece to its feature index
/// relative to the given perspective. Returns a fixed-capacity array + count.
///
/// Typical count: ~30-64 (4 players × ~8-16 pieces each).
pub fn active_features(board: &Board, perspective: Player) -> ([u16; 64], usize) {
    let mut features = [0u16; 64];
    let mut count = 0;
    for &player in &Player::ALL {
        let piece_list = board.piece_list(player);
        for &(pt, sq) in piece_list {
            let rel = relative_owner(perspective, player);
            if let Some(idx) = feature_index(sq, pt, rel) {
                debug_assert!(count < 64, "too many active features");
                features[count] = idx;
                count += 1;
            }
        }
    }
    (features, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{is_valid_square, valid_squares};
    use std::collections::HashSet;

    #[test]
    fn test_square_to_dense_count() {
        let count = (0..TOTAL_SQUARES as u8)
            .filter(|&sq| SQUARE_TO_DENSE[sq as usize] != 255)
            .count();
        assert_eq!(count, VALID_SQUARE_COUNT);
    }

    #[test]
    fn test_square_to_dense_range() {
        let mut seen = HashSet::new();
        for sq in valid_squares() {
            let dense = square_to_dense(sq);
            assert!(
                (dense as usize) < VALID_SQUARE_COUNT,
                "dense index {dense} out of range for sq {sq}"
            );
            assert!(seen.insert(dense), "duplicate dense index {dense}");
        }
        assert_eq!(seen.len(), VALID_SQUARE_COUNT);
    }

    #[test]
    fn test_dense_to_square_roundtrip() {
        for sq in valid_squares() {
            let dense = square_to_dense(sq);
            let back = dense_to_square(dense);
            assert_eq!(back, sq);
        }
    }

    #[test]
    fn test_invalid_squares_get_sentinel() {
        for sq in 0..TOTAL_SQUARES as u8 {
            if !is_valid_square(sq) {
                assert_eq!(
                    SQUARE_TO_DENSE[sq as usize],
                    255,
                    "invalid sq {sq} should map to 255"
                );
            }
        }
    }

    #[test]
    fn test_relative_owner_self() {
        for &p in &Player::ALL {
            assert_eq!(relative_owner(p, p), 0, "own piece should be 0");
        }
    }

    #[test]
    fn test_relative_owner_rotation() {
        // From Red's perspective: Red=0, Blue=1, Yellow=2, Green=3
        assert_eq!(relative_owner(Player::Red, Player::Red), 0);
        assert_eq!(relative_owner(Player::Red, Player::Blue), 1);
        assert_eq!(relative_owner(Player::Red, Player::Yellow), 2);
        assert_eq!(relative_owner(Player::Red, Player::Green), 3);

        // From Blue's perspective: Blue=0, Yellow=1, Green=2, Red=3
        assert_eq!(relative_owner(Player::Blue, Player::Blue), 0);
        assert_eq!(relative_owner(Player::Blue, Player::Yellow), 1);
        assert_eq!(relative_owner(Player::Blue, Player::Green), 2);
        assert_eq!(relative_owner(Player::Blue, Player::Red), 3);
    }

    #[test]
    fn test_feature_index_range() {
        let mut seen = HashSet::new();
        for sq in valid_squares() {
            for &pt in &PieceType::ALL {
                for rel in 0..RELATIVE_OWNER_COUNT as u8 {
                    let idx = feature_index(sq, pt, rel).unwrap();
                    assert!(
                        (idx as usize) < FEATURES_PER_PERSPECTIVE,
                        "index {idx} out of range for sq={sq} pt={pt:?} rel={rel}"
                    );
                    assert!(
                        seen.insert(idx),
                        "duplicate index {idx} for sq={sq} pt={pt:?} rel={rel}"
                    );
                }
            }
        }
        assert_eq!(seen.len(), FEATURES_PER_PERSPECTIVE);
    }

    #[test]
    fn test_feature_index_invalid_square() {
        // a1 = square_from(0, 0) = 0, which is invalid
        assert!(feature_index(0, PieceType::Pawn, 0).is_none());
    }
}
