// Development bonus — reward pieces that have moved off their back rank.
//
// In 4-player chess, the engine needs explicit incentive to develop pieces
// rather than push pawns. Without this, the eval is blind to the difference
// between a developed and undeveloped position.
//
// Per piece off back rank (tuned from 4 high-Elo human games, 2400-3500 Elo):
//   Queen:  +35cp  (primary weapon, deployed move 2-5 by winners, loss = catastrophe)
//   Knight: +25cp  (consistent first-movers, avg move 5.6 for winners)
//   Rook:   +15cp  (important when activated, avg move 13.4, slight bump)
//   Bishop: +15cp  (situational "snipers", avg move 13.4, wait for open lines)
//
// Back rank per player:
//   Red:    rank 0   (south side)
//   Blue:   file 0   (west side)
//   Yellow: rank 13  (north side)
//   Green:  file 13  (east side)

use crate::board::{file_of, rank_of, Board, PieceStatus, PieceType, Player};

const KNIGHT_DEV_BONUS: i16 = 45;
const BISHOP_DEV_BONUS: i16 = 30;
const QUEEN_DEV_BONUS: i16 = 50;
const ROOK_DEV_BONUS: i16 = 25;

/// Development score for a player. Positive = more developed.
pub(crate) fn development_score(board: &Board, player: Player) -> i16 {
    let mut score: i16 = 0;

    for &(pt, sq) in board.piece_list(player) {
        let bonus = match pt {
            PieceType::Knight => KNIGHT_DEV_BONUS,
            PieceType::Bishop => BISHOP_DEV_BONUS,
            PieceType::Queen => QUEEN_DEV_BONUS,
            PieceType::Rook => ROOK_DEV_BONUS,
            _ => continue,
        };

        // Only count alive pieces.
        if let Some(piece) = board.piece_at(sq) {
            if piece.status != PieceStatus::Alive {
                continue;
            }
        } else {
            continue;
        }

        if !is_on_back_rank(player, sq) {
            score = score.saturating_add(bonus);
        }
    }

    score
}

/// Check if a square is on the player's back rank.
fn is_on_back_rank(player: Player, sq: u8) -> bool {
    match player {
        Player::Red => rank_of(sq) == 0,
        Player::Blue => file_of(sq) == 0,
        Player::Yellow => rank_of(sq) == 13,
        Player::Green => file_of(sq) == 13,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_starting_position_zero_development() {
        let board = Board::starting_position();
        // All pieces on back rank at start — development score should be 0.
        for &player in &Player::ALL {
            let score = development_score(&board, player);
            assert_eq!(
                score, 0,
                "Starting position should have 0 development for {player:?}, got {score}"
            );
        }
    }

    #[test]
    fn test_max_development_value() {
        // Full development: 2 knights (90) + 2 bishops (60) + 1 queen (50) + 2 rooks (50) = 250cp
        let max = 2 * KNIGHT_DEV_BONUS + 2 * BISHOP_DEV_BONUS + QUEEN_DEV_BONUS + 2 * ROOK_DEV_BONUS;
        assert_eq!(max, 250);
    }

    #[test]
    fn test_back_rank_detection() {
        // Red back rank = rank 0
        assert!(is_on_back_rank(Player::Red, 3)); // rank 0, file 3
        assert!(!is_on_back_rank(Player::Red, 17)); // rank 1, file 3

        // Blue back rank = file 0
        assert!(is_on_back_rank(Player::Blue, 42)); // rank 3, file 0
        assert!(!is_on_back_rank(Player::Blue, 43)); // rank 3, file 1

        // Yellow back rank = rank 13
        assert!(is_on_back_rank(Player::Yellow, 13 * 14 + 3)); // rank 13, file 3
        assert!(!is_on_back_rank(Player::Yellow, 12 * 14 + 3)); // rank 12, file 3

        // Green back rank = file 13
        assert!(is_on_back_rank(Player::Green, 3 * 14 + 13)); // rank 3, file 13
        assert!(!is_on_back_rank(Player::Green, 3 * 14 + 12)); // rank 3, file 12
    }
}
