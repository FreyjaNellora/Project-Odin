// Stage 04 integration tests — Odin Protocol
//
// Permanent invariant: Protocol round-trip works — send position + go,
// get legal bestmove back.
//
// Tests use OdinEngine::handle_command() + take_output() to verify
// protocol behavior without actual stdin/stdout.

use odin_engine::board::Board;
use odin_engine::movegen;
use odin_engine::protocol::{Command, OdinEngine, SearchLimits};

// ============================================================
// Permanent invariant: protocol round-trip
// ============================================================

#[test]
fn test_protocol_roundtrip_startpos() {
    let mut engine = OdinEngine::new();

    // Set position
    engine.handle_command(Command::PositionStartpos { moves: vec![] });

    // Go
    engine.take_output(); // clear any output from position
    engine.handle_command(Command::Go(SearchLimits::default()));
    let output = engine.take_output();

    // Must have info + bestmove
    assert!(
        output.len() >= 2,
        "expected info + bestmove, got {output:?}"
    );
    assert!(output.last().unwrap().starts_with("bestmove "));

    // Extract move and verify it's legal
    let bestmove_line = output.last().unwrap();
    let move_str = bestmove_line.strip_prefix("bestmove ").unwrap();
    let mut board = Board::starting_position();
    let legal = movegen::generate_legal(&mut board);
    assert!(
        legal.iter().any(|m| m.to_algebraic() == move_str),
        "bestmove '{move_str}' is not a legal move from starting position"
    );
}

#[test]
fn test_protocol_roundtrip_fen4() {
    let mut engine = OdinEngine::new();

    // Use starting position in FEN4 format
    let fen = Board::starting_position().to_fen4();
    engine.handle_command(Command::PositionFen4 { fen, moves: vec![] });
    engine.take_output();

    engine.handle_command(Command::Go(SearchLimits::default()));
    let output = engine.take_output();

    assert!(output.len() >= 2);
    let bestmove_line = output.last().unwrap();
    let move_str = bestmove_line.strip_prefix("bestmove ").unwrap();

    let mut board = Board::starting_position();
    let legal = movegen::generate_legal(&mut board);
    assert!(
        legal.iter().any(|m| m.to_algebraic() == move_str),
        "bestmove '{move_str}' is not a legal move"
    );
}

#[test]
fn test_protocol_roundtrip_with_moves() {
    let mut engine = OdinEngine::new();

    // Get first legal move from starting position
    let mut board = Board::starting_position();
    let legal = movegen::generate_legal(&mut board);
    let first_move = legal[0].to_algebraic();

    // Set position with one move applied
    engine.handle_command(Command::PositionStartpos {
        moves: vec![first_move.clone()],
    });
    engine.take_output();

    // Go from the resulting position
    engine.handle_command(Command::Go(SearchLimits::default()));
    let output = engine.take_output();

    assert!(output.len() >= 2);
    let bestmove_line = output.last().unwrap();
    let move_str = bestmove_line.strip_prefix("bestmove ").unwrap();

    // Verify the move is legal from the resulting position
    // (after one move has been played)
    let mut board2 = Board::starting_position();
    let legal2 = movegen::generate_legal(&mut board2);
    let mv = legal2
        .iter()
        .find(|m| m.to_algebraic() == first_move)
        .unwrap();
    movegen::make_move(&mut board2, *mv);
    let legal3 = movegen::generate_legal(&mut board2);
    assert!(
        legal3.iter().any(|m| m.to_algebraic() == move_str),
        "bestmove '{move_str}' is not legal after '{first_move}'"
    );
}

// ============================================================
// Acceptance criteria
// ============================================================

#[test]
fn test_odin_responds_with_id() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::Odin);
    let output = engine.take_output();

    assert!(output.len() >= 3);
    assert!(output[0].starts_with("id name Odin"));
    assert!(output[1].contains("author"));
    assert_eq!(output[2], "odinok");
}

#[test]
fn test_isready_responds_readyok() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::IsReady);
    let output = engine.take_output();
    assert_eq!(output, vec!["readyok"]);
}

#[test]
fn test_position_set_via_startpos() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::PositionStartpos { moves: vec![] });
    assert!(engine.game_state().is_some());
}

#[test]
fn test_position_set_via_fen4() {
    let mut engine = OdinEngine::new();
    let fen = Board::starting_position().to_fen4();
    engine.handle_command(Command::PositionFen4 { fen, moves: vec![] });
    assert!(engine.game_state().is_some());
}

#[test]
fn test_go_returns_legal_move() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::PositionStartpos { moves: vec![] });
    engine.take_output();

    engine.handle_command(Command::Go(SearchLimits::default()));
    let output = engine.take_output();

    // Must have bestmove
    let bestmove_line = output.iter().find(|l| l.starts_with("bestmove")).unwrap();
    let move_str = bestmove_line.strip_prefix("bestmove ").unwrap();

    // Verify legal
    let mut board = Board::starting_position();
    let legal = movegen::generate_legal(&mut board);
    assert!(legal.iter().any(|m| m.to_algebraic() == move_str));
}

