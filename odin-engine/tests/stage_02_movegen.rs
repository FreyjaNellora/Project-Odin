// Stage 02 integration tests — Move Generation + Attack Query API
//
// Acceptance criteria:
// 1. Perft depths 1-4 match expected values (established here)
// 2. All special moves (castling, en passant, promotion) work correctly
// 3. Make/unmake round-trip preserves Zobrist hash
// 4. 1000+ random game playouts without crashes or assertion failures

use odin_engine::board::{square_from, Board, Piece, PieceType, Player};
use odin_engine::movegen::{
    generate_legal, is_in_check, is_square_attacked_by, make_move, perft, perft_divide,
    unmake_move, Move,
};

// ==================== Perft Tests ====================

// *** PERMANENT INVARIANTS — These values must never change. ***
// Established 2026-02-20 from FFA starting position (Red to move).

#[test]
fn test_perft_depth_1_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 1), 20);
    assert!(board.verify_zobrist());
}

#[test]
fn test_perft_depth_2_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 2), 395);
    assert!(board.verify_zobrist());
}

#[test]
fn test_perft_depth_3_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 3), 7800);
    assert!(board.verify_zobrist());
}

#[test]
fn test_perft_depth_4_starting_position() {
    let mut board = Board::starting_position();
    assert_eq!(perft(&mut board, 4), 152050);
    assert!(board.verify_zobrist());
}

#[test]
fn test_perft_divide_depth_1() {
    let mut board = Board::starting_position();
    let results = perft_divide(&mut board, 1);
    let total: u64 = results.iter().map(|(_, n)| n).sum();
    println!("perft_divide(1): {} moves", results.len());
    for (mv, nodes) in &results {
        println!("  {} -> {}", mv, nodes);
    }
    assert_eq!(total, perft(&mut board, 1));
}

// ==================== Make/Unmake Round-trip Tests ====================

#[test]
fn test_make_unmake_preserves_zobrist_all_depth1_moves() {
    let mut board = Board::starting_position();
    let hash_before = board.zobrist();
    let moves = generate_legal(&mut board);

    for mv in &moves {
        let undo = make_move(&mut board, *mv);
        assert!(board.verify_zobrist(), "zobrist invalid after make: {}", mv);
        assert!(board.verify_piece_lists(), "piece lists invalid after make: {}", mv);

        unmake_move(&mut board, *mv, undo);
        assert_eq!(
            board.zobrist(),
            hash_before,
            "zobrist not restored after unmake: {}",
            mv
        );
        assert!(board.verify_piece_lists(), "piece lists invalid after unmake: {}", mv);
    }
}

#[test]
fn test_make_unmake_preserves_zobrist_depth2() {
    let mut board = Board::starting_position();
    let hash_before = board.zobrist();
    let moves1 = generate_legal(&mut board);

    for mv1 in &moves1 {
        let undo1 = make_move(&mut board, *mv1);
        let hash_after_mv1 = board.zobrist();
        let moves2 = generate_legal(&mut board);

        for mv2 in &moves2 {
            let undo2 = make_move(&mut board, *mv2);
            assert!(board.verify_zobrist(), "zobrist invalid at depth 2: {} {}", mv1, mv2);

            unmake_move(&mut board, *mv2, undo2);
            assert_eq!(
                board.zobrist(),
                hash_after_mv1,
                "zobrist not restored at depth 2: {} {}",
                mv1,
                mv2
            );
        }

        unmake_move(&mut board, *mv1, undo1);
        assert_eq!(board.zobrist(), hash_before);
    }
}

// ==================== Special Move Tests ====================

#[test]
fn test_castling_available_in_starting_position() {
    // Clear the path for Red's kingside castle
    let mut board = Board::starting_position();

    // Remove pieces between Red's king (h1) and rook (k1)
    // i1 (8,0) has a bishop, j1 (9,0) has a knight
    board.remove_piece(square_from(8, 0).unwrap()); // i1
    board.remove_piece(square_from(9, 0).unwrap()); // j1

    let moves = generate_legal(&mut board);
    let castle_moves: Vec<_> = moves.iter().filter(|m| m.is_castle()).collect();
    assert!(
        !castle_moves.is_empty(),
        "Red should be able to castle kingside with clear path"
    );
}

