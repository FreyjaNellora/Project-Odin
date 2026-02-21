// Eval piece value constants — centipawn values for search evaluation.
//
// These are DIFFERENT from gamestate::scoring capture point values:
//   Scoring (FFA rules): Pawn=1pt, Knight=3pt, Bishop=5pt, Rook=5pt, Queen=9pt, PromotedQueen=1pt
//   Eval (search):       Pawn=100cp, Knight=300cp, Bishop=500cp, Rook=500cp, Queen=900cp, PromotedQueen=900cp
//
// The eval values drive search heuristics. The scoring values drive FFA game points.

use crate::board::PIECE_TYPE_COUNT;

/// Pawn evaluation value in centipawns.
pub const PAWN_EVAL_VALUE: i16 = 100;

/// Knight evaluation value in centipawns.
pub const KNIGHT_EVAL_VALUE: i16 = 300;

/// Bishop evaluation value in centipawns.
/// Equal to rook on the larger 14x14 board (more diagonal scope).
pub const BISHOP_EVAL_VALUE: i16 = 500;

/// Rook evaluation value in centipawns.
pub const ROOK_EVAL_VALUE: i16 = 500;

/// Queen evaluation value in centipawns.
pub const QUEEN_EVAL_VALUE: i16 = 900;

/// King evaluation value — not counted in material.
pub const KING_EVAL_VALUE: i16 = 0;

/// Promoted queen evaluation value in centipawns.
/// Moves as queen (900cp in search) but worth only 1 point on FFA capture.
pub const PROMOTED_QUEEN_EVAL_VALUE: i16 = 900;

/// Eval values indexed by `PieceType::index()`.
///   0=Pawn, 1=Knight, 2=Bishop, 3=Rook, 4=Queen, 5=King, 6=PromotedQueen
pub const PIECE_EVAL_VALUES: [i16; PIECE_TYPE_COUNT] = [
    PAWN_EVAL_VALUE,           // 0: Pawn
    KNIGHT_EVAL_VALUE,         // 1: Knight
    BISHOP_EVAL_VALUE,         // 2: Bishop
    ROOK_EVAL_VALUE,           // 3: Rook
    QUEEN_EVAL_VALUE,          // 4: Queen
    KING_EVAL_VALUE,           // 5: King
    PROMOTED_QUEEN_EVAL_VALUE, // 6: PromotedQueen
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::PieceType;

    #[test]
    fn test_piece_eval_values_match_constants() {
        assert_eq!(PIECE_EVAL_VALUES[PieceType::Pawn.index()], PAWN_EVAL_VALUE);
        assert_eq!(
            PIECE_EVAL_VALUES[PieceType::Knight.index()],
            KNIGHT_EVAL_VALUE
        );
        assert_eq!(
            PIECE_EVAL_VALUES[PieceType::Bishop.index()],
            BISHOP_EVAL_VALUE
        );
        assert_eq!(PIECE_EVAL_VALUES[PieceType::Rook.index()], ROOK_EVAL_VALUE);
        assert_eq!(
            PIECE_EVAL_VALUES[PieceType::Queen.index()],
            QUEEN_EVAL_VALUE
        );
        assert_eq!(PIECE_EVAL_VALUES[PieceType::King.index()], KING_EVAL_VALUE);
        assert_eq!(
            PIECE_EVAL_VALUES[PieceType::PromotedQueen.index()],
            PROMOTED_QUEEN_EVAL_VALUE
        );
    }

    #[test]
    fn test_promoted_queen_eval_equals_queen() {
        assert_eq!(PROMOTED_QUEEN_EVAL_VALUE, QUEEN_EVAL_VALUE);
    }

    #[test]
    fn test_king_eval_is_zero() {
        assert_eq!(KING_EVAL_VALUE, 0);
    }

    #[test]
    fn test_piece_ordering() {
        assert!(PAWN_EVAL_VALUE < KNIGHT_EVAL_VALUE);
        assert!(KNIGHT_EVAL_VALUE < BISHOP_EVAL_VALUE);
        assert!(BISHOP_EVAL_VALUE <= ROOK_EVAL_VALUE);
        assert!(ROOK_EVAL_VALUE < QUEEN_EVAL_VALUE);
    }
}
