// Search — Stages 7-11
//
// The `Searcher` trait is the permanent interface boundary between search
// implementations and the rest of the engine.
//
// Stage 7: BrsSearcher (plain BRS + alpha-beta)
// Stage 10: MctsSearcher (MCTS strategic search)
// Stage 11: HybridController (composes BRS and MCTS through this trait)

pub mod board_scanner;
pub mod brs;
pub mod hybrid;
pub mod mcts;
pub mod time_manager;
pub mod tt;

use crate::gamestate::GameState;
use crate::movegen::Move;

/// Time and depth budget for one search invocation.
///
/// All fields are optional — `None` means "no limit for that constraint."
/// The search stops when any non-None limit is reached.
pub struct SearchBudget {
    /// Maximum search depth (inclusive). None = no depth limit.
    pub max_depth: Option<u8>,
    /// Maximum nodes to visit. None = no node limit.
    pub max_nodes: Option<u64>,
    /// Maximum wall-clock time in milliseconds. None = no time limit.
    pub max_time_ms: Option<u64>,
}

/// The result of a completed search.
pub struct SearchResult {
    /// Best move found. Always a legal move from the starting position.
    pub best_move: Move,
    /// Score in centipawns from the root player's perspective.
    /// Positive = good for root player. Range: approximately -30000 to +30000.
    pub score: i16,
    /// Deepest fully completed iterative deepening depth.
    pub depth: u8,
    /// Total nodes visited during the search.
    pub nodes: u64,
    /// Principal variation: the best line found, starting with `best_move`.
    pub pv: Vec<Move>,
}

/// Interface for search implementations.
///
/// BrsSearcher implements this in Stage 7.
/// MctsSearcher implements this in Stage 10.
/// The hybrid controller in Stage 11 composes two Searchers through this trait.
///
/// The `&mut self` allows implementations to maintain internal mutable state
/// (history heuristic, TT in Stage 9, MCTS tree in Stage 10).
pub trait Searcher {
    /// Search the given position within the given budget and return the best move.
    ///
    /// `position` is not modified. Implementations may clone it internally.
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult;
}
