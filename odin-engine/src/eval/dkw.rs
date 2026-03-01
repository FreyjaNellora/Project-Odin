// DKW (Dead King Walking) proximity penalty — Stage 17
//
// Penalizes positions where a Dead King Walking is near our king.
// DKW kings make random moves and can disrupt our position.

use crate::board::{file_of, rank_of, Board, PieceStatus, Player};
use crate::gamestate::PlayerStatus;

use super::EvalWeights;

/// Compute the DKW proximity penalty for a player.
///
/// For each opponent whose pieces are Dead (DKW status), compute the Manhattan
/// distance from their king to our king. If within 3 squares, apply a penalty.
/// Dead kings make random moves and can accidentally capture our pieces.
pub(crate) fn dkw_proximity_penalty(
    board: &Board,
    player: Player,
    statuses: &[PlayerStatus; 4],
    weights: &EvalWeights,
) -> i16 {
    let our_king = board.king_square(player);
    let our_file = file_of(our_king) as i16;
    let our_rank = rank_of(our_king) as i16;

    let mut total_penalty: i16 = 0;

    for &opp in &Player::ALL {
        if opp == player {
            continue;
        }
        if statuses[opp.index()] == PlayerStatus::Eliminated {
            continue;
        }

        // Check if opponent's pieces are dead (DKW)
        let opp_king_sq = board.king_square(opp);
        let opp_piece = board.piece_at(opp_king_sq);
        let is_dead = opp_piece
            .map(|p| p.status == PieceStatus::Dead)
            .unwrap_or(false);

        if !is_dead {
            continue;
        }

        let opp_file = file_of(opp_king_sq) as i16;
        let opp_rank = rank_of(opp_king_sq) as i16;
        let manhattan = (our_file - opp_file).abs() + (our_rank - opp_rank).abs();

        if manhattan <= 3 {
            total_penalty = total_penalty.saturating_add(weights.dkw_proximity_penalty);
        }
    }

    total_penalty
}
