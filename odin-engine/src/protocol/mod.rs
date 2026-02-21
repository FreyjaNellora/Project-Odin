// Odin protocol — Stage 4
//
// UCI-like text protocol extended for four-player chess.
// Commands on stdin, responses on stdout.
// Malformed input produces error messages, never crashes.
//
// Huginn gates defined but not wired (deferred per established pattern):
// - command_receive: raw string, parsed type, parse errors
// - response_send: full string, what triggered it
// - position_set: FEN4/startpos, move list, resulting hash
// - search_request: time controls, depth limits, options

mod emitter;
mod parser;
mod types;

pub use parser::parse_command;
pub use types::{Command, EngineOptions, SearchLimits};

use crate::board::Board;
use crate::gamestate::{GameMode, GameState};

use emitter::{format_bestmove, format_error, format_id, format_info, format_readyok, SearchInfo};

use std::io::BufRead;

/// The Odin engine protocol handler.
///
/// Owns the game state and processes commands from stdin.
/// Responds on stdout. Collects output in a buffer for testing.
pub struct OdinEngine {
    game_state: Option<GameState>,
    options: EngineOptions,
    rng_seed: u64,
    /// Collected output lines (for testing).
    output_buffer: Vec<String>,
}

impl OdinEngine {
    /// Create a new engine with default settings.
    pub fn new() -> Self {
        Self {
            game_state: None,
            options: EngineOptions::default(),
            rng_seed: 0x0D14_CAFE_0000_BEEF,
            output_buffer: Vec::new(),
        }
    }

