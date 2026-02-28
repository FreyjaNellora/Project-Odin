// Stage 13 — Time Management integration tests
//
// Tests cover:
//   AC1: Engine manages time correctly (doesn't flag)
//   AC2: Time adapts to position complexity
//   AC3: Tunable parameters accepted via protocol

use std::time::Instant;

use odin_engine::board::{Board, Piece, PieceType, Player, square_from};
use odin_engine::eval::EvalProfile;
use odin_engine::gamestate::{GameMode, GameState};
use odin_engine::protocol::{parse_command, Command, OdinEngine, SearchLimits};
use odin_engine::search::hybrid::HybridController;
use odin_engine::search::time_manager::TimeManager;
use odin_engine::search::{SearchBudget, Searcher};

// ---------------------------------------------------------------------------
// T1: Basic time allocation (AC1)
// ---------------------------------------------------------------------------
#[test]
fn test_time_allocation_basic() {
    let ms = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, None);
    // 60000 / 50 * 0.8 (quiet) = 960, capped at 25% = 15000, min 100
    assert!(
        ms >= 500 && ms <= 3000,
        "basic allocation should be 500-3000ms, got {ms}"
    );
}

// ---------------------------------------------------------------------------
// T2: Increment increases allocated time (AC1)
// ---------------------------------------------------------------------------
#[test]
fn test_time_allocation_with_increment() {
    let without_inc = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, None);
    let with_inc = TimeManager::allocate(60_000, 2000, 0, None, 20, false, false, None);
    assert!(
        with_inc > without_inc,
        "increment should increase allocation: {with_inc} vs {without_inc}"
    );
}

// ---------------------------------------------------------------------------
// T3: Tactical positions get more time than quiet (AC2)
// ---------------------------------------------------------------------------
#[test]
fn test_time_allocation_tactical_bonus() {
    let quiet = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, None);
    let tactical = TimeManager::allocate(60_000, 0, 0, None, 20, true, false, None);
    assert!(
        tactical > quiet,
        "tactical should get more time: {tactical} vs {quiet}"
    );
}

// ---------------------------------------------------------------------------
// T4: Forced move — 1 legal move → 0ms allocation (AC2)
// ---------------------------------------------------------------------------
#[test]
fn test_time_allocation_forced_move() {
    let ms = TimeManager::allocate(60_000, 0, 0, None, 1, false, false, None);
    assert_eq!(ms, 0, "forced move (1 legal move) should return 0ms");
}

// ---------------------------------------------------------------------------
// T5: Safety cap — never exceeds 25% of remaining (AC1)
// ---------------------------------------------------------------------------
#[test]
fn test_time_allocation_safety_cap() {
    // High factor scenario: tactical + in_check + near_elimination
    let ms = TimeManager::allocate(4_000, 0, 0, None, 20, true, true, Some(1500));
    assert!(
        ms <= 1000,
        "should not exceed 25% of remaining (1000ms), got {ms}"
    );
}

// ---------------------------------------------------------------------------
// T6: Panic mode — low clock (<1s) is very conservative (AC1)
// ---------------------------------------------------------------------------
#[test]
fn test_time_allocation_panic_mode() {
    let ms = TimeManager::allocate(500, 0, 40, None, 20, false, false, None);
    assert!(ms <= 50, "panic mode (500ms remaining) should be <=50ms, got {ms}");
}

// ---------------------------------------------------------------------------
// T7: Near-elimination bonus (low score → more time) (AC2)
// ---------------------------------------------------------------------------
#[test]
fn test_time_allocation_near_elimination() {
    let normal = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, Some(4000));
    let desperate = TimeManager::allocate(60_000, 0, 0, None, 20, false, false, Some(1500));
    assert!(
        desperate > normal,
        "near-elimination should get more time: {desperate} vs {normal}"
    );
}

// ---------------------------------------------------------------------------
// T8: Protocol go with increments parses and produces bestmove (AC1)
// ---------------------------------------------------------------------------
#[test]
fn test_protocol_go_with_increments() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::PositionStartpos { moves: vec![] });
    engine.take_output();

    engine.handle_command(Command::Go(SearchLimits {
        wtime: Some(60_000),
        winc: Some(1000),
        btime: Some(60_000),
        binc: Some(1000),
        ytime: Some(60_000),
        yinc: Some(1000),
        gtime: Some(60_000),
        ginc: Some(1000),
        ..Default::default()
    }));
    let output = engine.take_output();

    assert!(
        output.last().unwrap().starts_with("bestmove "),
        "should produce bestmove with time controls + increments"
    );
    // Verify time_alloc info was emitted
    assert!(
        output.iter().any(|l| l.contains("time_alloc")),
        "should emit time_alloc info string; output: {output:?}"
    );
}

