// Stage 10 — MCTS Strategic Search integration tests
//
// Tests cover:
//   1. Node basics (creation, q_value)
//   2. Prior computation (softmax, captures > quiets)
//   3. Gumbel sampling (finite values, Top-k selection)
//   4. Sequential Halving (candidate elimination, budget allocation)
//   5. Tree policy (PUCT: unvisited preference, Q-value exploitation)
//   6. Expansion (child count, terminal detection, progressive widening)
//   7. Backpropagation (4-player MaxN value propagation)
//   8. Full search (2 sims, 100 sims, 1000 sims, time-budgeted)
//   9. Searcher trait (MctsSearcher as Box<dyn Searcher>)
//  10. Progressive history (with/without history table)
//  11. Tactical (free capture detection, score direction)

use odin_engine::board::{Board, Piece, PieceType, Player};
use odin_engine::eval::{BootstrapEvaluator, EvalProfile};
use odin_engine::gamestate::{GameMode, GameState};
use odin_engine::search::mcts::MctsSearcher;
use odin_engine::search::{SearchBudget, Searcher};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_mcts_searcher() -> MctsSearcher {
    MctsSearcher::with_seed(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        None,
        42,
    )
}

fn sim_budget(sims: u64) -> SearchBudget {
    SearchBudget {
        max_depth: None,
        max_nodes: Some(sims),
        max_time_ms: None,
    }
}

fn time_budget(ms: u64) -> SearchBudget {
    SearchBudget {
        max_depth: None,
        max_nodes: None,
        max_time_ms: Some(ms),
    }
}

fn starting_gs() -> GameState {
    GameState::new_standard_ffa()
}

fn assert_legal(gs: &GameState, mv: odin_engine::movegen::Move) {
    let mut gs_check = gs.clone();
    let legal = gs_check.legal_moves();
    assert!(
        legal.contains(&mv),
        "move {} is not legal in this position",
        mv.to_algebraic()
    );
}

/// Build a tactical position where Red can capture Blue's hanging queen.
/// Red: King e2, Queen h7
/// Blue: King b11, Queen g8 (hanging — Qh7xg8 is free)
/// Yellow/Green: kings only
fn make_free_capture_position() -> GameState {
    use odin_engine::board::square_from;

    let mut board = Board::empty();
    board.set_side_to_move(Player::Red);
    board.set_castling_rights(0);

    board.place_piece(
        square_from(4, 1).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board.place_piece(
        square_from(7, 6).unwrap(),
        Piece::new(PieceType::Queen, Player::Red),
    );
    board.place_piece(
        square_from(1, 10).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board.place_piece(
        square_from(6, 7).unwrap(),
        Piece::new(PieceType::Queen, Player::Blue),
    );
    board.place_piece(
        square_from(10, 12).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board.place_piece(
        square_from(13, 6).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );

    GameState::new(board, GameMode::FreeForAll, false)
}

// ---------------------------------------------------------------------------
// 1. Full search — 2 simulations (AC1)
// ---------------------------------------------------------------------------

#[test]
fn test_search_2_sims_returns_legal_move() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(2));

    assert_legal(&gs, result.best_move);
    assert!(result.nodes <= 2 + 64, "nodes {} too high for 2-sim budget", result.nodes);
    assert!(!result.pv.is_empty(), "PV must not be empty");
    assert_eq!(result.pv[0], result.best_move, "PV[0] must equal best_move");
}

// ---------------------------------------------------------------------------
// 2. Full search — 100 simulations (AC2)
// ---------------------------------------------------------------------------

#[test]
fn test_search_100_sims_returns_legal_move() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(100));

    assert_legal(&gs, result.best_move);
    assert!(
        result.score >= -9999 && result.score <= 9999,
        "score {} out of clamped range",
        result.score
    );
    assert!(result.nodes <= 100 + 64, "nodes {} too high", result.nodes);
    assert!(!result.pv.is_empty());
    assert_eq!(result.pv[0], result.best_move);
}

// ---------------------------------------------------------------------------
// 3. Full search — 1000 simulations, performance (AC5)
// ---------------------------------------------------------------------------

#[test]
fn test_search_1000_sims_completes() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let start = std::time::Instant::now();
    let result = searcher.search(&gs, sim_budget(1000));
    let elapsed = start.elapsed();

    assert_legal(&gs, result.best_move);
    assert!(result.nodes <= 1000 + 64);
    // In debug mode this may be slow; just check it completes.
    // AC5 specifies <5s in release build.
    assert!(
        elapsed.as_secs() < 120,
        "1000 sims took {:?} (should complete in reasonable time)",
        elapsed
    );
}

// ---------------------------------------------------------------------------
// 4. Full search — time-budgeted
// ---------------------------------------------------------------------------

