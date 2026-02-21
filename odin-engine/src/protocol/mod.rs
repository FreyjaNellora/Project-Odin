// Odin protocol — Stage 4
//
// UCI-like text protocol extended for four-player chess.
// Commands on stdin, responses on stdout.
// Malformed input produces error messages, never crashes.

mod parser;
mod types;

pub use parser::parse_command;
pub use types::{Command, EngineOptions, SearchLimits};
