// Multi-player relative evaluation components.
//
// Unique to 4-player chess: considers relative standing among all players.
//   - Lead penalty: being the strongest player attracts aggression.
//   - Threat penalty: count of distinct opponents threatening the king.
//   - FFA points integration: convert game points to centipawn contribution.
//
// Stage 8: lead_penalty and ffa_points_eval accept profile-driven weights
// instead of fixed module constants. Standard profile preserves original
// behavior; Aggressive profile disables lead penalty and increases FFA weight.

use crate::board::{Board, Player, PLAYER_COUNT};
use crate::gamestate::PlayerStatus;
use crate::movegen::is_square_attacked_by;

/// Default centipawns per FFA point (Standard profile).
pub(crate) const FFA_POINT_WEIGHT_DEFAULT: i16 = 50;


/// Penalty per distinct opponent threatening the king.
const THREAT_PENALTY_PER_OPPONENT: i16 = 30;

/// Lead penalty: if this player has the highest combined strength
/// (material + ffa_weight * ffa_score) among active players,
/// a penalty proportional to the lead is applied.
///
/// Returns 0 if not leading, disabled, or no active opponents.
/// Returns negative when leading (penalty subtracted from score).
pub(crate) fn lead_penalty(
    player: Player,
    material_scores: &[i16; PLAYER_COUNT],
    ffa_scores: &[i32; PLAYER_COUNT],
    player_statuses: &[PlayerStatus; PLAYER_COUNT],
    enabled: bool,
    divisor: i16,
    max_penalty: i16,
) -> i16 {
    if !enabled {
        return 0;
    }

    let ffa_weight = FFA_POINT_WEIGHT_DEFAULT;
    let my_strength =
        combined_strength(material_scores[player.index()], ffa_scores[player.index()], ffa_weight);

    let mut max_opponent_strength = i32::MIN;
    for &opp in &Player::ALL {
        if opp == player {
            continue;
        }
        if player_statuses[opp.index()] == PlayerStatus::Eliminated {
            continue;
        }
        let opp_strength = combined_strength(
            material_scores[opp.index()],
            ffa_scores[opp.index()],
            ffa_weight,
        );
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

    // Penalty = lead / divisor, capped at max_penalty.
    let penalty = (lead / i32::from(divisor)).min(i32::from(max_penalty));
    -(penalty as i16)
}

/// Combined strength metric: material (cp) + FFA score converted to cp.
fn combined_strength(material_cp: i16, ffa_score: i32, ffa_weight: i16) -> i32 {
    i32::from(material_cp) + ffa_score * i32::from(ffa_weight)
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
/// Each FFA point = `weight` centipawns.
pub(crate) fn ffa_points_eval(ffa_score: i32, weight: i16) -> i16 {
    let weighted = ffa_score as i64 * weight as i64;
    weighted.clamp(i16::MIN as i64, i16::MAX as i64) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: Standard profile defaults for backward-compatible tests.
    const STD_ENABLED: bool = true;
    const STD_DIVISOR: i16 = 4;
    const STD_MAX: i16 = 150;
    const STD_FFA_WEIGHT: i16 = FFA_POINT_WEIGHT_DEFAULT;

    #[test]
    fn test_lead_penalty_not_leading() {
        let materials = [4300, 4300, 4300, 4300];
        let ffa = [0, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(
            Player::Red, &materials, &ffa, &statuses,
            STD_ENABLED, STD_DIVISOR, STD_MAX,
        );
        assert_eq!(penalty, 0, "No penalty when not leading");
    }

    #[test]
    fn test_lead_penalty_when_leading() {
        // Red has 500cp more material than everyone else.
        let materials = [4800, 4300, 4300, 4300];
        let ffa = [0, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(
            Player::Red, &materials, &ffa, &statuses,
            STD_ENABLED, STD_DIVISOR, STD_MAX,
        );
        // Lead = 500, penalty = -500/4 = -125.
        assert_eq!(penalty, -125);
    }

    #[test]
    fn test_lead_penalty_capped() {
        // Massive lead should be capped.
        let materials = [8000, 4000, 4000, 4000];
        let ffa = [0, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(
            Player::Red, &materials, &ffa, &statuses,
            STD_ENABLED, STD_DIVISOR, STD_MAX,
        );
        assert_eq!(penalty, -STD_MAX);
    }

    #[test]
    fn test_lead_penalty_includes_ffa() {
        // Red has same material but 10 more FFA points.
        let materials = [4300, 4300, 4300, 4300];
        let ffa = [10, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(
            Player::Red, &materials, &ffa, &statuses,
            STD_ENABLED, STD_DIVISOR, STD_MAX,
        );
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
        let penalty = lead_penalty(
            Player::Red, &materials, &ffa, &statuses,
            STD_ENABLED, STD_DIVISOR, STD_MAX,
        );
        assert_eq!(penalty, 0);
    }

    #[test]
    fn test_lead_penalty_disabled_returns_zero() {
        // With penalty disabled (Aggressive profile), always returns 0.
        let materials = [8000, 4000, 4000, 4000];
        let ffa = [0, 0, 0, 0];
        let statuses = [PlayerStatus::Active; 4];
        let penalty = lead_penalty(
            Player::Red, &materials, &ffa, &statuses,
            false, STD_DIVISOR, 0,
        );
        assert_eq!(penalty, 0, "Disabled lead penalty should return 0");
    }

    #[test]
    fn test_ffa_points_eval_positive() {
        assert_eq!(ffa_points_eval(10, STD_FFA_WEIGHT), 500);
    }

    #[test]
    fn test_ffa_points_eval_zero() {
        assert_eq!(ffa_points_eval(0, STD_FFA_WEIGHT), 0);
    }

    #[test]
    fn test_ffa_points_eval_clamped() {
        // Very large FFA score should clamp to i16 bounds.
        let result = ffa_points_eval(1_000_000, STD_FFA_WEIGHT);
        assert_eq!(result, i16::MAX);
    }

    #[test]
    fn test_ffa_points_eval_aggressive_weight() {
        // Aggressive weight = 120cp per point. 10 points = 1200cp.
        assert_eq!(ffa_points_eval(10, 120), 1200);
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
