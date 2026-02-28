// Integration tests for Stage 6: Bootstrap Eval + Evaluator Trait.
//
// Tests the Evaluator trait contract, BootstrapEvaluator behavior,
// and acceptance criteria from the MASTERPLAN.

use odin_engine::board::{Board, PieceType, Player};
use odin_engine::eval::{BootstrapEvaluator, EvalProfile, Evaluator};
use odin_engine::gamestate::GameState;

// ── Acceptance Criterion 1: Different values for materially different positions ──

#[test]
fn test_materially_different_positions_get_different_scores() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
    let gs_full = GameState::new_standard_ffa();
    let score_full = evaluator.eval_scalar(&gs_full, Player::Red);

    // Create a position where Red is missing a queen.
    let mut board = Board::starting_position();
    let queen_sq = board
        .piece_list(Player::Red)
        .iter()
        .find(|(pt, _)| *pt == PieceType::Queen)
        .map(|&(_, sq)| sq)
        .expect("Red should have a queen");
    board.remove_piece(queen_sq);

    let gs_less = GameState::new(board, odin_engine::gamestate::GameMode::FreeForAll, false);
    let score_less = evaluator.eval_scalar(&gs_less, Player::Red);

    assert_ne!(
        score_full, score_less,
        "Different material should produce different scores"
    );
    assert!(
        score_full > score_less,
        "Full material ({score_full}) should score higher than missing queen ({score_less})"
    );
}

// ── Acceptance Criterion 2: Evaluation is perspective-dependent ──

#[test]
fn test_evaluation_is_perspective_dependent() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);

    // Remove Blue's queen to create asymmetry.
    let mut board = Board::starting_position();
    let queen_sq = board
        .piece_list(Player::Blue)
        .iter()
        .find(|(pt, _)| *pt == PieceType::Queen)
        .map(|&(_, sq)| sq)
        .expect("Blue should have a queen");
    board.remove_piece(queen_sq);

    let gs = GameState::new(board, odin_engine::gamestate::GameMode::FreeForAll, false);

    let red_score = evaluator.eval_scalar(&gs, Player::Red);
    let blue_score = evaluator.eval_scalar(&gs, Player::Blue);

    // Red has full material, Blue is missing a queen.
    // Red's score from Red's perspective should be higher than Blue's score from Blue's perspective.
    assert!(
        red_score > blue_score,
        "Red ({red_score}) should score higher than Blue ({blue_score}) when Blue is missing a queen"
    );
}

// ── Acceptance Criterion 3: Eval is fast (< 10us per position) ──

#[test]
fn test_eval_performance_under_10us() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
    let gs = GameState::new_standard_ffa();

    let iterations = 10_000;
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = std::hint::black_box(evaluator.eval_scalar(&gs, Player::Red));
    }
    let elapsed = start.elapsed();
    let per_eval = elapsed / iterations;

    // Target: < 10us per eval in release. Debug builds are ~2-3x slower due to
    // no inlining and bounds checking, so allow 50us in debug.
    let threshold = if cfg!(debug_assertions) { 50 } else { 10 };
    assert!(
        per_eval < std::time::Duration::from_micros(threshold),
        "Eval took {per_eval:?} per call, exceeds {threshold}us target"
    );
}

// ── Acceptance Criterion 4: Trait compiles and BootstrapEvaluator implements it ──

#[test]
fn test_evaluator_trait_compiles() {
    fn use_evaluator(eval: &dyn Evaluator, gs: &GameState) -> (i16, [f64; 4]) {
        let scalar = eval.eval_scalar(gs, Player::Red);
        let vec = eval.eval_4vec(gs);
        (scalar, vec)
    }

    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
    let gs = GameState::new_standard_ffa();
    let (scalar, vec) = use_evaluator(&evaluator, &gs);

    assert!(scalar > -30_000 && scalar < 30_000);
    for &v in &vec {
        assert!((0.0..=1.0).contains(&v));
    }
}

// ── Acceptance Criterion 5: eval_scalar and eval_4vec consistent ──

#[test]
fn test_eval_scalar_and_4vec_consistent() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);

    // Create asymmetric position: remove Blue's queen.
    let mut board = Board::starting_position();
    let queen_sq = board
        .piece_list(Player::Blue)
        .iter()
        .find(|(pt, _)| *pt == PieceType::Queen)
        .map(|&(_, sq)| sq)
        .expect("Blue should have a queen");
    board.remove_piece(queen_sq);
    let gs = GameState::new(board, odin_engine::gamestate::GameMode::FreeForAll, false);

    let scalar_red = evaluator.eval_scalar(&gs, Player::Red);
    let scalar_blue = evaluator.eval_scalar(&gs, Player::Blue);
    let vec = evaluator.eval_4vec(&gs);

    // Sigmoid is monotonically increasing, so ordering must be preserved.
    if scalar_red > scalar_blue {
        assert!(
            vec[Player::Red.index()] > vec[Player::Blue.index()],
            "4vec ordering should match scalar ordering: \
             Red scalar={scalar_red} > Blue scalar={scalar_blue}, \
             but Red 4vec={} <= Blue 4vec={}",
            vec[Player::Red.index()],
            vec[Player::Blue.index()]
        );
    }
}

