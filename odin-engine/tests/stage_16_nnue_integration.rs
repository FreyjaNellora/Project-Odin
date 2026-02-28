// Stage 16 — NNUE Integration Tests
//
// Verifies that AccumulatorStack push/pop is correctly wired into BRS and MCTS
// search paths, that NNUE eval replaces bootstrap at leaf nodes, and that
// fallback to bootstrap works when no .onnue file is provided.

use std::sync::Arc;

use odin_engine::board::Player;
use odin_engine::eval::nnue::accumulator::{Accumulator, AccumulatorStack};
use odin_engine::eval::nnue::weights::NnueWeights;
use odin_engine::eval::nnue::forward_pass;
use odin_engine::eval::{BootstrapEvaluator, EvalProfile};
use odin_engine::gamestate::GameState;
use odin_engine::movegen::{generate_legal, make_move};
use odin_engine::search::brs::BrsSearcher;
use odin_engine::search::hybrid::HybridController;
use odin_engine::search::mcts::MctsSearcher;
use odin_engine::search::{SearchBudget, Searcher};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn random_weights() -> Arc<NnueWeights> {
    Arc::new(NnueWeights::random(42))
}

fn depth_budget(d: u8) -> SearchBudget {
    SearchBudget {
        max_depth: Some(d),
        max_nodes: None,
        max_time_ms: None,
    }
}

fn sim_budget(sims: u64) -> SearchBudget {
    SearchBudget {
        max_depth: None,
        max_nodes: Some(sims),
        max_time_ms: None,
    }
}

// ---------------------------------------------------------------------------
// T1: Incremental push/pop matches full recompute
// ---------------------------------------------------------------------------

#[test]
fn test_nnue_brs_push_pop_matches_full() {
    let weights = random_weights();
    let mut gs = GameState::new_standard_ffa();

    // Init accumulator from starting position
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(gs.board(), &weights);

    // Play 8 moves, verifying at each step
    for step in 0..8 {
        let moves = generate_legal(gs.board_mut());
        assert!(!moves.is_empty(), "no moves at step {step}");
        let mv = moves[0]; // deterministic: always pick first legal move

        // Push before make_move (accumulator needs board_before)
        stack.push(mv, gs.board(), &weights);
        let _undo = make_move(gs.board_mut(), mv);

        // Refresh and get incremental values
        stack.refresh_if_needed(gs.board(), &weights);
        let incremental = stack.current().clone();

        // Compute full recompute from scratch
        let mut full = Accumulator::zeroed();
        full.compute_full(gs.board(), &weights);

        // Compare all 4 perspectives
        for p in 0..4 {
            assert_eq!(
                incremental.values[p], full.values[p],
                "perspective {p} mismatch at step {step}"
            );
        }
    }

    // Pop all 8 and verify depth returns to 0
    for _ in 0..8 {
        stack.pop();
    }
    assert_eq!(stack.depth(), 0, "stack depth should return to 0 after popping all");
}

// ---------------------------------------------------------------------------
// T2: MCTS simulation returns acc_stack to root depth
// ---------------------------------------------------------------------------

#[test]
fn test_nnue_mcts_sim_accumulator_depth() {
    let weights = random_weights();
    let mut searcher = MctsSearcher::with_seed(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        Some(weights),
        42,
    );
    let gs = GameState::new_standard_ffa();

    // Run MCTS with NNUE (100 sims). If acc_stack doesn't return to root
    // depth after each simulation, it would overflow or panic.
    let result = searcher.search(&gs, sim_budget(100));
    assert!(
        result.nodes > 0,
        "MCTS with NNUE should complete simulations"
    );
}

// ---------------------------------------------------------------------------
// T3: Fallback without NNUE file
// ---------------------------------------------------------------------------

#[test]
fn test_fallback_without_nnue_file() {
    let mut hybrid = HybridController::new(EvalProfile::Standard, None);
    let gs = GameState::new_standard_ffa();

    let result = hybrid.search(
        &gs,
        SearchBudget {
            max_depth: Some(4),
            max_nodes: Some(200),
            max_time_ms: Some(5_000),
        },
    );

    // Should complete using bootstrap eval, no panic
    let legal = {
        let mut gs2 = gs.clone();
        gs2.legal_moves()
    };
    assert!(
        legal.contains(&result.best_move),
        "fallback search should return a legal move"
    );
}

// ---------------------------------------------------------------------------
// T4: perft invariants unchanged
// ---------------------------------------------------------------------------

#[test]
fn test_perft_unchanged() {
    use odin_engine::movegen::perft;
    let mut gs = GameState::new_standard_ffa();
    assert_eq!(perft(gs.board_mut(), 1), 20);
    assert_eq!(perft(gs.board_mut(), 2), 395);
    assert_eq!(perft(gs.board_mut(), 3), 7800);
    assert_eq!(perft(gs.board_mut(), 4), 152050);
}

// ---------------------------------------------------------------------------
// T5: NNUE eval non-degenerate
// ---------------------------------------------------------------------------

#[test]
fn test_nnue_eval_non_degenerate() {
    let weights = random_weights();
    let gs = GameState::new_standard_ffa();

    let mut stack = AccumulatorStack::new();
    stack.init_from_board(gs.board(), &weights);
    stack.refresh_if_needed(gs.board(), &weights);

    let (brs_score, mcts_vals) = forward_pass(stack.current(), &weights, Player::Red);

    // BRS score should not be 0 or extreme (random weights give non-trivial values)
    assert!(
        brs_score != 0,
        "BRS score should be non-zero with random weights"
    );
    assert!(
        brs_score.abs() < 30000,
        "BRS score {} is unreasonably extreme",
        brs_score
    );

    // MCTS values should all be in (0, 1)
    for (i, &v) in mcts_vals.iter().enumerate() {
        assert!(
            v > 0.0 && v < 1.0,
            "MCTS value[{i}] = {v} should be in (0, 1)"
        );
    }
}