#[test]
fn test_castling_blocked_by_check() {
    let mut board = Board::empty();
    // Red king on h1, Red rook on k1 (normal castling squares)
    board.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board.place_piece(
        square_from(10, 0).unwrap(),
        Piece::new(PieceType::Rook, Player::Red),
    );
    // Place Blue rook attacking i1 (the path square)
    board.place_piece(
        square_from(8, 5).unwrap(),
        Piece::new(PieceType::Rook, Player::Blue),
    );
    // Need Blue king somewhere safe
    board.place_piece(
        square_from(0, 6).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );
    // Set castling rights for Red kingside
    board.set_castling_rights(0x01); // CASTLE_RED_KING

    let moves = generate_legal(&mut board);
    let castle_moves: Vec<_> = moves.iter().filter(|m| m.is_castle()).collect();
    assert!(
        castle_moves.is_empty(),
        "Red should not castle kingside when path is attacked"
    );
}

#[test]
fn test_en_passant_capture() {
    // Valid 4PC EP scenario: Green just double-pushed m5->k5 (file 12->10, rank 4).
    // EP target = l5 (11, 4). Red pawn on k4 (10, 3) can capture EP at l5.
    // prev_player(Red) = Green, so captured pawn = l5 + Green's forward (-1, 0) = k5 (10, 4).
    let mut board = Board::empty();
    board.place_piece(
        square_from(10, 3).unwrap(), // k4 — Red pawn
        Piece::new(PieceType::Pawn, Player::Red),
    );
    board.place_piece(
        square_from(10, 4).unwrap(), // k5 — Green pawn that just double-pushed
        Piece::new(PieceType::Pawn, Player::Green),
    );
    // Kings
    board.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board.place_piece(
        square_from(13, 7).unwrap(),
        Piece::new(PieceType::King, Player::Green),
    );
    // EP target at l5 (11, 4) — the square the Green pawn passed through
    board.set_en_passant(Some(square_from(11, 4).unwrap()));

    let moves = generate_legal(&mut board);
    let ep_moves: Vec<_> = moves.iter().filter(|m| m.is_en_passant()).collect();
    assert_eq!(ep_moves.len(), 1, "should have exactly 1 ep move");

    // Execute the EP capture
    let ep_mv = *ep_moves[0];
    let hash_before = board.zobrist();
    let undo = make_move(&mut board, ep_mv);

    // The Green pawn at k5 should be gone
    assert!(board.piece_at(square_from(10, 4).unwrap()).is_none());
    // Red pawn should be on l5
    assert_eq!(
        board.piece_at(square_from(11, 4).unwrap()).unwrap().owner,
        Player::Red
    );
    assert!(board.verify_zobrist());

    // Unmake
    unmake_move(&mut board, ep_mv, undo);
    assert_eq!(board.zobrist(), hash_before);
    assert!(board.verify_piece_lists());
    // Green pawn should be back at k5
    assert_eq!(
        board.piece_at(square_from(10, 4).unwrap()).unwrap().owner,
        Player::Green
    );
}

#[test]
fn test_promotion_generates_4_options() {
    let mut board = Board::empty();
    // Red pawn on e8 (file 4, rank 7) — will promote to rank 8
    board.place_piece(
        square_from(4, 7).unwrap(),
        Piece::new(PieceType::Pawn, Player::Red),
    );
    board.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );

    let moves = generate_legal(&mut board);
    let promo_moves: Vec<_> = moves.iter().filter(|m| m.is_promotion()).collect();
    assert_eq!(
        promo_moves.len(),
        4,
        "should have 4 promotion options: PromotedQueen, Knight, Rook, Bishop"
    );
}