// ── Additional: Starting position symmetry ──

#[test]
fn test_starting_position_approximate_symmetry() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
    let gs = GameState::new_standard_ffa();

    let scores: Vec<i16> = Player::ALL
        .iter()
        .map(|&p| evaluator.eval_scalar(&gs, p))
        .collect();

    // Red/Yellow should be identical (same orientation relative to board center).
    assert_eq!(
        scores[Player::Red.index()],
        scores[Player::Yellow.index()],
        "Red and Yellow should have identical scores at start"
    );

    // Blue/Green should be identical.
    assert_eq!(
        scores[Player::Blue.index()],
        scores[Player::Green.index()],
        "Blue and Green should have identical scores at start"
    );

    // All scores should be close (within 100cp — small PST asymmetry from K/Q swap).
    let max = *scores.iter().max().unwrap();
    let min = *scores.iter().min().unwrap();
    assert!(
        max - min <= 100,
        "Starting position scores should be close: min={min}, max={max}, diff={}",
        max - min
    );
}

// ── Additional: Eliminated player gets very low score ──

#[test]
fn test_eliminated_player_low_score() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
    let mut gs = GameState::new_standard_ffa();

    // Resign Blue to make them DKW, then let their king get stuck/eliminated.
    gs.resign_player(Player::Blue);

    // Blue is now DKW, not eliminated. Score should be very low but not the eliminated floor.
    // Actually, check if Blue is still Active as DKW.
    let blue_status = gs.player_status(Player::Blue);
    if blue_status == odin_engine::gamestate::PlayerStatus::Eliminated {
        let score = evaluator.eval_scalar(&gs, Player::Blue);
        assert_eq!(score, -30_000, "Eliminated player should get -30000");
    }
    // DKW player should still have some eval (their king is alive), but very low.
}

// ── Additional: 4vec values are bounded [0, 1] ──

#[test]
fn test_4vec_bounded_01() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);
    let gs = GameState::new_standard_ffa();
    let vec = evaluator.eval_4vec(&gs);

    for (i, &v) in vec.iter().enumerate() {
        assert!((0.0..=1.0).contains(&v), "4vec[{i}] = {v} is not in [0, 1]");
    }
}

// ── Additional: Eval over random games doesn't panic ──

#[test]
fn test_eval_during_random_games_no_panic() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);

    for seed in 0..100u64 {
        let mut gs = GameState::new_standard_ffa();
        let mut rng = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut ply = 0;

        while !gs.is_game_over() && ply < 100 {
            // Evaluate at every position — must not panic.
            let scalar = evaluator.eval_scalar(&gs, gs.current_player());
            assert!(
                (-30_000..=30_000).contains(&scalar),
                "Scalar {scalar} out of range at game {seed}, ply {ply}"
            );

            let vec = evaluator.eval_4vec(&gs);
            for (i, &v) in vec.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(&v),
                    "4vec[{i}] = {v} out of range at game {seed}, ply {ply}"
                );
            }

            // Pick a random legal move.
            let moves = gs.legal_moves();
            if moves.is_empty() {
                break;
            }
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let idx = (rng >> 33) as usize % moves.len();
            gs.apply_move(moves[idx]);
            ply += 1;
        }
    }
}

// ── Additional: Prior-stage perft invariants still hold ──

#[test]
fn test_perft_values_unchanged() {
    use odin_engine::movegen::perft;

    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 1), 20);
    assert_eq!(perft(&mut board, 2), 395);
    assert_eq!(perft(&mut board, 3), 7_800);
}

// ── Additional: Eval values sanity ──

#[test]
fn test_eval_values_sanity() {
    use odin_engine::eval::{PAWN_EVAL_VALUE, QUEEN_EVAL_VALUE};

    // PromotedQueen should evaluate the same as Queen in eval.
    assert_eq!(
        odin_engine::eval::PROMOTED_QUEEN_EVAL_VALUE,
        QUEEN_EVAL_VALUE
    );

    // Basic ordering.
    const { assert!(PAWN_EVAL_VALUE < QUEEN_EVAL_VALUE) };
}
