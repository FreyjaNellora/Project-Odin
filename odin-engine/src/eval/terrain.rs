// Terrain-aware evaluation — Stage 17
//
// Evaluates terrain piece proximity for king safety and piece positioning:
//   - King wall bonus: terrain adjacent to king provides cover (up to 2 pieces)
//   - King trap penalty: too much adjacent terrain restricts king mobility
//   - Fortress bonus: non-pawn pieces adjacent to terrain gain stability
//   - Outpost bonus: knights/bishops adjacent to terrain (can't be exchanged)

use crate::board::{file_of, is_valid_square, rank_of, square_from, Board, PieceStatus, PieceType, Player};
use crate::gamestate::PlayerStatus;

use super::EvalWeights;

/// The 8 adjacent square deltas (same as king_safety.rs).
const ADJACENT_DELTAS: [(i8, i8); 8] = [
    (-1, -1), (0, -1), (1, -1),
    (-1,  0),          (1,  0),
    (-1,  1), (0,  1), (1,  1),
];

/// Count terrain pieces adjacent to a given square.
fn count_adjacent_terrain(board: &Board, sq: crate::board::Square) -> u8 {
    let f = file_of(sq) as i8;
    let r = rank_of(sq) as i8;
    let mut count = 0u8;

    for &(df, dr) in &ADJACENT_DELTAS {
        let nf = f + df;
        let nr = r + dr;
        if !(0..=13).contains(&nf) || !(0..=13).contains(&nr) {
            continue;
        }
        if let Some(nsq) = square_from(nf as u8, nr as u8) {
            if !is_valid_square(nsq) {
                continue;
            }
            if let Some(piece) = board.piece_at(nsq) {
                if piece.status == PieceStatus::Terrain {
                    count += 1;
                }
            }
        }
    }

    count
}

/// Terrain-aware evaluation for a player. Only called when terrain mode is active.
///
/// Returns bonus/penalty in centipawns.
pub(crate) fn terrain_eval(
    board: &Board,
    player: Player,
    statuses: &[PlayerStatus; 4],
    weights: &EvalWeights,
) -> i16 {
    if statuses[player.index()] == PlayerStatus::Eliminated {
        return 0;
    }

    let mut score: i16 = 0;

    // --- King terrain proximity ---
    let king_sq = board.king_square(player);
    let king_terrain = count_adjacent_terrain(board, king_sq);

    if king_terrain >= 3 {
        // Too much terrain = trapped king
        score = score.saturating_sub(weights.terrain_king_trap_penalty);
    } else if king_terrain >= 1 {
        // 1-2 adjacent terrain = wall bonus
        let wall_bonus = (king_terrain as i16) * weights.terrain_king_wall_bonus;
        score = score.saturating_add(wall_bonus);
    }

    // --- Piece fortress and outpost bonuses ---
    let pieces = board.piece_list(player);
    for &(pt, sq) in pieces {
        if pt == PieceType::Pawn || pt == PieceType::King {
            continue;
        }

        let adj_terrain = count_adjacent_terrain(board, sq);
        if adj_terrain == 0 {
            continue;
        }

        // Fortress bonus for any piece adjacent to terrain
        score = score.saturating_add(weights.terrain_fortress_bonus);

        // Extra outpost bonus for knights and bishops
        if pt == PieceType::Knight || pt == PieceType::Bishop {
            score = score.saturating_add(10); // Fixed 10cp outpost bonus
        }
    }

    score
}
