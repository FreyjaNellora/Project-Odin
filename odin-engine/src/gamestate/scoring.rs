// FFA scoring system for four-player chess.
//
// Point values per chess.com rules:
//   Pawn=1, Knight=3, Bishop=5, Rook=5, Queen=9, PromotedQueen=1
//   Checkmate=20, Stalemate=20 (to stalemated player), Draw=10
//   Double check bonus=1, Triple check bonus=5
//   Dead/Terrain pieces=0 on capture

use crate::board::{PieceStatus, PieceType};

// Capture point values
pub const CAPTURE_PAWN: i32 = 1;
pub const CAPTURE_KNIGHT: i32 = 3;
pub const CAPTURE_BISHOP: i32 = 5;
pub const CAPTURE_ROOK: i32 = 5;
pub const CAPTURE_QUEEN: i32 = 9;
pub const CAPTURE_PROMOTED_QUEEN: i32 = 1;
pub const CAPTURE_KING: i32 = 0; // Kings aren't captured for points

// Event point values
pub const CHECKMATE_POINTS: i32 = 20;
pub const STALEMATE_POINTS: i32 = 20;
pub const DRAW_POINTS: i32 = 10;
pub const DOUBLE_CHECK_BONUS: i32 = 1;
pub const TRIPLE_CHECK_BONUS: i32 = 5;
pub const CLAIM_WIN_LEAD: i32 = 21;

/// Points awarded for capturing a piece of the given type.
/// Dead and Terrain pieces are worth 0.
pub fn capture_points(piece_type: PieceType, status: PieceStatus) -> i32 {
    if status != PieceStatus::Alive {
        return 0;
    }
    match piece_type {
        PieceType::Pawn => CAPTURE_PAWN,
        PieceType::Knight => CAPTURE_KNIGHT,
        PieceType::Bishop => CAPTURE_BISHOP,
        PieceType::Rook => CAPTURE_ROOK,
        PieceType::Queen => CAPTURE_QUEEN,
        PieceType::PromotedQueen => CAPTURE_PROMOTED_QUEEN,
        PieceType::King => CAPTURE_KING,
    }
}

/// Bonus points for checking multiple kings with one move.
/// 1 king checked = 0 bonus (normal), 2 = +1, 3 = +5.
pub fn check_bonus_points(kings_checked: usize) -> i32 {
    match kings_checked {
        0 | 1 => 0,
        2 => DOUBLE_CHECK_BONUS,
        _ => TRIPLE_CHECK_BONUS, // 3 is max in 4PC
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_points_alive() {
        assert_eq!(capture_points(PieceType::Pawn, PieceStatus::Alive), 1);
        assert_eq!(capture_points(PieceType::Knight, PieceStatus::Alive), 3);
        assert_eq!(capture_points(PieceType::Bishop, PieceStatus::Alive), 5);
        assert_eq!(capture_points(PieceType::Rook, PieceStatus::Alive), 5);
        assert_eq!(capture_points(PieceType::Queen, PieceStatus::Alive), 9);
        assert_eq!(
            capture_points(PieceType::PromotedQueen, PieceStatus::Alive),
            1
        );
        assert_eq!(capture_points(PieceType::King, PieceStatus::Alive), 0);
    }

    #[test]
    fn test_capture_points_dead() {
        assert_eq!(capture_points(PieceType::Queen, PieceStatus::Dead), 0);
        assert_eq!(capture_points(PieceType::Rook, PieceStatus::Dead), 0);
        assert_eq!(capture_points(PieceType::Pawn, PieceStatus::Dead), 0);
    }

    #[test]
    fn test_capture_points_terrain() {
        assert_eq!(capture_points(PieceType::Bishop, PieceStatus::Terrain), 0);
    }

    #[test]
    fn test_check_bonus() {
        assert_eq!(check_bonus_points(0), 0);
        assert_eq!(check_bonus_points(1), 0);
        assert_eq!(check_bonus_points(2), 1);
        assert_eq!(check_bonus_points(3), 5);
    }
}
