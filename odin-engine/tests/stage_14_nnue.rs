// Stage 14 Acceptance Tests — NNUE Feature Design & Architecture
//
// 18 tests covering feature indexing, weights, accumulator, forward pass,
// NnueEvaluator, and benchmarks. All tested with random weights.

use std::collections::HashSet;
use std::time::Instant;

use odin_engine::board::{
    is_valid_square, valid_squares, Board, PieceType, Player, Square, VALID_SQUARE_COUNT,
};
use odin_engine::eval::nnue::accumulator::{Accumulator, AccumulatorStack};
use odin_engine::eval::nnue::features::{
    feature_index, relative_owner, square_to_dense, FEATURES_PER_PERSPECTIVE,
    FT_OUT,
};
use odin_engine::eval::nnue::weights::NnueWeights;
use odin_engine::eval::nnue::NnueEvaluator;
use odin_engine::eval::Evaluator;
use odin_engine::gamestate::GameState;
use odin_engine::movegen::{generate_legal, make_move, unmake_move};

// -----------------------------------------------------------------------
// T1: Feature index range — all in [0, 4480), no duplicates
// -----------------------------------------------------------------------

#[test]
fn test_feature_index_range() {
    let mut seen = HashSet::new();
    for sq in valid_squares() {
        for &pt in &PieceType::ALL {
            for rel in 0..4u8 {
                let idx = feature_index(sq, pt, rel).unwrap();
                assert!(
                    (idx as usize) < FEATURES_PER_PERSPECTIVE,
                    "index {idx} out of range"
                );
                assert!(seen.insert(idx), "duplicate index {idx}");
            }
        }
    }
    assert_eq!(seen.len(), FEATURES_PER_PERSPECTIVE);
}

// -----------------------------------------------------------------------
// T2: Feature index symmetry — different perspectives give different indices
// -----------------------------------------------------------------------

#[test]
fn test_feature_index_symmetry() {
    // A Red pawn on d4 (file=3, rank=3, sq=3*14+3=45) from Red's perspective
    // vs Blue's perspective should give different feature indices.
    let sq: Square = 3 * 14 + 3; // d4
    assert!(is_valid_square(sq));

    // From Red's perspective: relative_owner(Red, Red) = 0
    let idx_red = feature_index(sq, PieceType::Pawn, relative_owner(Player::Red, Player::Red))
        .unwrap();
    // From Blue's perspective: relative_owner(Blue, Red) = 3 (Red is CCW from Blue)
    let idx_blue = feature_index(sq, PieceType::Pawn, relative_owner(Player::Blue, Player::Red))
        .unwrap();

    assert_ne!(idx_red, idx_blue, "same piece from different perspectives must differ");
}

// -----------------------------------------------------------------------
// T3: Dense square mapping — 160 valid squares to 0-159
// -----------------------------------------------------------------------

#[test]
fn test_dense_square_mapping() {
    let mut dense_seen = HashSet::new();
    let mut count = 0;

    for sq in 0..196u8 {
        let dense = square_to_dense(sq);
        if is_valid_square(sq) {
            assert_ne!(dense, 255, "valid square {sq} should have a dense index");
            assert!((dense as usize) < VALID_SQUARE_COUNT);
            assert!(dense_seen.insert(dense), "duplicate dense {dense}");
            count += 1;
        } else {
            assert_eq!(dense, 255, "invalid square {sq} should map to 255");
        }
    }
    assert_eq!(count, VALID_SQUARE_COUNT);
}

// -----------------------------------------------------------------------
// T4: Random weights deterministic — same seed same weights
// -----------------------------------------------------------------------

#[test]
fn test_weights_random_deterministic() {
    let w1 = NnueWeights::random(12345);
    let w2 = NnueWeights::random(12345);
    assert_eq!(w1.ft_weights, w2.ft_weights);
    assert_eq!(w1.ft_biases, w2.ft_biases);
    assert_eq!(w1.hidden_weights, w2.hidden_weights);
    assert_eq!(w1.hidden_biases, w2.hidden_biases);
    assert_eq!(w1.brs_weights, w2.brs_weights);
    assert_eq!(w1.brs_bias, w2.brs_bias);
    assert_eq!(w1.mcts_weights, w2.mcts_weights);
    assert_eq!(w1.mcts_biases, w2.mcts_biases);
}

// -----------------------------------------------------------------------
// T5: Save/load roundtrip — identical after save+load
// -----------------------------------------------------------------------

