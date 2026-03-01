// FFA scoring strategy evaluation — Stage 17
//
// Adjusts evaluation based on FFA point dynamics:
//   - Claim-win urgency: bonus when approaching 21-point lead with few players
//   - Opponent claim-win threat: penalty when opponent nears claim-win

use crate::board::Player;
use crate::gamestate::{GameMode, PlayerStatus};

use super::EvalWeights;

/// FFA claim-win threshold (21 points in standard rules).
const CLAIM_WIN_THRESHOLD: i32 = 21;

/// Compute FFA-specific strategy eval adjustments.
///
/// Returns a bonus/penalty in centipawns. Only active in FreeForAll mode.
pub(crate) fn ffa_strategy_eval(
    player: Player,
    scores: &[i32; 4],
    statuses: &[PlayerStatus; 4],
    game_mode: GameMode,
    weights: &EvalWeights,
) -> i16 {
    if game_mode != GameMode::FreeForAll {
        return 0;
    }

    let player_score = scores[player.index()];
    let active_count = statuses
        .iter()
        .filter(|&&s| s != PlayerStatus::Eliminated)
        .count();

    // Find the max opponent score
    let max_opp_score = Player::ALL
        .iter()
        .filter(|&&p| p != player && statuses[p.index()] != PlayerStatus::Eliminated)
        .map(|&p| scores[p.index()])
        .max()
        .unwrap_or(0);

    let mut adjustment: i16 = 0;

    // --- Claim-win urgency ---
    // When we have a big lead and few active players, push to claim the win.
    let our_lead = player_score - max_opp_score;
    if our_lead >= 15 && active_count <= 2 {
        // Scale bonus by proximity to claim-win threshold
        let proximity = (our_lead as f64 / CLAIM_WIN_THRESHOLD as f64).min(1.0);
        let bonus = (proximity * weights.claim_win_urgency_bonus as f64) as i16;
        adjustment = adjustment.saturating_add(bonus);
    }

    // --- Opponent claim-win threat ---
    // When an opponent is close to claim-win, apply a penalty to motivate disruption.
    let opp_lead = max_opp_score - player_score;
    if opp_lead >= 15 && active_count <= 2 {
        let threat = (opp_lead as f64 / CLAIM_WIN_THRESHOLD as f64).min(1.0);
        let penalty = (threat * weights.claim_win_urgency_bonus as f64 * 0.5) as i16;
        adjustment = adjustment.saturating_sub(penalty);
    }

    adjustment
}