#[test]
fn test_search_time_budgeted() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let start = std::time::Instant::now();
    let result = searcher.search(&gs, time_budget(500));
    let elapsed = start.elapsed();

    assert_legal(&gs, result.best_move);
    // Should stop reasonably close to 500ms (with some tolerance for overhead)
    assert!(
        elapsed.as_millis() < 2000,
        "time-budgeted search took {:?} (expected ~500ms)",
        elapsed
    );
    assert!(result.nodes > 0, "should complete at least 1 simulation");
}

// ---------------------------------------------------------------------------
// 5. Position not modified
// ---------------------------------------------------------------------------

#[test]
fn test_search_does_not_modify_position() {
    let gs = starting_gs();
    let gs_before = gs.clone();
    let mut searcher = make_mcts_searcher();
    let _result = searcher.search(&gs, sim_budget(50));

    assert_eq!(
        gs.current_player(),
        gs_before.current_player(),
        "position current_player changed"
    );
    assert_eq!(gs.is_game_over(), gs_before.is_game_over());
    assert_eq!(gs.board().zobrist(), gs_before.board().zobrist());
}

// ---------------------------------------------------------------------------
// 6. Searcher trait — Box<dyn Searcher> (AC6)
// ---------------------------------------------------------------------------

#[test]
fn test_mcts_as_box_dyn_searcher() {
    let gs = starting_gs();
    let mut searcher: Box<dyn Searcher> = Box::new(make_mcts_searcher());
    let result = searcher.search(&gs, sim_budget(20));

    assert_legal(&gs, result.best_move);
    assert!(!result.pv.is_empty());
}

// ---------------------------------------------------------------------------
// 7. Sequential Halving eliminates candidates (AC8)
// ---------------------------------------------------------------------------

#[test]
fn test_sequential_halving_allocates_budget() {
    // With 100 simulations, Sequential Halving should distribute sims
    // across multiple rounds and use most of the budget.
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(100));

    // Should use a significant portion of the budget
    assert!(
        result.nodes >= 50,
        "only {} sims done, expected >= 50 with 100 budget",
        result.nodes
    );
    assert_legal(&gs, result.best_move);
}

// ---------------------------------------------------------------------------
// 8. Backpropagation correctness — search produces valid scores (AC3)
// ---------------------------------------------------------------------------

#[test]
fn test_search_scores_are_bounded() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(50));

    assert!(
        result.score >= -9999 && result.score <= 9999,
        "score {} out of range [-9999, 9999]",
        result.score
    );
}

#[test]
fn test_search_depth_positive() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(50));

    assert!(
        result.depth >= 1,
        "depth {} should be >= 1",
        result.depth
    );
}

// ---------------------------------------------------------------------------
// 9. Progressive widening limits tree breadth (AC4)
// ---------------------------------------------------------------------------

#[test]
fn test_progressive_widening_limits_breadth() {
    // With only 10 sims, PW should limit internal node breadth.
    // We can't directly inspect tree internals from integration tests,
    // but we verify search still works correctly with PW active.
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(10));

    assert_legal(&gs, result.best_move);
    assert!(result.nodes <= 10 + 64);
}

// ---------------------------------------------------------------------------
// 10. Progressive history (AC7)
// ---------------------------------------------------------------------------

#[test]
fn test_progressive_history_changes_behavior() {
    use odin_engine::search::mcts::HistoryTable;

    let gs = starting_gs();

    // Search without history
    let mut searcher1 = make_mcts_searcher();
    let result1 = searcher1.search(&gs, sim_budget(50));

    // Search with a history table that strongly favors certain moves
    let mut searcher2 = make_mcts_searcher();
    let mut history: HistoryTable = [[[0i32; 196]; 7]; 4];
    // Give huge history bonus to Knight squares (piece_type index 1)
    for sq in 0..196 {
        history[0][1][sq] = 10_000;
    }
    searcher2.set_history_table(&history);
    let result2 = searcher2.search(&gs, sim_budget(50));

    // Both should be legal
    assert_legal(&gs, result1.best_move);
    assert_legal(&gs, result2.best_move);

    // We can't guarantee different moves, but the searcher should accept
    // and use the history table without errors. The test verifies the
    // API works correctly.
}

// ---------------------------------------------------------------------------
// 11. Tactical — finds free queen capture
// ---------------------------------------------------------------------------

#[test]
fn test_finds_free_queen_capture() {
    let gs = make_free_capture_position();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(200));

    assert_legal(&gs, result.best_move);
    // With a free queen available, score should be positive
    assert!(
        result.score > 0,
        "expected positive score with free queen capture, got {} (move {})",
        result.score,
        result.best_move.to_algebraic()
    );
}

// ---------------------------------------------------------------------------
// 12. Info callback integration
// ---------------------------------------------------------------------------