    /// Run the main protocol loop, reading from stdin.
    pub fn run(&mut self) {
        let stdin = std::io::stdin();
        let reader = stdin.lock();

        for line in reader.lines() {
            match line {
                Ok(input) => {
                    let command = parser::parse_command(&input);
                    if self.handle_command(command) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }

    /// Handle a single command. Returns true if the engine should quit.
    pub fn handle_command(&mut self, command: Command) -> bool {
        match command {
            Command::Odin => self.handle_odin(),
            Command::IsReady => self.handle_isready(),
            Command::SetOption { name, value } => self.handle_setoption(&name, &value),
            Command::PositionFen4 { fen, moves } => self.handle_position_fen4(&fen, &moves),
            Command::PositionStartpos { moves } => self.handle_position_startpos(&moves),
            Command::Go(limits) => self.handle_go(&limits),
            Command::Stop => { /* No-op: no search to stop in Stage 4 */ }
            Command::Quit => return true,
            Command::Unknown(s) => {
                if !s.is_empty() {
                    self.send(&format_error(&format!("unknown command: {s}")));
                }
            }
        }
        false
    }

    /// Take and clear collected output (for testing).
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output_buffer)
    }

    /// Get immutable access to the game state (for testing).
    pub fn game_state(&self) -> Option<&GameState> {
        self.game_state.as_ref()
    }

    // --- Command handlers ---

    fn handle_odin(&mut self) {
        for line in format_id() {
            self.send(&line);
        }
    }

    fn handle_isready(&mut self) {
        self.send(format_readyok());
    }

    fn handle_setoption(&mut self, name: &str, value: &str) {
        match name.to_lowercase().as_str() {
            "debug" => {
                self.options.debug = value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("on")
                    || value == "1";
            }
            "terrain" => {
                self.options.terrain_mode = value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("on")
                    || value == "1";
            }
            _ => {
                // Accept and silently ignore unrecognized options (UCI convention)
            }
        }
    }

    fn handle_position_fen4(&mut self, fen: &str, moves: &[String]) {
        match Board::from_fen4(fen) {
            Ok(board) => {
                let terrain = self.options.terrain_mode;
                self.game_state = Some(GameState::new(board, GameMode::FreeForAll, terrain));
                if let Err(e) = self.apply_moves(moves) {
                    self.send(&format_error(&e));
                    self.game_state = None;
                }
            }
            Err(e) => {
                self.send(&format_error(&format!("invalid FEN4: {e}")));
            }
        }
    }

    fn handle_position_startpos(&mut self, moves: &[String]) {
        if self.options.terrain_mode {
            self.game_state = Some(GameState::new_standard_ffa_terrain());
        } else {
            self.game_state = Some(GameState::new_standard_ffa());
        }
        if let Err(e) = self.apply_moves(moves) {
            self.send(&format_error(&e));
            self.game_state = None;
        }
    }

    fn handle_go(&mut self, _limits: &SearchLimits) {
        let gs = match self.game_state.as_mut() {
            Some(gs) => gs,
            None => {
                self.send(&format_error("no position set, send 'position' first"));
                return;
            }
        };

        if gs.is_game_over() {
            self.send(&format_error("game is already over"));
            return;
        }

        let legal = gs.legal_moves();
        if legal.is_empty() {
            self.send(&format_error("no legal moves available"));
            return;
        }

        // Stage 4: pick a random legal move via LCG
        self.rng_seed = self
            .rng_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let idx = (self.rng_seed >> 33) as usize % legal.len();
        let mv = legal[idx];

        // Send info line with minimal data
        let scores = gs.scores();
        let info = SearchInfo {
            depth: Some(0),
            score_cp: Some(0),
            values: Some(scores),
            nodes: Some(1),
            time_ms: Some(0),
            pv: vec![mv.to_algebraic()],
            ..Default::default()
        };
        self.send(&format_info(&info));
        self.send(&format_bestmove(&mv.to_algebraic(), None));
    }

    // --- Utilities ---

    /// Apply a sequence of move strings to the current game state.
    fn apply_moves(&mut self, moves: &[String]) -> Result<(), String> {
        for move_str in moves {
            let gs = self
                .game_state
                .as_mut()
                .ok_or_else(|| "no position set".to_string())?;

            if gs.is_game_over() {
                return Err(format!("game is over, cannot apply move: {move_str}"));
            }

            let legal = gs.legal_moves();
            let mv = legal
                .into_iter()
                .find(|m| m.to_algebraic() == *move_str)
                .ok_or_else(|| format!("illegal or unrecognized move: {move_str}"))?;
            gs.apply_move(mv);
        }
        Ok(())
    }

    /// Send a line to stdout and record it in the output buffer.
    fn send(&mut self, line: &str) {
        println!("{line}");
        self.output_buffer.push(line.to_string());
    }
}

impl Default for OdinEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movegen;

