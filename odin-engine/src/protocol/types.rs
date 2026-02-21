// Protocol types for the Odin engine — Stage 4
//
// Command, response, and configuration types for the UCI-like
// Odin protocol extended for four-player chess.

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
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            debug: false,
            terrain_mode: false,
        }
    }
}
