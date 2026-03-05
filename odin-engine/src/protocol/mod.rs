// Odin protocol — Stage 4
//
// UCI-like text protocol extended for four-player chess.
// Commands on stdin, responses on stdout.
// Malformed input produces error messages, never crashes.

mod emitter;
mod parser;
mod types;

pub use parser::parse_command;
pub use types::{Command, EngineOptions, SearchLimits};

use std::cell::RefCell;
use std::io::{BufRead, BufWriter, Write};
use std::fs::File;
use std::rc::Rc;

use crate::board::{Board, Player};
use crate::gamestate::{EliminationReason, GameMode, GameState};
use crate::movegen::is_in_check;
use crate::search::hybrid::HybridController;
use crate::search::{SearchBudget, Searcher};

use emitter::{format_bestmove, format_error, format_id, format_readyok};

/// The Odin engine protocol handler.
///
/// Owns the game state and processes commands from stdin.
/// Responds on stdout. Collects output in a buffer for testing.
pub struct OdinEngine {
    game_state: Option<GameState>,
    options: EngineOptions,
    /// Collected output lines (for testing).
    output_buffer: Vec<String>,
    /// Persistent hybrid searcher — BRS TT survives across `go` calls so entries
    /// from earlier searches can inform later ones (generation-based aging handles
    /// staleness). Created lazily on the first `go` command.
    searcher: Option<HybridController>,
    /// Optional protocol log file. When Some, all incoming commands and outgoing
    /// responses are written here. Toggle via `setoption name LogFile value <path>`
    /// (set to "none" or "" to close). Zero overhead when None.
    log_file: Option<BufWriter<File>>,
}

impl OdinEngine {
    /// Create a new engine with default settings.
    pub fn new() -> Self {
        Self {
            game_state: None,
            options: EngineOptions::default(),
            output_buffer: Vec::new(),
            searcher: None,
            log_file: None,
        }
    }