    #[test]
    fn test_odin_command_sends_id() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::Odin);
        let output = engine.take_output();
        assert_eq!(output.len(), 3);
        assert!(output[0].starts_with("id name Odin"));
        assert!(output[1].contains("author"));
        assert_eq!(output[2], "odinok");
    }

    #[test]
    fn test_isready_sends_readyok() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::IsReady);
        let output = engine.take_output();
        assert_eq!(output, vec!["readyok"]);
    }

    #[test]
    fn test_quit_returns_true() {
        let mut engine = OdinEngine::new();
        assert!(engine.handle_command(Command::Quit));
    }

    #[test]
    fn test_stop_is_noop() {
        let mut engine = OdinEngine::new();
        assert!(!engine.handle_command(Command::Stop));
        assert!(engine.take_output().is_empty());
    }

    #[test]
    fn test_unknown_command_sends_error() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::Unknown("foobar".to_string()));
        let output = engine.take_output();
        assert_eq!(output.len(), 1);
        assert!(output[0].contains("Error"));
        assert!(output[0].contains("foobar"));
    }

    #[test]
    fn test_empty_unknown_is_silent() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::Unknown(String::new()));
        assert!(engine.take_output().is_empty());
    }

    #[test]
    fn test_setoption_debug() {
        let mut engine = OdinEngine::new();
        assert!(!engine.options.debug);
        engine.handle_command(Command::SetOption {
            name: "Debug".to_string(),
            value: "true".to_string(),
        });
        assert!(engine.options.debug);
    }

    #[test]
    fn test_setoption_terrain() {
        let mut engine = OdinEngine::new();
        assert!(!engine.options.terrain_mode);
        engine.handle_command(Command::SetOption {
            name: "Terrain".to_string(),
            value: "on".to_string(),
        });
        assert!(engine.options.terrain_mode);
    }

    #[test]
    fn test_position_startpos() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionStartpos { moves: vec![] });
        assert!(engine.game_state().is_some());
        assert!(engine.take_output().is_empty());
    }

    #[test]
    fn test_position_startpos_with_moves() {
        let mut engine = OdinEngine::new();
        // Get the first legal move from starting position to use
        let gs = GameState::new_standard_ffa();
        let board = gs.board().clone();
        let legal = movegen::generate_legal(&mut board.clone());
        let first_move = legal[0].to_algebraic();

        engine.handle_command(Command::PositionStartpos {
            moves: vec![first_move],
        });
        assert!(engine.game_state().is_some());
        assert!(engine.take_output().is_empty());
    }

    #[test]
    fn test_position_fen4_starting() {
        let mut engine = OdinEngine::new();
        let fen = Board::starting_position().to_fen4();
        engine.handle_command(Command::PositionFen4 { fen, moves: vec![] });
        assert!(engine.game_state().is_some());
        assert!(engine.take_output().is_empty());
    }

    #[test]
    fn test_position_invalid_fen4() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionFen4 {
            fen: "garbage".to_string(),
            moves: vec![],
        });
        assert!(engine.game_state().is_none());
        let output = engine.take_output();
        assert_eq!(output.len(), 1);
        assert!(output[0].contains("Error"));
        assert!(output[0].contains("invalid FEN4"));
    }

    #[test]
    fn test_position_with_illegal_move() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionStartpos {
            moves: vec!["z9z9".to_string()],
        });
        // Position should be cleared on error
        assert!(engine.game_state().is_none());
        let output = engine.take_output();
        assert!(output[0].contains("Error"));
        assert!(output[0].contains("z9z9"));
    }

    #[test]
    fn test_go_without_position() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::Go(SearchLimits::default()));
        let output = engine.take_output();
        assert!(output[0].contains("Error"));
        assert!(output[0].contains("no position set"));
    }

    #[test]
    fn test_go_returns_bestmove() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionStartpos { moves: vec![] });
        engine.take_output(); // Clear position output

        engine.handle_command(Command::Go(SearchLimits::default()));
        let output = engine.take_output();

        // Should have info + bestmove
        assert!(output.len() >= 2);
        assert!(output[0].starts_with("info "));
        assert!(output[1].starts_with("bestmove "));
    }

    #[test]
    fn test_go_bestmove_is_legal() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionStartpos { moves: vec![] });
        engine.take_output();

        engine.handle_command(Command::Go(SearchLimits::default()));
        let output = engine.take_output();

        // Extract the move from "bestmove d4d5"
        let bestmove_line = &output[1];
        let move_str = bestmove_line.strip_prefix("bestmove ").unwrap();

        // Verify it's a legal move from starting position
        let mut board = Board::starting_position();
        let legal = movegen::generate_legal(&mut board);
        let found = legal.iter().any(|m| m.to_algebraic() == move_str);
        assert!(found, "bestmove '{move_str}' is not a legal move");
    }

    #[test]
    fn test_go_info_contains_scores() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionStartpos { moves: vec![] });
        engine.take_output();

        engine.handle_command(Command::Go(SearchLimits::default()));
        let output = engine.take_output();

        let info_line = &output[0];
        assert!(info_line.contains("v1 "));
        assert!(info_line.contains("v2 "));
        assert!(info_line.contains("v3 "));
        assert!(info_line.contains("v4 "));
    }
}
