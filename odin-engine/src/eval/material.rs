// Material counting for evaluation.
//
// Iterates each player's piece list, sums eval-value centipawns.
// Dead and Terrain pieces contribute 0cp.

use crate::board::{Board, PieceStatus, Player, PLAYER_COUNT};

use super::values::PIECE_EVAL_VALUES;

/// Material score in centipawns for a single player.
/// Only counts alive pieces; dead/terrain pieces contribute 0.
pub(crate) fn material_score(board: &Board, player: Player) -> i16 {
    let mut score: i16 = 0;
    for &(pt, sq) in board.piece_list(player) {
        if let Some(piece) = board.piece_at(sq) {
            if piece.status == PieceStatus::Alive {
                score = score.saturating_add(PIECE_EVAL_VALUES[pt.index()]);
            }
        }
    }
    score
}

/// Material scores for all four players.
pub(crate) fn material_scores(board: &Board) -> [i16; PLAYER_COUNT] {
    let mut scores = [0i16; PLAYER_COUNT];
    for &p in &Player::ALL {
        scores[p.index()] = material_score(board, p);
    }
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::eval::values::*;

    #[test]
    fn test_starting_position_all_players_equal_material() {
        let board = Board::starting_position();
        let scores = material_scores(&board);
        // All 4 players should have identical material at start.
        assert_eq!(scores[0], scores[1]);
        assert_eq!(scores[1], scores[2]);
        assert_eq!(scores[2], scores[3]);
    }

    #[test]
    fn test_starting_position_material_value() {
        let board = Board::starting_position();
        let score = material_score(&board, Player::Red);
        // Red's starting material: 8 pawns + 2 knights + 2 bishops + 2 rooks + 1 queen + 1 king
        // = 800 + 600 + 1000 + 1000 + 900 + 0 = 4300cp
        let expected = 8 * PAWN_EVAL_VALUE
            + 2 * KNIGHT_EVAL_VALUE
            + 2 * BISHOP_EVAL_VALUE
            + 2 * ROOK_EVAL_VALUE
            + QUEEN_EVAL_VALUE
            + KING_EVAL_VALUE;
        assert_eq!(score, expected);
        assert_eq!(score, 4300);
    }

    #[test]
    fn test_empty_board_zero_material() {
        let board = Board::empty();
        for &p in &Player::ALL {
            assert_eq!(material_score(&board, p), 0);
        }
    }

    #[test]
    fn test_material_after_piece_removal() {
        let mut board = Board::starting_position();
        // Remove Red's queen (Red queen is at file 6, rank 0 = square 6 in starting position)
        // Actually, let's find it from the piece list.
        let queen_sq = board
            .piece_list(Player::Red)
            .iter()
            .find(|(pt, _)| *pt == crate::board::PieceType::Queen)
            .map(|&(_, sq)| sq);

        if let Some(sq) = queen_sq {
            board.remove_piece(sq);
            let score = material_score(&board, Player::Red);
            assert_eq!(score, 4300 - QUEEN_EVAL_VALUE);
        }
    }
}
