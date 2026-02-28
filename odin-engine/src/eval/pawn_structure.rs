// Pawn structure evaluation — connected pawns.
//
// Connected pawns: +8cp per pawn defended by another friendly pawn.
// A pawn is "connected" if a same-color pawn is on a square that could
// capture to the pawn's square (i.e., diagonally behind in that player's
// pawn advance direction).
//
// Defender squares per player (pawn at file f, rank r):
//   Red    (fwd +rank): (f-1, r-1), (f+1, r-1)
//   Blue   (fwd +file): (f-1, r-1), (f-1, r+1)
//   Yellow (fwd -rank): (f-1, r+1), (f+1, r+1)
//   Green  (fwd -file): (f+1, r-1), (f+1, r+1)

use crate::board::{file_of, rank_of, square_from, Board, PieceStatus, PieceType, Player};

const CONNECTED_PAWN_BONUS: i16 = 8;

/// Connected pawn score for a player. +8cp per pawn defended by a friendly pawn.
pub(crate) fn connected_pawn_score(board: &Board, player: Player) -> i16 {
    let mut score: i16 = 0;

    for &(pt, sq) in board.piece_list(player) {
        if pt != PieceType::Pawn {
            continue;
        }

        // Only count alive pawns.
        if let Some(piece) = board.piece_at(sq) {
            if piece.status != PieceStatus::Alive {
                continue;
            }
        } else {
            continue;
        }

        let f = file_of(sq);
        let r = rank_of(sq);

        // Only award connected bonus for pawns 2+ ranks past starting rank.
        // Single-step pushes (f2f3) create instant chains that reward passivity.
        if !is_sufficiently_advanced(player, f, r) {
            continue;
        }

        // Check both potential defender squares.
        let defenders = defender_squares(player, f, r);
        for def_sq in defenders.into_iter().flatten() {
            if let Some(piece) = board.piece_at(def_sq) {
                if piece.piece_type == PieceType::Pawn
                    && piece.owner == player
                    && piece.status == PieceStatus::Alive
                {
                    score = score.saturating_add(CONNECTED_PAWN_BONUS);
                    break; // Count each pawn as connected at most once
                }
            }
        }
    }

    score
}

/// Check if a pawn has advanced at least 2 ranks from its starting rank.
/// Red starts rank 1, Blue file 1, Yellow rank 12, Green file 12.
fn is_sufficiently_advanced(player: Player, f: u8, r: u8) -> bool {
    match player {
        Player::Red => r >= 3,    // start rank 1, need rank 3+
        Player::Blue => f >= 3,   // start file 1, need file 3+
        Player::Yellow => r <= 10, // start rank 12, need rank 10-
        Player::Green => f <= 10,  // start file 12, need file 10-
    }
}

/// Return up to 2 potential defender squares for a pawn of the given player.
/// A "defender" is a friendly pawn one diagonal step behind (in the pawn's
/// advance direction) that could capture to this pawn's square.
fn defender_squares(player: Player, f: u8, r: u8) -> [Option<u8>; 2] {
    match player {
        // Red pawns advance +rank; defenders are at rank-1, file±1
        Player::Red => [
            if r > 0 && f > 0 {
                square_from(f - 1, r - 1)
            } else {
                None
            },
            if r > 0 && f < 13 {
                square_from(f + 1, r - 1)
            } else {
                None
            },
        ],
        // Blue pawns advance +file; defenders are at file-1, rank±1
        Player::Blue => [
            if f > 0 && r > 0 {
                square_from(f - 1, r - 1)
            } else {
                None
            },
            if f > 0 && r < 13 {
                square_from(f - 1, r + 1)
            } else {
                None
            },
        ],
        // Yellow pawns advance -rank; defenders are at rank+1, file±1
        Player::Yellow => [
            if r < 13 && f > 0 {
                square_from(f - 1, r + 1)
            } else {
                None
            },
            if r < 13 && f < 13 {
                square_from(f + 1, r + 1)
            } else {
                None
            },
        ],
        // Green pawns advance -file; defenders are at file+1, rank±1
        Player::Green => [
            if f < 13 && r > 0 {
                square_from(f + 1, r - 1)
            } else {
                None
            },
            if f < 13 && r < 13 {
                square_from(f + 1, r + 1)
            } else {
                None
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_starting_position_no_connected_pawns() {
        let board = Board::starting_position();
        // In the starting position, all pawns are on the same rank (2nd rank).
        // No pawn is diagonally behind another, so connected score = 0.
        for &player in &Player::ALL {
            let score = connected_pawn_score(&board, player);
            assert_eq!(
                score, 0,
                "Starting position should have 0 connected pawns for {player:?}, got {score}"
            );
        }
    }

    #[test]
    fn test_connected_pawn_bonus_value() {
        assert_eq!(CONNECTED_PAWN_BONUS, 8);
    }

    #[test]
    fn test_defender_squares_red() {
        // Red pawn at e5 (file 4, rank 4): defenders at d4 and f4
        let defs = defender_squares(Player::Red, 4, 4);
        assert_eq!(defs[0], square_from(3, 3)); // d4
        assert_eq!(defs[1], square_from(5, 3)); // f4
    }

    #[test]
    fn test_defender_squares_blue() {
        // Blue pawn at (file 4, rank 6): defenders at (3,5) and (3,7)
        let defs = defender_squares(Player::Blue, 4, 6);
        assert_eq!(defs[0], square_from(3, 5));
        assert_eq!(defs[1], square_from(3, 7));
    }

    #[test]
    fn test_defender_squares_yellow() {
        // Yellow pawn at (file 4, rank 10): defenders at (3,11) and (5,11)
        let defs = defender_squares(Player::Yellow, 4, 10);
        assert_eq!(defs[0], square_from(3, 11));
        assert_eq!(defs[1], square_from(5, 11));
    }

    #[test]
    fn test_defender_squares_green() {
        // Green pawn at (file 10, rank 6): defenders at (11,5) and (11,7)
        let defs = defender_squares(Player::Green, 10, 6);
        assert_eq!(defs[0], square_from(11, 5));
        assert_eq!(defs[1], square_from(11, 7));
    }

    #[test]
    fn test_edge_pawn_no_wrap() {
        // Red pawn at file 0, rank 2: left defender would be file -1 (None)
        let defs = defender_squares(Player::Red, 0, 2);
        assert_eq!(defs[0], None);
        assert_eq!(defs[1], square_from(1, 1));
    }

    #[test]
    fn test_symmetry_all_players() {
        // All players should have the same connected pawn score at start.
        let board = Board::starting_position();
        let scores: Vec<i16> = Player::ALL
            .iter()
            .map(|&p| connected_pawn_score(&board, p))
            .collect();
        assert_eq!(scores[0], scores[1], "Red vs Blue mismatch");
        assert_eq!(scores[0], scores[2], "Red vs Yellow mismatch");
        assert_eq!(scores[0], scores[3], "Red vs Green mismatch");
    }
}
