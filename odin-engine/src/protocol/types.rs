// Protocol types for the Odin engine — Stage 4, 8
//
// Command, response, and configuration types for the UCI-like
// Odin protocol extended for four-player chess.

use crate::eval::EvalProfile;
use crate::gamestate::GameMode;

/// Parsed command from stdin.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// `odin` — initialize protocol
    Odin,
    /// `isready` — readiness check
    IsReady,
    /// `setoption name <name> value <value>`
    SetOption { name: String, value: String },
    /// `position fen4 <fen> [moves <move_list>]`
    PositionFen4 { fen: String, moves: Vec<String> },
    /// `position startpos [moves <move_list>]`
    PositionStartpos { moves: Vec<String> },
    /// `go` with optional limits
    Go(SearchLimits),
    /// `stop` — halt current search
    Stop,
    /// `quit` — exit engine
    Quit,
    /// Unknown command (stored for error reporting)
    Unknown(String),
}

/// Time controls and search limits from `go` command.
#[derive(Debug, Default, PartialEq)]
pub struct SearchLimits {
    /// Red time remaining in ms
    pub wtime: Option<u64>,
    /// Blue time remaining in ms
    pub btime: Option<u64>,
    /// Yellow time remaining in ms
    pub ytime: Option<u64>,
    /// Green time remaining in ms
    pub gtime: Option<u64>,
    /// Maximum search depth
    pub depth: Option<u32>,
    /// Maximum nodes to search
    pub nodes: Option<u64>,
    /// Exact time to spend in ms
    pub movetime: Option<u64>,
    /// Search until `stop` command
    pub infinite: bool,
}

/// Engine options set via `setoption`.
#[derive(Debug, Clone)]
pub struct EngineOptions {
    /// Enable debug output
    pub debug: bool,
    /// Use terrain mode for new games
    pub terrain_mode: bool,
    /// Game mode: FFA (points) or LKS (survival). Default: FFA.
    pub game_mode: GameMode,
    /// Eval profile override. None = auto-resolve from game_mode.
    pub eval_profile: Option<EvalProfile>,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            debug: false,
            terrain_mode: false,
            game_mode: GameMode::FreeForAll,
            eval_profile: None,
        }
    }
}

impl EngineOptions {
    /// Resolve the effective eval profile.
    ///
    /// If explicitly set, returns that. Otherwise, auto-resolves:
    /// - FreeForAll → Aggressive
    /// - LastKingStanding → Standard
    pub fn resolved_eval_profile(&self) -> EvalProfile {
        match self.eval_profile {
            Some(profile) => profile,
            None => match self.game_mode {
                GameMode::FreeForAll => EvalProfile::Aggressive,
                GameMode::LastKingStanding => EvalProfile::Standard,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = EngineOptions::default();
        assert!(!opts.debug);
        assert!(!opts.terrain_mode);
        assert_eq!(opts.game_mode, GameMode::FreeForAll);
        assert_eq!(opts.eval_profile, None);
    }

    #[test]
    fn test_resolved_profile_auto_ffa() {
        let opts = EngineOptions {
            game_mode: GameMode::FreeForAll,
            eval_profile: None,
            ..Default::default()
        };
        assert_eq!(opts.resolved_eval_profile(), EvalProfile::Aggressive);
    }

    #[test]
    fn test_resolved_profile_auto_lks() {
        let opts = EngineOptions {
            game_mode: GameMode::LastKingStanding,
            eval_profile: None,
            ..Default::default()
        };
        assert_eq!(opts.resolved_eval_profile(), EvalProfile::Standard);
    }

    #[test]
    fn test_resolved_profile_explicit_override() {
        // Explicitly set Standard in FFA mode (overrides auto Aggressive).
        let opts = EngineOptions {
            game_mode: GameMode::FreeForAll,
            eval_profile: Some(EvalProfile::Standard),
            ..Default::default()
        };
        assert_eq!(opts.resolved_eval_profile(), EvalProfile::Standard);

        // Explicitly set Aggressive in LKS mode (overrides auto Standard).
        let opts2 = EngineOptions {
            game_mode: GameMode::LastKingStanding,
            eval_profile: Some(EvalProfile::Aggressive),
            ..Default::default()
        };
        assert_eq!(opts2.resolved_eval_profile(), EvalProfile::Aggressive);
    }
}