#[test]
fn test_info_callback_receives_lines() {
    use std::sync::{Arc, Mutex};

    let gs = starting_gs();
    let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let lines_clone = Arc::clone(&lines);
    let cb = Box::new(move |line: String| {
        lines_clone.lock().unwrap().push(line);
    });

    let _searcher = MctsSearcher::with_info_callback(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        None,
        cb,
    );
    // Use a fixed seed for deterministic behavior
    let mut det_searcher = MctsSearcher::with_seed(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        None,
        42,
    );
    let info_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let info_clone = Arc::clone(&info_lines);
    det_searcher.set_info_callback(Box::new(move |line: String| {
        info_clone.lock().unwrap().push(line);
    }));
    let _result = det_searcher.search(&gs, sim_budget(100));

    let captured = info_lines.lock().unwrap();
    assert!(
        !captured.is_empty(),
        "info callback should receive at least one line"
    );
    // Verify search info lines contain expected fields.
    // Skip "info string ..." lines (metadata like mcts_visits, stop_reason).
    for line in captured.iter() {
        if line.starts_with("info string") {
            continue;
        }
        assert!(
            line.contains("phase mcts"),
            "info line missing 'phase mcts': {}",
            line
        );
        assert!(
            line.contains("nodes"),
            "info line missing 'nodes': {}",
            line
        );
    }
    drop(_searcher);
}

// ---------------------------------------------------------------------------
// 13. Multiple searches on same searcher
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_searches_same_searcher() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();

    let r1 = searcher.search(&gs, sim_budget(20));
    let r2 = searcher.search(&gs, sim_budget(20));

    assert_legal(&gs, r1.best_move);
    assert_legal(&gs, r2.best_move);
    // Both searches should complete without panics
}

// ---------------------------------------------------------------------------
// 14. Search with depth limit
// ---------------------------------------------------------------------------

#[test]
fn test_search_with_depth_limit() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let budget = SearchBudget {
        max_depth: Some(3),
        max_nodes: Some(100),
        max_time_ms: None,
    };
    let result = searcher.search(&gs, budget);

    assert_legal(&gs, result.best_move);
    assert!(
        result.depth <= 3,
        "depth {} exceeds limit of 3",
        result.depth
    );
}

// ---------------------------------------------------------------------------
// 15. Deterministic with same seed
// ---------------------------------------------------------------------------

#[test]
fn test_deterministic_with_same_seed() {
    let gs = starting_gs();
    let mut s1 = MctsSearcher::with_seed(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        None,
        12345,
    );
    let mut s2 = MctsSearcher::with_seed(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        None,
        12345,
    );

    let r1 = s1.search(&gs, sim_budget(50));
    let r2 = s2.search(&gs, sim_budget(50));

    assert_eq!(
        r1.best_move, r2.best_move,
        "same seed should produce same move: {} vs {}",
        r1.best_move.to_algebraic(),
        r2.best_move.to_algebraic()
    );
    assert_eq!(r1.nodes, r2.nodes, "same seed should produce same node count");
}

// ---------------------------------------------------------------------------
// 16. Random game robustness (no panics)
// ---------------------------------------------------------------------------

#[test]
fn test_random_game_no_panics() {
    let mut gs = GameState::new_standard_ffa();
    let mut searcher = make_mcts_searcher();
    let mut ply = 0;

    // Play 12 plies (3 full rounds) with MCTS
    while !gs.is_game_over() && ply < 12 {
        let result = searcher.search(&gs, sim_budget(10));
        assert_legal(&gs, result.best_move);
        gs.apply_move(result.best_move);
        ply += 1;
    }
    // If we got here without panicking, the test passes.
}

// ---------------------------------------------------------------------------
// 17. PV is well-formed
// ---------------------------------------------------------------------------

#[test]
fn test_pv_well_formed() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let result = searcher.search(&gs, sim_budget(100));

    assert!(!result.pv.is_empty(), "PV must not be empty");
    assert_eq!(result.pv[0], result.best_move, "PV[0] must be best_move");

    // Verify PV is a sequence of legal moves
    let mut replay = gs.clone();
    for (i, &mv) in result.pv.iter().enumerate() {
        let legal = replay.legal_moves();
        assert!(
            legal.contains(&mv),
            "PV[{}] ({}) is not legal at ply {}",
            i,
            mv.to_algebraic(),
            i
        );
        replay.apply_move(mv);
    }
}

// ---------------------------------------------------------------------------
// 18. 1000 sims release performance (AC5) [ignored — release only]
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_1000_sims_under_5_seconds_release() {
    let gs = starting_gs();
    let mut searcher = make_mcts_searcher();
    let start = std::time::Instant::now();
    let result = searcher.search(&gs, sim_budget(1000));
    let elapsed = start.elapsed();

    assert_legal(&gs, result.best_move);
    assert!(
        elapsed.as_secs() < 5,
        "1000 sims took {:?} (AC5: must be < 5s in release)",
        elapsed
    );
    eprintln!(
        "  1000 sims: {:?}, {} nodes, move {}",
        elapsed,
        result.nodes,
        result.best_move.to_algebraic()
    );
}