#[test]
fn test_promotion_make_unmake() {
    let mut board = Board::empty();
    let pawn_sq = square_from(4, 7).unwrap();
    let promo_sq = square_from(4, 8).unwrap();
    board.place_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
    board.place_piece(
        square_from(7, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );

    let hash_before = board.zobrist();
    let mv = Move::new_promotion(pawn_sq, promo_sq, None, PieceType::PromotedQueen);
    let undo = make_move(&mut board, mv);

    // Promoted piece should be on the target square
    let piece = board.piece_at(promo_sq).unwrap();
    assert_eq!(piece.piece_type, PieceType::PromotedQueen);
    assert_eq!(piece.owner, Player::Red);
    assert!(board.piece_at(pawn_sq).is_none());
    assert!(board.verify_zobrist());

    unmake_move(&mut board, mv, undo);
    assert_eq!(board.zobrist(), hash_before);
    let pawn = board.piece_at(pawn_sq).unwrap();
    assert_eq!(pawn.piece_type, PieceType::Pawn);
    assert!(board.piece_at(promo_sq).is_none());
}

// ==================== Attack API Tests ====================

#[test]
fn test_no_check_at_start() {
    let board = Board::starting_position();
    for &player in &Player::ALL {
        assert!(!is_in_check(player, &board), "{:?} should not be in check at start", player);
    }
}

#[test]
fn test_check_detection_simple() {
    let mut board = Board::empty();
    // Red king on e1, Blue rook on e8 — Red is in check
    board.place_piece(
        square_from(4, 0).unwrap(),
        Piece::new(PieceType::King, Player::Red),
    );
    board.place_piece(
        square_from(4, 7).unwrap(),
        Piece::new(PieceType::Rook, Player::Blue),
    );
    board.place_piece(
        square_from(0, 6).unwrap(),
        Piece::new(PieceType::King, Player::Blue),
    );

    assert!(is_in_check(Player::Red, &board));
    assert!(!is_in_check(Player::Blue, &board));
}

#[test]
fn test_all_four_player_pawn_directions() {
    // Verify each player's pawns attack the correct diagonal
    let mut board = Board::empty();

    // Red pawn on g7 (6, 6)
    board.place_piece(
        square_from(6, 6).unwrap(),
        Piece::new(PieceType::Pawn, Player::Red),
    );
    // Red should attack f8 (5, 7) and h8 (7, 7)
    assert!(is_square_attacked_by(
        square_from(5, 7).unwrap(),
        Player::Red,
        &board
    ));
    assert!(is_square_attacked_by(
        square_from(7, 7).unwrap(),
        Player::Red,
        &board
    ));

    let mut board2 = Board::empty();
    // Blue pawn on g7 (6, 6) — Blue attacks at +file diagonals
    board2.place_piece(
        square_from(6, 6).unwrap(),
        Piece::new(PieceType::Pawn, Player::Blue),
    );
    // Blue should attack h6 (7, 5) and h8 (7, 7)
    assert!(is_square_attacked_by(
        square_from(7, 5).unwrap(),
        Player::Blue,
        &board2
    ));
    assert!(is_square_attacked_by(
        square_from(7, 7).unwrap(),
        Player::Blue,
        &board2
    ));

    let mut board3 = Board::empty();
    // Yellow pawn on g7 (6, 6) — Yellow attacks at -rank diagonals
    board3.place_piece(
        square_from(6, 6).unwrap(),
        Piece::new(PieceType::Pawn, Player::Yellow),
    );
    // Yellow should attack f6 (5, 5) and h6 (7, 5)
    assert!(is_square_attacked_by(
        square_from(5, 5).unwrap(),
        Player::Yellow,
        &board3
    ));
    assert!(is_square_attacked_by(
        square_from(7, 5).unwrap(),
        Player::Yellow,
        &board3
    ));

    let mut board4 = Board::empty();
    // Green pawn on g7 (6, 6) — Green attacks at -file diagonals
    board4.place_piece(
        square_from(6, 6).unwrap(),
        Piece::new(PieceType::Pawn, Player::Green),
    );
    // Green should attack f6 (5, 5) and f8 (5, 7)
    assert!(is_square_attacked_by(
        square_from(5, 5).unwrap(),
        Player::Green,
        &board4
    ));
    assert!(is_square_attacked_by(
        square_from(5, 7).unwrap(),
        Player::Green,
        &board4
    ));
}

// ==================== Stress Test: Random Playouts ====================

#[test]
fn test_random_playouts_no_crash() {
    // Play 1000+ random games to at least 50 ply.
    // Verify invariants after every move.
    let num_games = 1000;
    let max_ply = 100;

    for game_idx in 0..num_games {
        let mut board = Board::starting_position();
        let mut ply = 0;

        // Simple deterministic "random" using game_idx + ply as seed
        let mut seed = game_idx as u64;

        while ply < max_ply {
            let legal_moves = generate_legal(&mut board);
            if legal_moves.is_empty() {
                break; // Stalemate or no moves
            }

            // Pseudo-random move selection
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let idx = (seed >> 33) as usize % legal_moves.len();
            let mv = legal_moves[idx];

            let _undo = make_move(&mut board, mv);

            assert!(
                board.verify_zobrist(),
                "game {} ply {}: zobrist invalid after {}",
                game_idx,
                ply,
                mv
            );
            assert!(
                board.verify_piece_lists(),
                "game {} ply {}: piece lists invalid after {}",
                game_idx,
                ply,
                mv
            );

            ply += 1;
        }
    }
}

// ==================== Board Restoration Tests ====================

#[test]
fn test_board_fully_restored_after_perft() {
    let mut board = Board::starting_position();
    let hash_before = board.zobrist();
    let piece_count_before = board.piece_count();

    let _ = perft(&mut board, 3);

    assert_eq!(board.zobrist(), hash_before, "zobrist not restored after perft");
    assert_eq!(
        board.piece_count(),
        piece_count_before,
        "piece count changed after perft"
    );
    assert!(board.verify_zobrist());
    assert!(board.verify_piece_lists());
}

// ==================== Multi-player Move Sequence ====================

#[test]
fn test_four_player_move_sequence() {
    // Play one move for each player and verify state
    let mut board = Board::starting_position();
    assert_eq!(board.side_to_move(), Player::Red);

    // Red: e2-e4
    let red_moves = generate_legal(&mut board);
    assert!(!red_moves.is_empty());
    let undo_red = make_move(&mut board, red_moves[0]);
    assert_eq!(board.side_to_move(), Player::Blue);

    // Blue
    let blue_moves = generate_legal(&mut board);
    assert!(!blue_moves.is_empty());
    let undo_blue = make_move(&mut board, blue_moves[0]);
    assert_eq!(board.side_to_move(), Player::Yellow);

    // Yellow
    let yellow_moves = generate_legal(&mut board);
    assert!(!yellow_moves.is_empty());
    let undo_yellow = make_move(&mut board, yellow_moves[0]);
    assert_eq!(board.side_to_move(), Player::Green);

    // Green
    let green_moves = generate_legal(&mut board);
    assert!(!green_moves.is_empty());
    let undo_green = make_move(&mut board, green_moves[0]);
    assert_eq!(board.side_to_move(), Player::Red);

    assert!(board.verify_zobrist());

    // Unmake everything
    let hash_start = {
        let fresh = Board::starting_position();
        fresh.zobrist()
    };

    unmake_move(&mut board, green_moves[0], undo_green);
    unmake_move(&mut board, yellow_moves[0], undo_yellow);
    unmake_move(&mut board, blue_moves[0], undo_blue);
    unmake_move(&mut board, red_moves[0], undo_red);

    assert_eq!(board.zobrist(), hash_start);
    assert!(board.verify_piece_lists());
}
