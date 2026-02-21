// Multi-player relative evaluation components.
//
// Unique to 4-player chess: considers relative standing among all players.
//   - Lead penalty: being the strongest player attracts aggression.
//   - Threat penalty: count of distinct opponents threatening the king.
//   - FFA points integration: convert game points to centipawn contribution.

use crate::board::{Board, Player, PLAYER_COUNT};
use crate::gamestate::PlayerStatus;
use crate::movegen::is_square_attacked_by;

/// Centipawns per FFA point. 1 FFA point = 50cp in eval.
const FFA_POINT_WEIGHT: i16 = 50;

/// Lead penalty scaling: penalty = lead_amount / LEAD_PENALTY_DIVISOR.
const LEAD_PENALTY_DIVISOR: i16 = 4;

/// Maximum lead penalty in centipawns.
const MAX_LEAD_PENALTY: i16 = 150;

/// Penalty per distinct opponent threatening the king.
const THREAT_PENALTY_PER_OPPONENT: i16 = 30;

/// Lead penalty: if this player has the highest combined strength
/// (material + FFA_POINT_WEIGHT * ffa_score) among active players,
/// a penalty proportional to the lead is applied.
///
/// Returns 0 if not leading, negative if leading.
pub(crate) fn lead_penalty(
    player: Player,
    material_scores: &[i16; PLAYER_COUNT],
    ffa_scores: &[i32; PLAYER_COUNT],
    player_statuses: &[PlayerStatus; PLAYER_COUNT],
) -> i16 {
    let my_strength =
        combined_strength(material_scores[player.index()], ffa_scores[player.index()]);

    let mut max_opponent_strength = i32::MIN;
    for &opp in &Player::ALL {
        if opp == player {
            continue;
        }
        if player_statuses[opp.index()] == PlayerStatus::Eliminated {
            continue;
        }
        let opp_strength = combined_strength(material_scores[opp.index()], ffa_scores[opp.index()]);
        if opp_strength > max_opponent_strength {
            max_opponent_strength = opp_strength;
        }
    }

    // If no active opponents, no penalty.
    if max_opponent_strength == i32::MIN {
        return 0;
    }

    let lead = my_strength.saturating_sub(max_opponent_strength);
    if lead <= 0 {
        return 0;
    }

    // Penalty = lead / LEAD_PENALTY_DIVISOR, capped at MAX_LEAD_PENALTY.
    let penalty = (lead / i32::from(LEAD_PENALTY_DIVISOR)).min(i32::from(MAX_LEAD_PENALTY));
    -(penalty as i16)
}

/// Combined strength metric: material (cp) + FFA score converted to cp.
fn combined_strength(material_cp: i16, ffa_score: i32) -> i32 {
    i32::from(material_cp) + ffa_score * i32::from(FFA_POINT_WEIGHT)
}

/// Threat penalty: count distinct active opponents whose pieces attack the king.
/// Uses `is_square_attacked_by` (allocation-free).
///
/// Returns a positive value (the penalty amount to be subtracted).
pub(crate) fn threat_penalty(
    board: &Board,
    player: Player,
    player_statuses: &[PlayerStatus; PLAYER_COUNT],
) -> i16 {
    let king_sq = board.king_square(player);
    let mut threatening_opponents: i16 = 0;

    for &opp in &Player::ALL {
        if opp == player {
            continue;
        }
        if player_statuses[opp.index()] == PlayerStatus::Eliminated {
            continue;
        }
        if is_square_attacked_by(king_sq, opp, board) {
            threatening_opponents += 1;
        }
    }

    threatening_opponents * THREAT_PENALTY_PER_OPPONENT
}

/// Convert FFA game points to centipawn evaluation contribution.
/// Each FFA point = FFA_POINT_WEIGHT centipawns.
pub(crate) fn ffa_points_eval(ffa_score: i32) -> i16 {
    let weighted = ffa_score as i64 * FFA_POINT_WEIGHT as i64;
    weighted.clamp(i16::MIN as i64, i16::MAX as i64) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lead_penalty_not_leading() {
        let materials = [4300, 4300, 4300, 4300];
        let ffa = [0, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(Player::Red, &materials, &ffa, &statuses);
        assert_eq!(penalty, 0, "No penalty when not leading");
    }

    #[test]
    fn test_lead_penalty_when_leading() {
        // Red has 500cp more material than everyone else.
        let materials = [4800, 4300, 4300, 4300];
        let ffa = [0, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(Player::Red, &materials, &ffa, &statuses);
        // Lead = 500, penalty = -500/4 = -125.
        assert_eq!(penalty, -125);
    }

    #[test]
    fn test_lead_penalty_capped() {
        // Massive lead should be capped.
        let materials = [8000, 4000, 4000, 4000];
        let ffa = [0, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(Player::Red, &materials, &ffa, &statuses);
        assert_eq!(penalty, -MAX_LEAD_PENALTY);
    }

    #[test]
    fn test_lead_penalty_includes_ffa() {
        // Red has same material but 10 more FFA points.
        let materials = [4300, 4300, 4300, 4300];
        let ffa = [10, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(Player::Red, &materials, &ffa, &statuses);
        // Lead = 10 * 50 = 500cp, penalty = -500/4 = -125.
        assert_eq!(penalty, -125);
    }

    #[test]
    fn test_lead_penalty_eliminated_ignored() {
        // All opponents eliminated — no penalty.
        let materials = [4300, 4300, 4300, 4300];
        let ffa = [0, 0, 0, 0];
        let statuses = [
            PlayerStatus::Active,
            PlayerStatus::Eliminated,
            PlayerStatus::Eliminated,
            PlayerStatus::Eliminated,
        ];
        let penalty = lead_penalty(Player::Red, &materials, &ffa, &statuses);
        assert_eq!(penalty, 0);
    }

    #[test]
    fn test_ffa_points_eval_positive() {
        assert_eq!(ffa_points_eval(10), 500);
    }

    #[test]
    fn test_ffa_points_eval_zero() {
        assert_eq!(ffa_points_eval(0), 0);
    }

    #[test]
    fn test_ffa_points_eval_clamped() {
        // Very large FFA score should clamp to i16 bounds.
        let result = ffa_points_eval(1_000_000);
        assert_eq!(result, i16::MAX);
    }

    #[test]
    fn test_threat_penalty_starting_position() {
        let board = crate::board::Board::starting_position();
        let statuses = [PlayerStatus::Active; 4];
        // At start, no opponent attacks any king directly.
        for &player in &Player::ALL {
            let penalty = threat_penalty(&board, player, &statuses);
            assert_eq!(penalty, 0, "No threats at starting position for {player:?}");
        }
    }
}
