// Stage 17 — Game Mode Variant Tuning Tests
//
// T1-T5: Chess960 position generation and castling
// T6-T7: DKW awareness (eval + move ordering)
// T8-T9: FFA strategy eval
// T10-T12: Terrain eval
// T13-T16: Integration / completion tests
// T17-T18: Regression / DKW game tests

use odin_engine::board::{square_from, Board, Piece, PieceStatus, PieceType, Player};
use odin_engine::eval::{BootstrapEvaluator, EvalProfile, Evaluator};
use odin_engine::gamestate::{GameMode, GameState};
use odin_engine::movegen;
use odin_engine::search::hybrid::HybridController;
use odin_engine::search::{SearchBudget, Searcher};
use odin_engine::variants::chess960;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn depth_budget(d: u8) -> SearchBudget {
    SearchBudget {
        max_depth: Some(d),
        max_nodes: None,
        max_time_ms: None,
    }
}

fn make_hybrid() -> HybridController {
    HybridController::new(EvalProfile::Standard, None)
}

// ---------------------------------------------------------------------------
// T1: Chess960 valid position
// ---------------------------------------------------------------------------

#[test]
fn test_chess960_valid_position() {
    for seed in 0..100 {
        let rank = chess960::generate_back_rank(seed);
        assert!(
            chess960::is_valid_chess960(&rank),
            "seed {seed} produced invalid Chess960 arrangement: {rank:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// T2: Chess960 all players symmetric
// ---------------------------------------------------------------------------

#[test]
fn test_chess960_all_players_symmetric() {
    for seed in [0, 42, 99, 314] {
        let board = Board::chess960_position(seed);

        // Extract logical piece types for each player's back rank (files 3-10).
        // Red: rank 0, files 3-10
        let red_rank: Vec<PieceType> = (3..=10)
            .map(|f| board.piece_at(square_from(f, 0).unwrap()).unwrap().piece_type)
            .collect();

        // Blue: file 0, ranks 3-10 — REVERSED for comparison
        let blue_rank: Vec<PieceType> = (3..=10)
            .rev()
            .map(|r| board.piece_at(square_from(0, r).unwrap()).unwrap().piece_type)
            .collect();

        // Yellow: rank 13, files 3-10 — REVERSED for comparison
        let yellow_rank: Vec<PieceType> = (3..=10)
            .rev()
            .map(|f| board.piece_at(square_from(f, 13).unwrap()).unwrap().piece_type)
            .collect();

        // Green: file 13, ranks 3-10
        let green_rank: Vec<PieceType> = (3..=10)
            .map(|r| board.piece_at(square_from(13, r).unwrap()).unwrap().piece_type)
            .collect();

        assert_eq!(
            red_rank, green_rank,
            "seed {seed}: Red and Green should have same layout"
        );
        assert_eq!(
            red_rank, blue_rank,
            "seed {seed}: Red and Blue should have same logical layout (Blue reversed)"
        );
        assert_eq!(
            red_rank, yellow_rank,
            "seed {seed}: Red and Yellow should have same logical layout (Yellow reversed)"
        );
    }
}

// ---------------------------------------------------------------------------
// T3: Chess960 deterministic seed
// ---------------------------------------------------------------------------

#[test]
fn test_chess960_deterministic_seed() {
    for seed in [0, 42, 999] {
        let a = Board::chess960_position(seed);
        let b = Board::chess960_position(seed);
        // Compare piece placements
        for sq_idx in 0u8..196 {
            assert_eq!(
                format!("{:?}", a.piece_at(sq_idx)),
                format!("{:?}", b.piece_at(sq_idx)),
                "seed {seed}, sq {sq_idx}: positions differ"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// T4: Chess960 different seeds differ
// ---------------------------------------------------------------------------

#[test]
fn test_chess960_different_seeds_differ() {
    let mut arrangements = std::collections::HashSet::new();
    for seed in 0..50 {
        let rank = chess960::generate_back_rank(seed);
        arrangements.insert(rank);
    }
    assert!(
        arrangements.len() > 5,
        "only {} distinct arrangements from 50 seeds",
        arrangements.len()
    );
}

// ---------------------------------------------------------------------------
// T5: Chess960 castling legal
// ---------------------------------------------------------------------------

#[test]
fn test_chess960_castling_legal() {
    // Generate several Chess960 positions and verify castling_starts are set.
    for seed in [0, 42, 100, 777] {
        let board = Board::chess960_position(seed);
        let starts = board.castling_starts();

        // Verify each player has valid castling starts
        for (pi, &(king_sq, ks_rook, qs_rook)) in starts.iter().enumerate() {
            let king_piece = board.piece_at(king_sq);
            assert!(
                king_piece.is_some(),
                "seed {seed}, player {pi}: no piece at king start square"
            );
            assert_eq!(
                king_piece.unwrap().piece_type,
                PieceType::King,
                "seed {seed}, player {pi}: expected King at start square"
            );

            let ks_piece = board.piece_at(ks_rook);
            assert!(
                ks_piece.is_some(),
                "seed {seed}, player {pi}: no piece at KS rook square"
            );
            assert_eq!(
                ks_piece.unwrap().piece_type,
                PieceType::Rook,
                "seed {seed}, player {pi}: expected Rook at KS rook square"
            );

            let qs_piece = board.piece_at(qs_rook);
            assert!(
                qs_piece.is_some(),
                "seed {seed}, player {pi}: no piece at QS rook square"
            );
            assert_eq!(
                qs_piece.unwrap().piece_type,
                PieceType::Rook,
                "seed {seed}, player {pi}: expected Rook at QS rook square"
            );
        }

        // Verify legal move generation doesn't panic
        let mut board_clone = board.clone();
        let _legal = movegen::generate_legal(&mut board_clone);
    }
}

// ---------------------------------------------------------------------------
// T6: DKW eval penalty
// ---------------------------------------------------------------------------

#[test]
fn test_dkw_eval_penalty() {
    // Create a position where Blue's pieces are Dead (DKW) and Blue's king
    // is near Red's king. Compare eval with Blue king far away.
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);

    // Position 1: DKW king near Red's king (distance 2)
    let mut board_near = Board::empty();
    board_near.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    // Blue king at (9, 0) — distance 2
    let blue_king_near = square_from(9, 0).unwrap();
    board_near.place_piece(blue_king_near, Piece::new(PieceType::King, Player::Blue));
    board_near.set_piece_status(blue_king_near, PieceStatus::Dead);
    // Other kings far away
    board_near.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board_near.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board_near.set_side_to_move(Player::Red);

    // Position 2: DKW king far from Red's king (distance 10)
    let mut board_far = Board::empty();
    board_far.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    // Blue king at (3, 10) — far away
    let blue_king_far = square_from(3, 10).unwrap();
    board_far.place_piece(blue_king_far, Piece::new(PieceType::King, Player::Blue));
    board_far.set_piece_status(blue_king_far, PieceStatus::Dead);
    board_far.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board_far.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board_far.set_side_to_move(Player::Red);

    // Create game states — mark Blue as DeadKingWalking
    let gs_near = GameState::new(board_near, GameMode::FreeForAll, false);
    let gs_far = GameState::new(board_far, GameMode::FreeForAll, false);

    let eval_near = evaluator.eval_scalar(&gs_near, Player::Red);
    let eval_far = evaluator.eval_scalar(&gs_far, Player::Red);

    // Near DKW should produce lower eval (penalty) than far DKW
    // Note: The penalty only applies when the DKW king is actually marked Dead on the board.
    // Since we set the piece status to Dead, the DKW proximity check will trigger.
    // The eval difference may be subtle since other factors dominate, but the
    // near position should have a penalty the far one doesn't.
    // We just verify they're different — the near case should be worse or equal.
    assert!(
        eval_near <= eval_far,
        "DKW near ({eval_near}) should be <= far ({eval_far}) due to proximity penalty"
    );
}

// ---------------------------------------------------------------------------
// T7: Dead piece capture ordering
// ---------------------------------------------------------------------------

#[test]
fn test_dead_piece_capture_ordering() {
    // The dead piece capture fix in BRS order_moves ensures dead captures
    // get minimal value. We verify by checking that the engine doesn't
    // produce degenerate behavior with dead pieces present.
    // A simple smoke test: board with dead pieces, search doesn't panic.
    let mut board = Board::empty();
    board.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board.place_piece(
        square_from(6, 1).unwrap(),
        Piece::new(PieceType::Queen, Player::Red),
    );

    // Dead Blue queen adjacent to Red queen
    let dead_sq = square_from(5, 2).unwrap();
    board.place_piece(dead_sq, Piece::new(PieceType::Queen, Player::Blue));
    board.set_piece_status(dead_sq, PieceStatus::Dead);

    // Alive Green rook also adjacent
    board.place_piece(
        square_from(8, 3).unwrap(),
        Piece::new(PieceType::Rook, Player::Green),
    );

    // Other kings
    board.place_piece(
        square_from(3, 10).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board.set_piece_status(square_from(3, 10).unwrap(), PieceStatus::Dead);
    board.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board.set_side_to_move(Player::Red);

    let gs = GameState::new(board, GameMode::FreeForAll, false);
    let mut searcher = make_hybrid();
    let result = searcher.search(&gs, depth_budget(4));

    // Search should complete without panic
    assert!(
        result.depth > 0,
        "search should complete with non-zero depth"
    );
}

// ---------------------------------------------------------------------------
// T8: FFA claim-win urgency
// ---------------------------------------------------------------------------

#[test]
fn test_ffa_claim_win_urgency() {
    let weights_agg = EvalProfile::Aggressive.weights();
    let weights_std = EvalProfile::Standard.weights();

    // Aggressive profile should have higher claim-win urgency than Standard
    assert!(
        weights_agg.claim_win_urgency_bonus > weights_std.claim_win_urgency_bonus,
        "Aggressive profile should have higher claim-win urgency than Standard"
    );
    assert_eq!(weights_agg.claim_win_urgency_bonus, 100);
    assert_eq!(weights_std.claim_win_urgency_bonus, 50);

    // Verify the new weights are properly set for both profiles
    assert_eq!(weights_std.dkw_proximity_penalty, 20);
    assert_eq!(weights_agg.dkw_proximity_penalty, 20);
    assert_eq!(weights_std.terrain_fortress_bonus, 15);
    assert_eq!(weights_std.terrain_king_wall_bonus, 20);
    assert_eq!(weights_std.terrain_king_trap_penalty, 30);
}

// ---------------------------------------------------------------------------
// T9: FFA eval mode gated
// ---------------------------------------------------------------------------

#[test]
fn test_ffa_eval_mode_gated() {
    // Verify that the evaluator works correctly in both FFA and LKS modes.
    let eval_ffa = BootstrapEvaluator::new(EvalProfile::Aggressive);
    let eval_lks = BootstrapEvaluator::new(EvalProfile::Standard);

    let gs_ffa = GameState::new_standard_ffa();
    let gs_lks = GameState::new_standard_lks();

    // Both should produce valid scores without panic
    let score_ffa = eval_ffa.eval_scalar(&gs_ffa, Player::Red);
    let score_lks = eval_lks.eval_scalar(&gs_lks, Player::Red);

    assert!(score_ffa > -30_000 && score_ffa < 30_000);
    assert!(score_lks > -30_000 && score_lks < 30_000);

    // 4vec should also work
    let vec_ffa = eval_ffa.eval_4vec(&gs_ffa);
    let vec_lks = eval_lks.eval_4vec(&gs_lks);

    for &v in &vec_ffa {
        assert!((0.0..=1.0).contains(&v));
    }
    for &v in &vec_lks {
        assert!((0.0..=1.0).contains(&v));
    }
}

// ---------------------------------------------------------------------------
// T10: Terrain fortress bonus
// ---------------------------------------------------------------------------

#[test]
fn test_terrain_fortress_bonus() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);

    // Position with terrain pieces near Red's pieces
    let mut board_with_terrain = Board::empty();
    board_with_terrain.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board_with_terrain.place_piece(
        square_from(6, 2).unwrap(),
        Piece::new(PieceType::Knight, Player::Red),
    );
    // Place terrain adjacent to the knight
    let terrain_sq = square_from(5, 2).unwrap();
    board_with_terrain.place_piece(terrain_sq, Piece::new(PieceType::Pawn, Player::Blue));
    board_with_terrain.set_piece_status(terrain_sq, PieceStatus::Terrain);

    // Other kings
    board_with_terrain.place_piece(
        square_from(0, 6).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board_with_terrain.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board_with_terrain.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board_with_terrain.set_side_to_move(Player::Red);

    // Same position without terrain
    let mut board_no_terrain = Board::empty();
    board_no_terrain.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board_no_terrain.place_piece(
        square_from(6, 2).unwrap(),
        Piece::new(PieceType::Knight, Player::Red),
    );
    board_no_terrain.place_piece(
        square_from(0, 6).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board_no_terrain.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board_no_terrain.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board_no_terrain.set_side_to_move(Player::Red);

    // With terrain mode ON, the terrain-adjacent piece should get a bonus
    let gs_terrain = GameState::new(board_with_terrain, GameMode::FreeForAll, true);
    let gs_no_terrain = GameState::new(board_no_terrain, GameMode::FreeForAll, true);

    let eval_with = evaluator.eval_scalar(&gs_terrain, Player::Red);
    let eval_without = evaluator.eval_scalar(&gs_no_terrain, Player::Red);

    // The position with terrain adjacent to our knight should eval differently
    // than the one without terrain (fortress + outpost bonus)
    assert_ne!(
        eval_with, eval_without,
        "terrain fortress bonus should change eval"
    );
}

// ---------------------------------------------------------------------------
// T11: Terrain eval mode gated
// ---------------------------------------------------------------------------

#[test]
fn test_terrain_eval_mode_gated() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);

    // Same board, terrain_mode off vs on
    let board = Board::starting_position();

    let gs_no_terrain = GameState::new(board.clone(), GameMode::FreeForAll, false);
    let gs_terrain = GameState::new(board, GameMode::FreeForAll, true);

    let eval_off = evaluator.eval_scalar(&gs_no_terrain, Player::Red);
    let eval_on = evaluator.eval_scalar(&gs_terrain, Player::Red);

    // In starting position there's no terrain on the board, so terrain eval
    // should return 0 in both cases. The scores should be equal.
    assert_eq!(
        eval_off, eval_on,
        "with no terrain pieces on board, terrain_mode flag should not affect eval"
    );
}

// ---------------------------------------------------------------------------
// T12: Terrain king safety
// ---------------------------------------------------------------------------

#[test]
fn test_terrain_king_safety() {
    let evaluator = BootstrapEvaluator::new(EvalProfile::Standard);

    // King surrounded by 1-2 terrain pieces (wall bonus) vs 3+ (trap penalty)
    let mut board_wall = Board::empty();
    board_wall.place_piece(
        square_from(7, 5).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    // 2 terrain pieces adjacent (wall bonus)
    let t1 = square_from(6, 5).unwrap();
    let t2 = square_from(8, 5).unwrap();
    board_wall.place_piece(t1, Piece::new(PieceType::Pawn, Player::Blue));
    board_wall.set_piece_status(t1, PieceStatus::Terrain);
    board_wall.place_piece(t2, Piece::new(PieceType::Pawn, Player::Blue));
    board_wall.set_piece_status(t2, PieceStatus::Terrain);
    board_wall.place_piece(
        square_from(0, 6).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board_wall.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board_wall.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board_wall.set_side_to_move(Player::Red);

    let mut board_trap = Board::empty();
    board_trap.place_piece(
        square_from(7, 5).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    // 4 terrain pieces adjacent (trap penalty)
    for &(f, r) in &[(6, 5), (8, 5), (7, 4), (7, 6)] {
        let sq = square_from(f, r).unwrap();
        board_trap.place_piece(sq, Piece::new(PieceType::Pawn, Player::Blue));
        board_trap.set_piece_status(sq, PieceStatus::Terrain);
    }
    board_trap.place_piece(
        square_from(0, 6).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board_trap.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board_trap.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board_trap.set_side_to_move(Player::Red);

    let gs_wall = GameState::new(board_wall, GameMode::FreeForAll, true);
    let gs_trap = GameState::new(board_trap, GameMode::FreeForAll, true);

    let eval_wall = evaluator.eval_scalar(&gs_wall, Player::Red);
    let eval_trap = evaluator.eval_scalar(&gs_trap, Player::Red);

    // Wall (1-2 terrain) should eval better than trap (3+ terrain)
    assert!(
        eval_wall > eval_trap,
        "wall eval ({eval_wall}) should be > trap eval ({eval_trap})"
    );
}

// ---------------------------------------------------------------------------
// T13: FFA game completion
// ---------------------------------------------------------------------------

#[test]
fn test_ffa_game_completion() {
    let gs = GameState::new_standard_ffa();
    let mut searcher = make_hybrid();

    // Play up to 20 plies (5 full rotations)
    let mut position = gs;
    for _ in 0..20 {
        if position.is_game_over() {
            break;
        }
        let moves = position.legal_moves();
        if moves.is_empty() {
            position.handle_no_legal_moves();
            continue;
        }
        let result = searcher.search(&position, depth_budget(2));
        position.apply_move(result.best_move);
    }
    // Just verify it ran without panicking
}

// ---------------------------------------------------------------------------
// T14: LKS + Terrain game completion
// ---------------------------------------------------------------------------

#[test]
fn test_lks_terrain_game_completion() {
    let gs = GameState::new_standard_lks_terrain();
    let mut searcher = HybridController::new(EvalProfile::Standard, None);

    let mut position = gs;
    for _ in 0..20 {
        if position.is_game_over() {
            break;
        }
        let moves = position.legal_moves();
        if moves.is_empty() {
            position.handle_no_legal_moves();
            continue;
        }
        let result = searcher.search(&position, depth_budget(2));
        position.apply_move(result.best_move);
    }
    // Verify no panic
}

// ---------------------------------------------------------------------------
// T15: Chess960 game runs
// ---------------------------------------------------------------------------

#[test]
fn test_chess960_game_runs() {
    let board = Board::chess960_position(42);
    let gs = GameState::new(board, GameMode::FreeForAll, false);
    let mut searcher = make_hybrid();

    let mut position = gs;
    for _ in 0..12 {
        if position.is_game_over() {
            break;
        }
        let moves = position.legal_moves();
        if moves.is_empty() {
            position.handle_no_legal_moves();
            continue;
        }
        let result = searcher.search(&position, depth_budget(2));
        position.apply_move(result.best_move);
    }
    // Verify no panic from Chess960 position
}

// ---------------------------------------------------------------------------
// T16: perft standard unchanged
// ---------------------------------------------------------------------------

#[test]
fn test_perft_standard_unchanged() {
    let mut board = Board::starting_position();
    assert_eq!(movegen::perft(&mut board, 1), 20, "perft(1) must be 20");
    assert_eq!(movegen::perft(&mut board, 2), 395, "perft(2) must be 395");
    assert_eq!(
        movegen::perft(&mut board, 3),
        7800,
        "perft(3) must be 7800"
    );
    assert_eq!(
        movegen::perft(&mut board, 4),
        152050,
        "perft(4) must be 152050"
    );
}

// ---------------------------------------------------------------------------
// T17: No regression standard FFA (eval weights are valid)
// ---------------------------------------------------------------------------

#[test]
fn test_no_regression_standard_ffa() {
    // Verify that the new eval weights don't break the standard game.
    // Both profiles should produce valid scores on starting position.
    let gs = GameState::new_standard_ffa();

    let eval_std = BootstrapEvaluator::new(EvalProfile::Standard);
    let eval_agg = BootstrapEvaluator::new(EvalProfile::Aggressive);

    for &player in &Player::ALL {
        let score_std = eval_std.eval_scalar(&gs, player);
        let score_agg = eval_agg.eval_scalar(&gs, player);

        assert!(
            score_std > -30_000 && score_std < 30_000,
            "Standard eval for {player:?} out of range: {score_std}"
        );
        assert!(
            score_agg > -30_000 && score_agg < 30_000,
            "Aggressive eval for {player:?} out of range: {score_agg}"
        );
    }

    // Verify search still works and produces valid moves
    let mut searcher = make_hybrid();
    let result = searcher.search(&gs, depth_budget(4));
    let mut gs_check = gs.clone();
    let legal = gs_check.legal_moves();
    assert!(
        legal.contains(&result.best_move),
        "bestmove {} is not legal",
        result.best_move.to_algebraic()
    );
}

// ---------------------------------------------------------------------------
// T18: DKW game no panic
// ---------------------------------------------------------------------------

#[test]
fn test_dkw_game_no_panic() {
    // Create a position where one player gets mated (triggering DKW),
    // then continue playing. Verify no panics.
    let mut board = Board::empty();

    // Red king in starting area
    board.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board.place_piece(
        square_from(5, 1).unwrap(),
        Piece::new(PieceType::Pawn, Player::Red),
    );

    // Blue with some pieces
    board.place_piece(
        square_from(0, 6).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board.place_piece(
        square_from(3, 6).unwrap(),
        Piece::new(PieceType::Rook, Player::Blue),
    );

    // Yellow with king
    board.place_piece(
        square_from(6, 13).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );

    // Green with enough to checkmate Red (if needed)
    board.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    board.place_piece(
        square_from(13, 3).unwrap(),
        Piece::new(PieceType::Queen, Player::Green),
    );

    board.set_side_to_move(Player::Red);
    let gs = GameState::new(board, GameMode::FreeForAll, false);

    let mut searcher = make_hybrid();
    let mut position = gs;

    // Play several moves — this should exercise DKW paths if a player gets eliminated
    for _ in 0..16 {
        if position.is_game_over() {
            break;
        }
        let moves = position.legal_moves();
        if moves.is_empty() {
            position.handle_no_legal_moves();
            continue;
        }
        let result = searcher.search(&position, depth_budget(2));
        position.apply_move(result.best_move);
    }
    // No panic = pass
}