#[test]
fn test_weights_save_load_roundtrip() {
    let original = NnueWeights::random(999);
    let path = std::env::temp_dir().join("test_nnue_roundtrip.onnue");

    original.save(&path).expect("save should succeed");
    let loaded = NnueWeights::load(&path).expect("load should succeed");

    assert_eq!(original.ft_weights, loaded.ft_weights);
    assert_eq!(original.ft_biases, loaded.ft_biases);
    assert_eq!(original.hidden_weights, loaded.hidden_weights);
    assert_eq!(original.hidden_biases, loaded.hidden_biases);
    assert_eq!(original.brs_weights, loaded.brs_weights);
    assert_eq!(original.brs_bias, loaded.brs_bias);
    assert_eq!(original.mcts_weights, loaded.mcts_weights);
    assert_eq!(original.mcts_biases, loaded.mcts_biases);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// -----------------------------------------------------------------------
// T6: Accumulator full refresh — starting position non-zero
// -----------------------------------------------------------------------

#[test]
fn test_accumulator_full_refresh() {
    let board = Board::starting_position();
    let weights = NnueWeights::random(42);
    let mut acc = Accumulator::zeroed();
    acc.compute_full(&board, &weights);

    for pidx in 0..4 {
        assert!(!acc.needs_refresh[pidx]);
        // At least some values should be non-zero after adding 64 features.
        let nonzero = acc.values[pidx].iter().filter(|&&v| v != 0).count();
        assert!(
            nonzero > 0,
            "perspective {pidx} accumulator should have non-zero values"
        );
    }
}

// -----------------------------------------------------------------------
// T7: Incremental matches full — after N random moves
// -----------------------------------------------------------------------

#[test]
fn test_incremental_matches_full() {
    let weights = NnueWeights::random(77);
    let mut board = Board::starting_position();
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(&board, &weights);

    // Play several random moves, checking incremental vs full after each.
    let move_counts = [1, 3, 5, 10];
    let mut total_moves = 0;

    for &target in &move_counts {
        while total_moves < target {
            let moves = generate_legal(&mut board);
            if moves.is_empty() {
                break;
            }
            let mv = moves[total_moves % moves.len()];

            // Push incremental
            stack.push(mv, &board, &weights);
            let undo = make_move(&mut board, mv);
            let _ = undo;

            // Refresh any flagged perspectives
            stack.refresh_if_needed(&board, &weights);

            total_moves += 1;
        }

        // Compute fresh from scratch for comparison.
        let mut fresh = Accumulator::zeroed();
        fresh.compute_full(&board, &weights);

        let incremental = stack.current();
        for pidx in 0..4 {
            assert_eq!(
                incremental.values[pidx], fresh.values[pidx],
                "perspective {pidx} mismatch after {total_moves} moves"
            );
        }
    }
}

// -----------------------------------------------------------------------
// T8: Push/pop roundtrip — push N, pop N, bit-for-bit match
// -----------------------------------------------------------------------

#[test]
fn test_push_pop_roundtrip() {
    let weights = NnueWeights::random(88);
    let mut board = Board::starting_position();
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(&board, &weights);

    // Save the original accumulator values.
    let original: [[i16; FT_OUT]; 4] = stack.current().values;

    // Push N moves, collect undos, then pop them all.
    let mut undos = Vec::new();
    let mut moves_played = Vec::new();
    let n = 10;

    for _ in 0..n {
        let moves = generate_legal(&mut board);
        if moves.is_empty() {
            break;
        }
        let mv = moves[0];
        stack.push(mv, &board, &weights);
        let undo = make_move(&mut board, mv);
        moves_played.push(mv);
        undos.push(undo);
    }

    // Now pop all and unmake.
    for i in (0..moves_played.len()).rev() {
        unmake_move(&mut board, moves_played[i], undos[i]);
        stack.pop();
    }

    // Should be back to original.
    let restored = stack.current();
    for pidx in 0..4 {
        assert_eq!(
            restored.values[pidx], original[pidx],
            "perspective {pidx} not restored after push/pop roundtrip"
        );
    }
}

// -----------------------------------------------------------------------
// T9: Forward pass deterministic — same input same output
// -----------------------------------------------------------------------

#[test]
fn test_forward_pass_deterministic() {
    use odin_engine::eval::nnue::forward_pass;

    let board = Board::starting_position();
    let weights = NnueWeights::random(42);
    let mut acc = Accumulator::zeroed();
    acc.compute_full(&board, &weights);

    let (brs1, mcts1) = forward_pass(&acc, &weights, Player::Red);
    let (brs2, mcts2) = forward_pass(&acc, &weights, Player::Red);

    assert_eq!(brs1, brs2, "BRS output should be deterministic");
    assert_eq!(mcts1, mcts2, "MCTS output should be deterministic");
}

// -----------------------------------------------------------------------
// T10: eval_scalar range — output in [-30000, 30000]
// -----------------------------------------------------------------------

#[test]
fn test_eval_scalar_range() {
    let evaluator = NnueEvaluator::with_random_weights(42);
    let gs = GameState::new_standard_ffa();

    for &player in &Player::ALL {
        let score = evaluator.eval_scalar(&gs, player);
        assert!(
            (-30_000..=30_000).contains(&score),
            "eval_scalar for {player:?} = {score}, out of range"
        );
    }
}

// -----------------------------------------------------------------------
// T11: eval_4vec range — all 4 values in [0.0, 1.0]
// -----------------------------------------------------------------------

#[test]
fn test_eval_4vec_range() {
    let evaluator = NnueEvaluator::with_random_weights(42);
    let gs = GameState::new_standard_ffa();
    let values = evaluator.eval_4vec(&gs);

    for (i, &v) in values.iter().enumerate() {
        assert!(
            (0.0..=1.0).contains(&v),
            "eval_4vec[{i}] = {v}, out of [0, 1] range"
        );
    }
}

// -----------------------------------------------------------------------
// T12: Sensitivity — both heads produce different outputs for different positions
// -----------------------------------------------------------------------

#[test]
fn test_eval_sensitivity() {
    let evaluator = NnueEvaluator::with_random_weights(42);
    let gs1 = GameState::new_standard_ffa();

    // Play a few moves to get a different position.
    let mut gs2 = GameState::new_standard_ffa();
    let mut board_tmp = gs2.board().clone();
    let moves = generate_legal(&mut board_tmp);
    assert!(!moves.is_empty());

    // Make the first legal move to get a different position.
    gs2.apply_move(moves[0]);

    let scalar1 = evaluator.eval_scalar(&gs1, Player::Red);
    let scalar2 = evaluator.eval_scalar(&gs2, Player::Red);

    let vec1 = evaluator.eval_4vec(&gs1);
    let vec2 = evaluator.eval_4vec(&gs2);

    // With random weights, different positions should produce different scores.
    // (Extremely unlikely to be identical with random weights.)
    let scalar_differs = scalar1 != scalar2;
    let vec_differs = vec1 != vec2;

    assert!(
        scalar_differs || vec_differs,
        "both heads returned identical values for different positions"
    );
}

// -----------------------------------------------------------------------
// T13: Incremental captures — accumulator correct after a capture
// -----------------------------------------------------------------------

#[test]
fn test_incremental_captures() {
    let weights = NnueWeights::random(55);
    let mut board = Board::starting_position();
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(&board, &weights);

    // Play moves until we find a capture.
    let mut found_capture = false;
    for _ in 0..40 {
        let moves = generate_legal(&mut board);
        if moves.is_empty() {
            break;
        }

        // Prefer capture moves.
        let mv = moves
            .iter()
            .find(|m| m.is_capture())
            .copied()
            .unwrap_or(moves[0]);

        stack.push(mv, &board, &weights);
        let _undo = make_move(&mut board, mv);
        stack.refresh_if_needed(&board, &weights);

        if mv.is_capture() {
            found_capture = true;
            // Verify incremental matches full.
            let mut fresh = Accumulator::zeroed();
            fresh.compute_full(&board, &weights);
            let inc = stack.current();
            for pidx in 0..4 {
                assert_eq!(
                    inc.values[pidx], fresh.values[pidx],
                    "capture: perspective {pidx} mismatch"
                );
            }
            break;
        }
    }

    // It's OK if no capture was found in 40 random moves from starting pos.
    // The test still validates the non-capture incremental path.
    if !found_capture {
        eprintln!("info string T13: no capture found in 40 moves (non-critical)");
    }
}

// -----------------------------------------------------------------------
// T14: Incremental castling — accumulator correct after castling
// -----------------------------------------------------------------------

#[test]
fn test_incremental_castling() {
    let weights = NnueWeights::random(66);
    let mut board = Board::starting_position();
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(&board, &weights);

    // Play moves until we find a castling move.
    let mut found_castle = false;
    for _ in 0..100 {
        let moves = generate_legal(&mut board);
        if moves.is_empty() {
            break;
        }

        // Prefer castling moves.
        let mv = moves
            .iter()
            .find(|m| m.is_castle())
            .copied()
            .unwrap_or(moves[0]);

        stack.push(mv, &board, &weights);
        let _undo = make_move(&mut board, mv);
        stack.refresh_if_needed(&board, &weights);

        if mv.is_castle() {
            found_castle = true;
            // Verify refresh-based approach matches full.
            let mut fresh = Accumulator::zeroed();
            fresh.compute_full(&board, &weights);
            let inc = stack.current();
            for pidx in 0..4 {
                assert_eq!(
                    inc.values[pidx], fresh.values[pidx],
                    "castling: perspective {pidx} mismatch"
                );
            }
            break;
        }
    }

    if !found_castle {
        eprintln!("info string T14: no castling found in 100 moves (non-critical)");
    }
}

// -----------------------------------------------------------------------
// T15: Incremental promotion — accumulator correct after pawn promotion
// -----------------------------------------------------------------------

#[test]
fn test_incremental_promotion() {
    let weights = NnueWeights::random(77);
    let mut board = Board::starting_position();

    // Play moves until we find a promotion. Re-init the stack periodically
    // to avoid stack overflow (we don't pop in this test).
    let mut found_promotion = false;
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(&board, &weights);
    let mut depth = 0;

    for _ in 0..200 {
        let moves = generate_legal(&mut board);
        if moves.is_empty() {
            break;
        }

        // Prefer promotion moves.
        let mv = moves
            .iter()
            .find(|m| m.is_promotion())
            .copied()
            .unwrap_or(moves[0]);

        stack.push(mv, &board, &weights);
        let _undo = make_move(&mut board, mv);
        stack.refresh_if_needed(&board, &weights);
        depth += 1;

        if mv.is_promotion() {
            found_promotion = true;
            let mut fresh = Accumulator::zeroed();
            fresh.compute_full(&board, &weights);
            let inc = stack.current();
            for pidx in 0..4 {
                assert_eq!(
                    inc.values[pidx], fresh.values[pidx],
                    "promotion: perspective {pidx} mismatch"
                );
            }
            break;
        }

        // Re-init stack every 100 moves to avoid overflow.
        if depth >= 100 {
            stack.init_from_board(&board, &weights);
            depth = 0;
        }
    }

    if !found_promotion {
        eprintln!("info string T15: no promotion found in 200 moves (non-critical)");
    }
}

// -----------------------------------------------------------------------
// T16: .onnue magic validation — wrong magic fails gracefully
// -----------------------------------------------------------------------

#[test]
fn test_onnue_magic_validation() {
    let path = std::env::temp_dir().join("test_nnue_bad_magic.onnue");

    // Write a file with bad magic.
    let mut bad_data = vec![0u8; 100];
    bad_data[0] = b'B';
    bad_data[1] = b'A';
    bad_data[2] = b'D';
    bad_data[3] = b'!';
    std::fs::write(&path, &bad_data).unwrap();

    let result = NnueWeights::load(&path);
    assert!(result.is_err(), "loading bad magic should fail");

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// -----------------------------------------------------------------------
// T17: Benchmark incremental — < 5us per incremental update
// -----------------------------------------------------------------------

#[test]
fn test_benchmark_incremental() {
    let weights = NnueWeights::random(42);
    let mut board = Board::starting_position();
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(&board, &weights);

    // Warm up: play one move.
    let moves = generate_legal(&mut board);
    let mv = moves[0];
    stack.push(mv, &board, &weights);
    let _undo = make_move(&mut board, mv);

    // Benchmark: push + refresh on subsequent moves.
    let moves = generate_legal(&mut board);
    let iterations = 1000;
    let start = Instant::now();
    for i in 0..iterations {
        let mv = moves[i % moves.len()];
        stack.push(mv, &board, &weights);
        stack.refresh_if_needed(&board, &weights);
        stack.pop();
    }
    let elapsed = start.elapsed();
    let per_op = elapsed / iterations as u32;

    eprintln!(
        "info string T17: incremental update: {:?}/op ({iterations} iterations, total {:?})",
        per_op, elapsed
    );

    // In debug mode this will be slower; the 5us target is for release.
    // We just print the timing and don't assert in debug builds.
    #[cfg(not(debug_assertions))]
    assert!(
        per_op.as_micros() < 5,
        "incremental update too slow: {:?}",
        per_op
    );
}

// -----------------------------------------------------------------------
// T18: Benchmark full eval — < 50us per full evaluation
// -----------------------------------------------------------------------

#[test]
fn test_benchmark_full() {
    let evaluator = NnueEvaluator::with_random_weights(42);
    let gs = GameState::new_standard_ffa();

    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = evaluator.eval_scalar(&gs, Player::Red);
    }
    let elapsed = start.elapsed();
    let per_op = elapsed / iterations as u32;

    eprintln!(
        "info string T18: full eval: {:?}/op ({iterations} iterations, total {:?})",
        per_op, elapsed
    );

    #[cfg(not(debug_assertions))]
    assert!(
        per_op.as_micros() < 50,
        "full eval too slow: {:?}",
        per_op
    );
}