#[test]
fn test_malformed_input_no_crash() {
    let mut engine = OdinEngine::new();

    // Various malformed inputs — none should crash
    let garbage_inputs = [
        Command::Unknown("".to_string()),
        Command::Unknown("foobar baz quux".to_string()),
        Command::Unknown("position".to_string()),
        Command::PositionFen4 {
            fen: "totally invalid fen".to_string(),
            moves: vec![],
        },
        Command::PositionStartpos {
            moves: vec!["zzz9".to_string()],
        },
        Command::Go(SearchLimits::default()), // go without position
        Command::Stop,
    ];

    for cmd in garbage_inputs {
        // None of these should panic
        engine.handle_command(cmd);
    }

    // Engine should still be functional after all the garbage
    engine.handle_command(Command::PositionStartpos { moves: vec![] });
    engine.take_output();
    engine.handle_command(Command::Go(SearchLimits::default()));
    let output = engine.take_output();
    assert!(output.iter().any(|l| l.starts_with("bestmove")));
}

// ============================================================
// Edge cases
// ============================================================

#[test]
fn test_go_without_position_reports_error() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::Go(SearchLimits::default()));
    let output = engine.take_output();
    assert!(output[0].contains("Error"));
    assert!(output[0].contains("no position set"));
}

#[test]
fn test_illegal_move_in_position_reports_error() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::PositionStartpos {
        moves: vec!["a1a2".to_string()], // not a legal move
    });
    let output = engine.take_output();
    assert!(output[0].contains("Error"));
    assert!(output[0].contains("a1a2"));
    // Position should be cleared
    assert!(engine.game_state().is_none());
}

#[test]
fn test_setoption_accepted() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::SetOption {
        name: "Debug".to_string(),
        value: "true".to_string(),
    });
    // No error output
    assert!(engine.take_output().is_empty());
}

#[test]
fn test_unknown_command_produces_error() {
    let mut engine = OdinEngine::new();
    engine.handle_command(Command::Unknown("xyzzy".to_string()));
    let output = engine.take_output();
    assert!(output[0].contains("Error"));
    assert!(output[0].contains("xyzzy"));
}

#[test]
fn test_quit_stops_engine() {
    let mut engine = OdinEngine::new();
    assert!(engine.handle_command(Command::Quit));
}

#[test]
fn test_full_session_sequence() {
    // Simulate a complete protocol session
    let mut engine = OdinEngine::new();

    // Initialize
    assert!(!engine.handle_command(Command::Odin));
    let output = engine.take_output();
    assert!(output[0].starts_with("id name Odin"));

    // Ready check
    assert!(!engine.handle_command(Command::IsReady));
    assert_eq!(engine.take_output(), vec!["readyok"]);

    // Set option
    assert!(!engine.handle_command(Command::SetOption {
        name: "Debug".to_string(),
        value: "true".to_string(),
    }));

    // Set position
    assert!(!engine.handle_command(Command::PositionStartpos { moves: vec![] }));

    // Search
    assert!(!engine.handle_command(Command::Go(SearchLimits {
        wtime: Some(60000),
        btime: Some(60000),
        ytime: Some(60000),
        gtime: Some(60000),
        ..Default::default()
    })));
    let output = engine.take_output();
    assert!(output.iter().any(|l| l.starts_with("bestmove")));

    // Quit
    assert!(engine.handle_command(Command::Quit));
}

#[test]
fn test_command_parsing_roundtrip() {
    use odin_engine::protocol::parse_command;

    // Verify that parsing raw strings produces correct commands
    let cmd = parse_command("position startpos moves d4d5");
    match cmd {
        Command::PositionStartpos { moves } => {
            assert_eq!(moves, vec!["d4d5"]);
        }
        _ => panic!("expected PositionStartpos"),
    }

    let cmd = parse_command("go wtime 60000 btime 55000 depth 8");
    match cmd {
        Command::Go(limits) => {
            assert_eq!(limits.wtime, Some(60000));
            assert_eq!(limits.btime, Some(55000));
            assert_eq!(limits.depth, Some(8));
        }
        _ => panic!("expected Go"),
    }
}

// ============================================================
// Prior invariant preservation
// ============================================================

#[test]
fn test_perft_values_unchanged() {
    let mut board = Board::starting_position();
    assert_eq!(movegen::perft(&mut board, 1), 20);
    assert_eq!(movegen::perft(&mut board, 2), 395);
    assert_eq!(movegen::perft(&mut board, 3), 7800);
}
