// Time Management — Stage 13
//
// Pure-function time allocation for 4-player chess. Computes how many
// milliseconds to spend on the current move based on remaining clock,
// increment, game phase, and position features.
//
// Called by HybridController::search() with full position context.
// The protocol layer extracts clock info into a TimeContext; the search
// layer provides position features (legal move count, check status, etc.).

/// Clock information passed from the protocol layer to the search layer.
///
/// Contains time control data that the search layer needs but cannot derive
/// from `GameState` alone (because clock times are external to game state).
#[derive(Debug, Clone)]
pub struct TimeContext {
    /// Remaining time for the side to move, in ms.
    pub remaining_ms: u64,
    /// Per-move increment for the side to move, in ms.
    pub increment_ms: u64,
    /// Moves until next time control reset (None = sudden death / Fischer).
    pub movestogo: Option<u32>,
    /// Current game ply (number of half-moves played so far).
    pub ply: u32,
}

/// Stateless time manager. All methods are pure functions.
pub struct TimeManager;

impl TimeManager {
    /// Calculate time budget for this move.
    ///
    /// Returns allocated time in milliseconds.
    #[allow(clippy::too_many_arguments)]
    ///
    /// # Safety constraints (never flag)
    /// - Never uses more than 25% of remaining clock
    /// - Minimum 100ms (or remaining time if less)
    /// - Panic mode: if remaining < 1s, uses at most 10%
    pub fn allocate(
        remaining_ms: u64,
        increment_ms: u64,
        ply: u32,
        movestogo: Option<u32>,
        num_legal_moves: usize,
        is_tactical: bool,
        is_in_check: bool,
        score_cp: Option<i16>,
    ) -> u64 {
        // Forced move: 1 legal move → instant return
        if num_legal_moves <= 1 {
            return 0;
        }

        // No time remaining → can't allocate anything
        if remaining_ms == 0 {
            return 0;
        }

        // Estimate moves remaining in the game.
        // In 4-player chess, games tend to run ~50 ply per player.
        // As ply increases, we expect fewer moves remaining.
        let estimated_remaining = (50u32.saturating_sub(ply / 4)).clamp(10, 50);
        let moves_left = match movestogo {
            Some(mtg) if mtg > 0 => mtg.max(estimated_remaining),
            _ => estimated_remaining,
        };

        // Base time allocation: divide remaining evenly + increment
        let base_time = remaining_ms / moves_left as u64 + increment_ms;

        // Position-based adjustment factor
        let mut factor: f64 = 1.0;

        if is_tactical {
            factor *= 1.3;
        } else {
            factor *= 0.8; // quiet position: conserve time
        }

        // Near-elimination: score below 2000cp means lost ~half material.
        // Spend more time to find survival moves.
        if let Some(score) = score_cp {
            if score < 2000 {
                factor *= 2.0;
            }
        }

        // In check: need slightly more time to find the best escape
        if is_in_check {
            factor *= 1.2;
        }

        // Apply factor
        let mut allocated = (base_time as f64 * factor) as u64;

        // --- Safety constraints (CRITICAL: never flag) ---

        // 1. Never use more than 25% of remaining clock
        allocated = allocated.min(remaining_ms / 4);

        // 2. Minimum 100ms (or all remaining if less than 100ms)
        allocated = allocated.max(100.min(remaining_ms));

        // 3. Panic mode: if remaining < 1s, use at most 10%
        if remaining_ms < 1000 {
            allocated = allocated.min(remaining_ms / 10);
        }

        allocated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let ms = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, None);
        // 60000 / 50 * 0.8 = 960, capped at 25% = 15000, min 100
        assert!(ms >= 100, "should be at least 100ms, got {ms}");
        assert!(ms <= 15_000, "should not exceed 25% of remaining, got {ms}");
    }

    #[test]
    fn test_forced_move_returns_zero() {
        assert_eq!(
            TimeManager::allocate(60_000, 0, 0, None, 1, false, false, None),
            0
        );
    }

    #[test]
    fn test_no_remaining_returns_zero() {
        assert_eq!(
            TimeManager::allocate(0, 0, 0, None, 20, false, false, None),
            0
        );
    }

    #[test]
    fn test_increment_increases_allocation() {
        let without = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, None);
        let with = TimeManager::allocate(60_000, 2000, 0, None, 20, false, false, None);
        assert!(with > without, "increment should increase: {with} vs {without}");
    }

    #[test]
    fn test_tactical_gets_more_than_quiet() {
        let quiet = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, None);
        let tactical = TimeManager::allocate(60_000, 0, 0, None, 20, true, false, None);
        assert!(
            tactical > quiet,
            "tactical should get more: {tactical} vs {quiet}"
        );
    }

    #[test]
    fn test_safety_cap_25_percent() {
        // High factor scenario: tactical + in_check + near_elimination
        let ms = TimeManager::allocate(4_000, 0, 0, None, 20, true, true, Some(1500));
        assert!(ms <= 1000, "should not exceed 25% (1000ms), got {ms}");
    }

    #[test]
    fn test_panic_mode() {
        let ms = TimeManager::allocate(500, 0, 40, None, 20, false, false, None);
        assert!(ms <= 50, "panic mode: should use <=10% (50ms), got {ms}");
    }

    #[test]
    fn test_near_elimination_bonus() {
        let normal = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, Some(4000));
        let desperate =
            TimeManager::allocate(60_000, 0, 0, None, 20, false, false, Some(1500));
        assert!(
            desperate > normal,
            "near-elimination should get more: {desperate} vs {normal}"
        );
    }

    #[test]
    fn test_late_game_allocates_more_per_move() {
        // At ply 160 (40 moves per player), estimated_remaining = max(50-40, 10) = 10
        // So each move gets more time (remaining/10 vs remaining/50)
        let early = TimeManager::allocate(30_000, 0, 0, None, 20, false, false, None);
        let late = TimeManager::allocate(30_000, 0, 160, None, 20, false, false, None);
        assert!(
            late > early,
            "late game should allocate more per move: {late} vs {early}"
        );
    }
}
