// Protocol types for the Odin engine — Stage 4, 8
//
// Command, response, and configuration types for the UCI-like
// Odin protocol extended for four-player chess.

use crate::board::Player;
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
    /// Red increment per move in ms
    pub winc: Option<u64>,
    /// Blue increment per move in ms
    pub binc: Option<u64>,
    /// Yellow increment per move in ms
    pub yinc: Option<u64>,
    /// Green increment per move in ms
    pub ginc: Option<u64>,
    /// Moves until next time control reset
    pub movestogo: Option<u32>,
    /// Maximum search depth
    pub depth: Option<u32>,
    /// Maximum nodes to search
    pub nodes: Option<u64>,
    /// Exact time to spend in ms
    pub movetime: Option<u64>,
    /// Search until `stop` command
    pub infinite: bool,
}

impl SearchLimits {
    /// Get remaining time and increment for a specific player.
    pub fn time_for_player(&self, player: Player) -> (Option<u64>, Option<u64>) {
        match player {
            Player::Red => (self.wtime, self.winc),
            Player::Blue => (self.btime, self.binc),
            Player::Yellow => (self.ytime, self.yinc),
            Player::Green => (self.gtime, self.ginc),
        }
    }
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
    // --- Tunable search parameters (Stage 13) ---
    /// BRS survivor threshold in centipawns. Default: 150.
    pub tactical_margin: Option<i16>,
    /// BRS time fraction for tactical positions. Default: 0.30.
    pub brs_fraction_tactical: Option<f64>,
    /// BRS time fraction for quiet positions. Default: 0.10.
    pub brs_fraction_quiet: Option<f64>,
    /// Default MCTS simulation count (depth-only mode). Default: 2000.
    pub mcts_default_sims: Option<u64>,
    /// Maximum BRS search depth. Default: 8.
    pub brs_max_depth: Option<u8>,
    // --- NNUE (Stage 16) ---
    /// Path to .onnue weight file. None = use bootstrap eval.
    pub nnue_file: Option<String>,
    // --- Chess960 (Stage 17) ---
    /// Enable Chess960 (Fischer Random) starting positions.
    pub chess960: bool,
    // --- Defense-aware move ordering ---
    /// Multiplier for defense bonus in move ordering. None = default (0.5).
    /// 0.0 = disabled (current behavior). Range: 0.0 - 2.0.
    pub defense_weight: Option<f32>,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            debug: false,
            terrain_mode: false,
            game_mode: GameMode::FreeForAll,
            eval_profile: None,
            tactical_margin: None,
            brs_fraction_tactical: None,
            brs_fraction_quiet: None,
            mcts_default_sims: None,
            brs_max_depth: None,
            nnue_file: None,
            chess960: false,
            defense_weight: None,
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