    /// Run the main protocol loop, reading from stdin.
    pub fn run(&mut self) {
        let stdin = std::io::stdin();
        let reader = stdin.lock();

        for line in reader.lines() {
            match line {
                Ok(input) => {
                    // Log incoming command if logging is active
                    if let Some(ref mut f) = self.log_file {
                        let _ = writeln!(f, "> {input}");
                    }
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
            "gamemode" | "game_mode" => {
                let v = value.to_lowercase();
                match v.as_str() {
                    "ffa" | "freeforall" | "free_for_all" => {
                        self.options.game_mode = GameMode::FreeForAll;
                    }
                    "lks" | "lastkingstanding" | "last_king_standing" => {
                        self.options.game_mode = GameMode::LastKingStanding;
                    }
                    _ => {} // Silently ignore unknown values
                }
            }
            "evalprofile" | "eval_profile" => {
                let v = value.to_lowercase();
                match v.as_str() {
                    "standard" | "std" => {
                        self.options.eval_profile = Some(crate::eval::EvalProfile::Standard);
                    }
                    "aggressive" | "aggro" => {
                        self.options.eval_profile = Some(crate::eval::EvalProfile::Aggressive);
                    }
                    "auto" | "none" => {
                        self.options.eval_profile = None;
                    }
                    _ => {} // Silently ignore unknown values
                }
            }
            "logfile" | "log_file" => {
                let v = value.trim();
                if v.is_empty() || v.eq_ignore_ascii_case("none") || v.eq_ignore_ascii_case("off") {
                    // Flush and close
                    if let Some(ref mut f) = self.log_file {
                        let _ = f.flush();
                    }
                    self.log_file = None;
                } else {
                    // Create parent dirs if needed, then open file
                    if let Some(parent) = std::path::Path::new(v).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match File::create(v) {
                        Ok(f) => {
                            self.log_file = Some(BufWriter::new(f));
                            // Write header
                            if let Some(ref mut lf) = self.log_file {
                                let _ = writeln!(lf, "# Odin protocol log");
                                let _ = writeln!(lf, "# > = incoming command, < = engine response");
                                let _ = writeln!(lf, "# LogFile opened");
                            }
                        }
                        Err(e) => {
                            self.send(&format_error(&format!("cannot open log file: {e}")));
                        }
                    }
                }
            }
            // --- Tunable search parameters (Stage 13) ---
            "tactical_margin" => {
                if let Ok(v) = value.parse::<i16>() {
                    self.options.tactical_margin = Some(v);
                }
            }
            "brs_fraction_tactical" => {
                if let Ok(v) = value.parse::<f64>() {
                    self.options.brs_fraction_tactical = Some(v.clamp(0.0, 1.0));
                }
            }
            "brs_fraction_quiet" => {
                if let Ok(v) = value.parse::<f64>() {
                    self.options.brs_fraction_quiet = Some(v.clamp(0.0, 1.0));
                }
            }
            "mcts_default_sims" => {
                if let Ok(v) = value.parse::<u64>() {
                    self.options.mcts_default_sims = Some(v);
                }
            }
            "brs_max_depth" => {
                if let Ok(v) = value.parse::<u8>() {
                    self.options.brs_max_depth = Some(v.clamp(1, 20));
                }
            }
            // --- Chess960 (Stage 17) ---
            "chess960" => {
                self.options.chess960 = value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("on")
                    || value == "1";
            }
            // --- NNUE (Stage 16) ---
            "nnuefile" | "nnue_file" => {
                let v = value.trim();
                if v.is_empty() || v.eq_ignore_ascii_case("none") || v.eq_ignore_ascii_case("off") {
                    self.options.nnue_file = None;
                } else {
                    self.options.nnue_file = Some(v.to_string());
                }
                // Force searcher recreation with new eval
                self.searcher = None;
            }
            _ => {
                // Accept and silently ignore unrecognized options (UCI convention)
            }
        }
    }

    fn handle_position_fen4(&mut self, fen: &str, moves: &[String]) {
        match Board::from_fen4(fen) {
            Ok(board) => {
                let mode = self.options.game_mode;
                let terrain = self.options.terrain_mode;
                self.game_state = Some(GameState::new(board, mode, terrain));
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
        let mode = self.options.game_mode;
        let terrain = self.options.terrain_mode;
        let board = if self.options.chess960 {
            // Use system time as seed for Chess960 positions.
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(42);
            Board::chess960_position(seed)
        } else {
            Board::starting_position()
        };
        self.game_state = Some(GameState::new(board, mode, terrain));
        if let Err(e) = self.apply_moves(moves) {
            self.send(&format_error(&e));
            self.game_state = None;
        }
    }

    fn handle_go(&mut self, limits: &SearchLimits) {
        if self.game_state.is_none() {
            self.send(&format_error("no position set, send 'position' first"));
            return;
        }
        if self.game_state.as_ref().expect("game_state is Some: checked by is_none() guard above").is_game_over() {
            self.send(&format_error("game is already over"));
            return;
        }

        // Handle checkmate/stalemate: current player has no legal moves.
        if self.game_state.as_mut().expect("game_state is Some: checked by is_none() guard above").legal_moves().is_empty() {
            let move_result = self.game_state.as_mut().expect("game_state is Some: checked by is_none() guard above").handle_no_legal_moves();

            for (player, reason) in &move_result.eliminations {
                let reason_str = match reason {
                    EliminationReason::Checkmate => "checkmate",
                    EliminationReason::Stalemate => "stalemate",
                    _ => "eliminated",
                };
                self.send(&format!(
                    "info string eliminated {} {reason_str}",
                    player_color(*player)
                ));
            }

            if self.game_state.as_ref().expect("game_state is Some: checked by is_none() guard above").is_game_over() {
                let winner_str = self
                    .game_state
                    .as_ref()
                    .expect("game_state is Some: checked by is_none() guard above")
                    .winner()
                    .map_or("none", player_color);
                self.send(&format!("info string gameover {winner_str}"));
            } else {
                let next = self.game_state.as_ref().expect("game_state is Some: checked by is_none() guard above").current_player();
                self.send(&format!("info string nextturn {}", player_color(next)));
                // Recurse: chain eliminations handled by recursion; ultimately produces
                // a bestmove for the first player found to have legal moves.
                self.handle_go(limits);
            }
            return;
        }

        // Normal path: clone position, search, apply to get post-move state.
        // Clone so the mutable borrow of self.game_state is released before send().
        let position = self.game_state.as_ref().expect("game_state is Some: checked by is_none() guard above").clone();
        let current_player = position.current_player();

        let budget = Self::limits_to_budget(limits, Some(current_player));

        // Collect info strings via callback (Rc/RefCell: single-threaded, blocking search).
        let info_buf = Rc::new(RefCell::new(Vec::<String>::new()));
        let info_buf_cb = Rc::clone(&info_buf);
        let cb: Box<dyn FnMut(String)> = Box::new(move |line: String| {
            info_buf_cb.borrow_mut().push(line);
        });

        let profile = self.options.resolved_eval_profile();
        let nnue_path = self.options.nnue_file.clone();
        let searcher = self.searcher.get_or_insert_with(|| {
            HybridController::new(profile, nnue_path.as_deref())
        });

        // Stage 13: Set time context if time controls are present.
        let (own_time, own_inc) = limits.time_for_player(current_player);
        if let Some(remaining) = own_time {
            let ply = position.position_history().len() as u32;
            searcher.set_time_context(crate::search::time_manager::TimeContext {
                remaining_ms: remaining,
                increment_ms: own_inc.unwrap_or(0),
                movestogo: limits.movestogo,
                ply,
            });
        }

        // Apply tunable parameter overrides.
        searcher.apply_options(&self.options);

        searcher.set_info_callback(cb);
        let result = searcher.search(&position, budget);

        // Apply the best move to determine post-move state for UI synchronization.
        let mut post_position = position;
        let move_result = post_position.apply_move(result.best_move);

        for line in info_buf.borrow().iter() {
            self.send(line);
        }

        // Emit elimination events before bestmove so the UI can process them first.
        for (player, _reason) in &move_result.eliminations {
            self.send(&format!("info string eliminated {}", player_color(*player)));
        }

        // Emit check status for the next player (Stage 18: UI check highlight).
        if !post_position.is_game_over() {
            let next_player = post_position.current_player();
            if is_in_check(next_player, post_position.board()) {
                self.send(&format!(
                    "info string in_check {}",
                    player_color(next_player)
                ));
            }
        }

        // Emit game-over or next-turn indicator so the UI can sync its turn tracker.
        if post_position.is_game_over() {
            let winner_str = post_position.winner().map_or("none", player_color);
            self.send(&format!("info string gameover {winner_str}"));
        } else {
            self.send(&format!(
                "info string nextturn {}",
                player_color(post_position.current_player())
            ));
        }

        self.send(&format_bestmove(&result.best_move.to_algebraic(), None));
    }

    /// Convert `SearchLimits` (from the protocol) into a `SearchBudget` for the searcher.
    ///
    /// Priority: infinite > movetime > depth > nodes > time controls > default (depth 8).
    ///
    /// When `current_player` is Some, the correct player's time is used (Stage 13 fix).
    /// The real time allocation is computed by `TimeManager` inside `HybridController`,
    /// so this function provides a conservative fallback (time/50, min 200ms).
    fn limits_to_budget(limits: &SearchLimits, current_player: Option<Player>) -> SearchBudget {
        if limits.infinite {
            return SearchBudget {
                max_depth: None,
                max_nodes: None,
                max_time_ms: None,
            };
        }
        if let Some(mt) = limits.movetime {
            return SearchBudget {
                max_depth: None,
                max_nodes: None,
                max_time_ms: Some(mt),
            };
        }
        if let Some(d) = limits.depth {
            return SearchBudget {
                max_depth: Some(d as u8),
                max_nodes: None,
                max_time_ms: None,
            };
        }
        if let Some(n) = limits.nodes {
            return SearchBudget {
                max_depth: None,
                max_nodes: Some(n),
                max_time_ms: None,
            };
        }
        // Time controls: use the current player's time (Stage 13 fix).
        // Falls back to first available if no player specified.
        let own_time = if let Some(player) = current_player {
            limits.time_for_player(player).0
        } else {
            limits
                .wtime
                .or(limits.btime)
                .or(limits.ytime)
                .or(limits.gtime)
        };
        if let Some(t) = own_time {
            // Conservative fallback: time/50 with min 200ms.
            // The real allocation is computed by TimeManager in HybridController.
            let ms = (t / 50).max(200);
            return SearchBudget {
                max_depth: None,
                max_nodes: None,
                max_time_ms: Some(ms),
            };
        }
        // No limits specified: default to depth 8 (2 full rotations in 4PC).
        SearchBudget {
            max_depth: Some(8),
            max_nodes: None,
            max_time_ms: None,
        }
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
    /// If a log file is active, also writes there (prefixed with `< `).
    fn send(&mut self, line: &str) {
        println!("{line}");
        self.output_buffer.push(line.to_string());
        if let Some(ref mut f) = self.log_file {
            let _ = writeln!(f, "< {line}");
            let _ = f.flush();
        }
    }
}

impl Default for OdinEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a player to its display color string (used in protocol events).
fn player_color(player: Player) -> &'static str {
    match player {
        Player::Red => "Red",
        Player::Blue => "Blue",
        Player::Yellow => "Yellow",
        Player::Green => "Green",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{square_from, Piece, PieceType};
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

        // depth 4: fast in debug mode, deterministic, exercises iterative deepening.
        engine.handle_command(Command::Go(SearchLimits {
            depth: Some(4),
            ..Default::default()
        }));
        let output = engine.take_output();

        // Should have at least one info line + bestmove as the final line.
        assert!(output.len() >= 2);
        assert!(output[0].starts_with("info "));
        assert!(output.last().unwrap().starts_with("bestmove "));
    }

    #[test]
    fn test_go_bestmove_is_legal() {
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionStartpos { moves: vec![] });
        engine.take_output();

        engine.handle_command(Command::Go(SearchLimits {
            depth: Some(4),
            ..Default::default()
        }));
        let output = engine.take_output();

        // bestmove is always the last line; info lines precede it.
        let bestmove_line = output.last().unwrap();
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

        engine.handle_command(Command::Go(SearchLimits {
            depth: Some(4),
            ..Default::default()
        }));
        let output = engine.take_output();

        // Depth-1 info line (output[0]) must carry all required fields.
        let info_line = &output[0];
        assert!(info_line.contains("depth "));
        assert!(info_line.contains("score cp "));
        assert!(info_line.contains("v1 "));
        assert!(info_line.contains("v2 "));
        assert!(info_line.contains("v3 "));
        assert!(info_line.contains("v4 "));
        assert!(info_line.contains("phase brs"));
    }

    #[test]
    fn test_go_mated_player_emits_eliminated_and_advances() {
        // Red king at h1 (7,0) is in checkmate (same position as gamestate test):
        // Green queen at i2 (8,1) gives check; Blue bishop at c8 (2,7) protects queen;
        // Blue rook at g5 (6,4) covers g1 — the only remaining escape square.
        let mut board = Board::empty();
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.place_piece(
            square_from(8, 1).unwrap(),
            Piece::new(PieceType::Queen, Player::Green),
        );
        board.place_piece(
            square_from(2, 7).unwrap(),
            Piece::new(PieceType::Bishop, Player::Blue),
        );
        board.place_piece(
            square_from(6, 4).unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );
        board.place_piece(
            square_from(3, 13).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.place_piece(
            square_from(6, 13).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.place_piece(
            square_from(13, 6).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Red);

        let fen = board.to_fen4();
        let mut engine = OdinEngine::new();
        engine.handle_command(Command::PositionFen4 { fen, moves: vec![] });
        engine.take_output();

        engine.handle_command(Command::Go(SearchLimits {
            depth: Some(2),
            ..Default::default()
        }));
        let output = engine.take_output();

        // Must not emit an error
        assert!(
            !output.iter().any(|l| l.contains("Error")),
            "handle_go should not error on a mated player; output: {output:?}"
        );
        // Red should be reported as eliminated
        assert!(
            output.iter().any(|l| l.contains("eliminated Red")),
            "expected 'eliminated Red' in output; got: {output:?}"
        );
        // A nextturn event should follow
        assert!(
            output.iter().any(|l| l.contains("nextturn")),
            "expected nextturn in output; got: {output:?}"
        );
        // The final line must be a bestmove (for the next alive player)
        assert!(
            output.last().unwrap().starts_with("bestmove "),
            "expected bestmove as last line; got: {output:?}"
        );
    }
}