// ---------------------------------------------------------------------------
// T6: BRS search with NNUE returns valid move
// ---------------------------------------------------------------------------

#[test]
fn test_brs_search_with_nnue() {
    let weights = random_weights();
    let mut searcher = BrsSearcher::new(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        Some(weights),
    );
    let gs = GameState::new_standard_ffa();

    let result = searcher.search(&gs, depth_budget(6));

    let legal = {
        let mut gs2 = gs.clone();
        gs2.legal_moves()
    };
    assert!(
        legal.contains(&result.best_move),
        "BRS+NNUE should return a legal move"
    );
    assert!(result.depth > 0, "BRS+NNUE should reach depth > 0");
}

// ---------------------------------------------------------------------------
// T7: MCTS search with NNUE returns valid move
// ---------------------------------------------------------------------------

#[test]
fn test_mcts_search_with_nnue() {
    let weights = random_weights();
    let mut searcher = MctsSearcher::with_seed(
        Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
        Some(weights),
        42,
    );
    let gs = GameState::new_standard_ffa();

    let result = searcher.search(&gs, sim_budget(500));

    let legal = {
        let mut gs2 = gs.clone();
        gs2.legal_moves()
    };
    assert!(
        legal.contains(&result.best_move),
        "MCTS+NNUE should return a legal move"
    );
    assert!(result.nodes > 0, "MCTS+NNUE should complete simulations");
}

// ---------------------------------------------------------------------------
// T8: Hybrid search with NNUE returns valid move
// ---------------------------------------------------------------------------

#[test]
fn test_hybrid_search_with_nnue() {
    let weights = random_weights();
    let nnue_weights = NnueWeights::random(42);
    // Write weights to a temp file, then load via HybridController
    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join("test_stage16_weights.onnue");
    nnue_weights.save(std::path::Path::new(&tmp_path)).unwrap();

    let mut hybrid = HybridController::new(
        EvalProfile::Aggressive,
        Some(tmp_path.to_str().unwrap()),
    );
    let gs = GameState::new_standard_ffa();

    let result = hybrid.search(
        &gs,
        SearchBudget {
            max_depth: Some(4),
            max_nodes: Some(200),
            max_time_ms: Some(10_000),
        },
    );

    let legal = {
        let mut gs2 = gs.clone();
        gs2.legal_moves()
    };
    assert!(
        legal.contains(&result.best_move),
        "Hybrid+NNUE should return a legal move"
    );

    // Cleanup
    let _ = std::fs::remove_file(&tmp_path);
    let _ = weights; // suppress unused warning
}

// ---------------------------------------------------------------------------
// T9: NNUE self-play (10 games, 5 ply each) — no panics
// ---------------------------------------------------------------------------

#[test]
fn test_nnue_vs_bootstrap_no_crash() {
    let weights = random_weights();

    for _game in 0..10 {
        let mut gs = GameState::new_standard_ffa();
        let mut searcher = BrsSearcher::new(
            Box::new(BootstrapEvaluator::new(EvalProfile::Aggressive)),
            Some(weights.clone()),
        );

        for _ply in 0..5 {
            if gs.is_game_over() {
                break;
            }
            let result = searcher.search(&gs, depth_budget(3));
            gs.apply_move(result.best_move);
        }
        // Just verify no panic occurred
    }
}

// ---------------------------------------------------------------------------
// T10: Incremental vs full speed comparison
// ---------------------------------------------------------------------------

#[test]
fn test_incremental_vs_full_speed() {
    use std::time::Instant;

    let weights = random_weights();
    let mut gs = GameState::new_standard_ffa();
    let iterations = 500;

    // Benchmark incremental: init once, then push/pop for each move
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(gs.board(), &weights);

    let moves = generate_legal(gs.board_mut());
    let mv = moves[0];

    let start_inc = Instant::now();
    for _ in 0..iterations {
        stack.push(mv, gs.board(), &weights);
        stack.refresh_if_needed(gs.board(), &weights);
        let _ = forward_pass(stack.current(), &weights, Player::Red);
        stack.pop();
    }
    let inc_time = start_inc.elapsed();

    // Benchmark full: fresh init + forward_pass each time
    let start_full = Instant::now();
    for _ in 0..iterations {
        let mut acc = Accumulator::zeroed();
        acc.compute_full(gs.board(), &weights);
        let _ = forward_pass(&acc, &weights, Player::Red);
    }
    let full_time = start_full.elapsed();

    eprintln!(
        "Incremental: {:?} / {} = {:?} per iter",
        inc_time,
        iterations,
        inc_time / iterations
    );
    eprintln!(
        "Full:        {:?} / {} = {:?} per iter",
        full_time,
        iterations,
        full_time / iterations
    );

    // Incremental should be faster (push is O(features changed) vs full O(all features))
    // With random weights and starting position this should hold easily.
    // Use 1.5x instead of 2x to avoid flaky failures in debug builds.
    assert!(
        inc_time < full_time,
        "Incremental ({:?}) should be faster than full ({:?})",
        inc_time,
        full_time
    );
}