// ---------------------------------------------------------------------------
// T9: Forced move instant return (<50ms) (AC2)
// ---------------------------------------------------------------------------
#[test]
fn test_hybrid_forced_move_instant() {
    // Red king at e1 (4,0), boxed in by Blue rooks at d3 (3,2) and f3 (5,2).
    // Only legal move: e1-e2. This is a forced move.
    let mut board = Board::empty();
    board.set_side_to_move(Player::Red);
    board.set_castling_rights(0);
    board.place_piece(
        square_from(4, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board.place_piece(
        square_from(3, 2).unwrap(),
        Piece::new(PieceType::Rook, Player::Blue),
    );
    board.place_piece(
        square_from(5, 2).unwrap(),
        Piece::new(PieceType::Rook, Player::Blue),
    );
    // Other kings so the game is valid
    board.place_piece(
        square_from(3, 13).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    board.place_piece(
        square_from(10, 12).unwrap(),
        Piece::new(PieceType::King, Player::Yellow),
    );
    board.place_piece(
        square_from(13, 6).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    let gs = GameState::new(board, GameMode::FreeForAll, false);

    // Verify it's actually a forced move
    let mut gs_clone = gs.clone();
    let legal = gs_clone.legal_moves();
    assert!(
        legal.len() <= 2,
        "expected 1-2 legal moves for forced position, got {}",
        legal.len()
    );

    let start = Instant::now();
    let mut hybrid = HybridController::new(EvalProfile::Standard, None);
    let result = hybrid.search(
        &gs,
        SearchBudget {
            max_depth: Some(8),
            max_nodes: None,
            max_time_ms: Some(5000),
        },
    );
    let elapsed = start.elapsed();

    // If truly forced (1 move), should be instant
    if legal.len() == 1 {
        assert!(
            elapsed.as_millis() < 50,
            "forced move should return instantly, took {}ms",
            elapsed.as_millis()
        );
        assert_eq!(result.nodes, 0, "forced move should use 0 nodes");
    }
    // Either way, result must be legal
    assert!(
        legal.iter().any(|m| *m == result.best_move),
        "result move must be legal"
    );
}

// ---------------------------------------------------------------------------
// T10: Full game no flag — 20+ ply timed game (AC1)
// ---------------------------------------------------------------------------
#[test]
fn test_full_game_no_flag() {
    let mut engine = OdinEngine::new();
    let mut remaining = [60_000u64; 4]; // 60s each
    let increment = 1_000u64; // 1s increment
    let mut moves: Vec<String> = Vec::new();

    for ply in 0..24 {
        // Set position with accumulated moves
        if moves.is_empty() {
            engine.handle_command(Command::PositionStartpos { moves: vec![] });
        } else {
            let cmd_str = format!("position startpos moves {}", moves.join(" "));
            engine.handle_command(parse_command(&cmd_str));
        }
        engine.take_output();

        let player_idx = ply % 4;

        let limits = SearchLimits {
            wtime: Some(remaining[0]),
            btime: Some(remaining[1]),
            ytime: Some(remaining[2]),
            gtime: Some(remaining[3]),
            winc: Some(increment),
            binc: Some(increment),
            yinc: Some(increment),
            ginc: Some(increment),
            ..Default::default()
        };

        let start = Instant::now();
        engine.handle_command(Command::Go(limits));
        let elapsed = start.elapsed().as_millis() as u64;
        let output = engine.take_output();

        // Check for game over
        if output.iter().any(|l| l.contains("gameover")) {
            break;
        }

        // Extract bestmove
        let bestmove_line = output.iter().find(|l| l.starts_with("bestmove "));
        if bestmove_line.is_none() {
            break;
        }
        let mv = bestmove_line
            .unwrap()
            .strip_prefix("bestmove ")
            .unwrap()
            .to_string();

        // Update clock: must not have exceeded remaining time
        assert!(
            elapsed <= remaining[player_idx] + 500, // 500ms grace for OS scheduling
            "ply {} player {} used {}ms but only had {}ms",
            ply,
            player_idx,
            elapsed,
            remaining[player_idx]
        );
        remaining[player_idx] = remaining[player_idx].saturating_sub(elapsed) + increment;

        moves.push(mv);
    }
    // If we get here without panic/assertion failure, AC1 is satisfied
}

// ---------------------------------------------------------------------------
// T11: Enriched position classification (AC2)
// ---------------------------------------------------------------------------
#[test]
fn test_enriched_position_classification() {
    // Verify that check and tactical factors change allocation
    let normal = TimeManager::allocate(30_000, 0, 20, None, 15, false, false, None);
    let in_check = TimeManager::allocate(30_000, 0, 20, None, 15, false, true, None);
    // in_check gets ×1.2 boost but quiet ×0.8 → 0.96 vs 0.8 → should be more
    assert!(
        in_check > normal,
        "in-check should get more time than normal: {in_check} vs {normal}"
    );

    // Tactical + in check vs just quiet
    let tactical_check = TimeManager::allocate(30_000, 0, 20, None, 15, true, true, None);
    assert!(
        tactical_check > normal,
        "tactical + check > quiet: {tactical_check} vs {normal}"
    );
}

// ---------------------------------------------------------------------------
// T12: setoption tunable params accepted by protocol (AC3)
// ---------------------------------------------------------------------------
#[test]
fn test_setoption_tunable_params() {
    let mut engine = OdinEngine::new();

    engine.handle_command(Command::SetOption {
        name: "tactical_margin".to_string(),
        value: "200".to_string(),
    });
    let output = engine.take_output();
    assert!(
        output.is_empty() || !output.iter().any(|l| l.contains("Error")),
        "setoption tactical_margin should not error"
    );

    engine.handle_command(Command::SetOption {
        name: "brs_max_depth".to_string(),
        value: "10".to_string(),
    });
    let output = engine.take_output();
    assert!(
        output.is_empty() || !output.iter().any(|l| l.contains("Error")),
        "setoption brs_max_depth should not error"
    );

    engine.handle_command(Command::SetOption {
        name: "mcts_default_sims".to_string(),
        value: "4000".to_string(),
    });
    let output = engine.take_output();
    assert!(
        output.is_empty() || !output.iter().any(|l| l.contains("Error")),
        "setoption mcts_default_sims should not error"
    );

    // Verify the engine still works with tuned params
    engine.handle_command(Command::PositionStartpos { moves: vec![] });
    engine.take_output();
    engine.handle_command(Command::Go(SearchLimits {
        depth: Some(4),
        ..Default::default()
    }));
    let output = engine.take_output();
    assert!(
        output.last().unwrap().starts_with("bestmove "),
        "engine should still produce bestmove with tuned params"
    );
}
