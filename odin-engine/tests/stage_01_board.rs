// Integration tests for Stage 1: Board Representation.
//
// Tests acceptance criteria:
// - All 160 valid squares identified, all 36 corners rejected
// - FEN4 round-trip: parse starting position -> serialize -> matches original
// - Zobrist hash changes on piece placement/removal
// - Piece lists stay synchronized with board array

use odin_engine::board::{
    file_of, is_valid_square, rank_of, square_from, Board, Piece, PieceType, Player,
    INVALID_CORNER_COUNT, TOTAL_SQUARES, VALID_SQUARE_COUNT,
};

// --- Acceptance criterion: All 160 valid squares identified, all 36 corners rejected ---

#[test]
fn test_exactly_160_valid_squares() {
    let valid = (0..TOTAL_SQUARES as u8)
        .filter(|&sq| is_valid_square(sq))
        .count();
    assert_eq!(valid, VALID_SQUARE_COUNT);
}

#[test]
fn test_exactly_36_invalid_corners() {
    let invalid = (0..TOTAL_SQUARES as u8)
        .filter(|&sq| !is_valid_square(sq))
        .count();
    assert_eq!(invalid, INVALID_CORNER_COUNT);
}

#[test]
fn test_all_corner_squares_are_correctly_identified() {
    // Per 4PC_RULES_REFERENCE: corners at a1-c3, l1-n3, a12-c14, l12-n14
    let corner_ranges: [(std::ops::RangeInclusive<u8>, std::ops::RangeInclusive<u8>); 4] = [
        (0..=2, 0..=2),     // a1-c3 (bottom-left)
        (11..=13, 0..=2),   // l1-n3 (bottom-right)
        (0..=2, 11..=13),   // a12-c14 (top-left)
        (11..=13, 11..=13), // l12-n14 (top-right)
    ];

    for (file_range, rank_range) in &corner_ranges {
        for file in file_range.clone() {
            for rank in rank_range.clone() {
                let sq = square_from(file, rank).unwrap();
                assert!(
                    !is_valid_square(sq),
                    "square at file={file} rank={rank} should be invalid corner"
                );
            }
        }
    }

    // Verify border squares adjacent to corners ARE valid
    assert!(is_valid_square(square_from(3, 0).unwrap())); // d1
    assert!(is_valid_square(square_from(0, 3).unwrap())); // a4
    assert!(is_valid_square(square_from(10, 0).unwrap())); // k1
    assert!(is_valid_square(square_from(13, 3).unwrap())); // n4
    assert!(is_valid_square(square_from(3, 13).unwrap())); // d14
    assert!(is_valid_square(square_from(0, 10).unwrap())); // a11
    assert!(is_valid_square(square_from(10, 13).unwrap())); // k14
    assert!(is_valid_square(square_from(13, 10).unwrap())); // n11
}

// --- Acceptance criterion: FEN4 round-trip ---

#[test]
fn test_fen4_roundtrip_starting_position() {
    let board = Board::starting_position();
    let fen = board.to_fen4();
    let parsed = Board::from_fen4(&fen).expect("FEN4 parse failed");
    let fen2 = parsed.to_fen4();
    assert_eq!(
        fen, fen2,
        "FEN4 round-trip mismatch:\n  original:  {fen}\n  roundtrip: {fen2}"
    );
}

#[test]
fn test_fen4_roundtrip_preserves_zobrist() {
    let board = Board::starting_position();
    let fen = board.to_fen4();
    let parsed = Board::from_fen4(&fen).expect("FEN4 parse failed");
    assert_eq!(
        board.zobrist(),
        parsed.zobrist(),
        "Zobrist mismatch after FEN4 round-trip"
    );
}

#[test]
fn test_fen4_roundtrip_preserves_all_pieces() {
    let board = Board::starting_position();
    let fen = board.to_fen4();
    let parsed = Board::from_fen4(&fen).expect("FEN4 parse failed");

    // Check every valid square has the same piece
    for sq in 0..TOTAL_SQUARES as u8 {
        if is_valid_square(sq) {
            assert_eq!(
                board.piece_at(sq),
                parsed.piece_at(sq),
                "piece mismatch at square {sq} (file={}, rank={})",
                file_of(sq),
                rank_of(sq)
            );
        }
    }
}

// --- Acceptance criterion: Zobrist hash changes on piece placement/removal ---

#[test]
fn test_zobrist_changes_on_piece_placement() {
    let mut board = Board::empty();
    let initial_hash = board.zobrist();

    let sq = square_from(5, 5).unwrap();
    board.place_piece(sq, Piece::new(PieceType::Pawn, Player::Red));

    assert_ne!(
        board.zobrist(),
        initial_hash,
        "hash should change after placement"
    );
}

#[test]
fn test_zobrist_different_pieces_different_hashes() {
    let sq = square_from(5, 5).unwrap();

    let mut board1 = Board::empty();
    board1.place_piece(sq, Piece::new(PieceType::Pawn, Player::Red));

    let mut board2 = Board::empty();
    board2.place_piece(sq, Piece::new(PieceType::Knight, Player::Red));

    let mut board3 = Board::empty();
    board3.place_piece(sq, Piece::new(PieceType::Pawn, Player::Blue));

    assert_ne!(
        board1.zobrist(),
        board2.zobrist(),
        "different piece types should have different hashes"
    );
    assert_ne!(
        board1.zobrist(),
        board3.zobrist(),
        "different owners should have different hashes"
    );
}

#[test]
fn test_zobrist_place_remove_restores_hash() {
    let mut board = Board::empty();
    let initial_hash = board.zobrist();

    let sq = square_from(5, 5).unwrap();
    board.place_piece(sq, Piece::new(PieceType::Rook, Player::Green));
    board.remove_piece(sq);

    assert_eq!(
        board.zobrist(),
        initial_hash,
        "hash should restore after place+remove"
    );
}

// --- Acceptance criterion: Piece lists stay synchronized ---

#[test]
fn test_starting_position_piece_lists_synchronized() {
    let board = Board::starting_position();
    assert!(
        board.verify_piece_lists(),
        "piece lists out of sync in starting position"
    );
}

#[test]
fn test_piece_lists_sync_after_mutations() {
    let mut board = Board::empty();

    // Place several pieces
    let pieces = [
        (square_from(5, 5).unwrap(), PieceType::Pawn, Player::Red),
        (square_from(6, 5).unwrap(), PieceType::Knight, Player::Blue),
        (
            square_from(7, 7).unwrap(),
            PieceType::Bishop,
            Player::Yellow,
        ),
        (square_from(8, 8).unwrap(), PieceType::Queen, Player::Green),
    ];

    for (sq, pt, player) in &pieces {
        board.place_piece(*sq, Piece::new(*pt, *player));
        assert!(
            board.verify_piece_lists(),
            "piece lists out of sync after placing {pt:?}"
        );
    }

    // Remove one
    board.remove_piece(pieces[1].0);
    assert!(
        board.verify_piece_lists(),
        "piece lists out of sync after removal"
    );

    // Move one
    board.move_piece(pieces[0].0, square_from(5, 6).unwrap());
    assert!(
        board.verify_piece_lists(),
        "piece lists out of sync after move"
    );
}

// --- Starting position verification ---

#[test]
fn test_starting_position_has_64_pieces() {
    let board = Board::starting_position();
    assert_eq!(board.piece_count(), 64);
}

#[test]
fn test_starting_position_16_pieces_per_player() {
    let board = Board::starting_position();
    for &player in &Player::ALL {
        assert_eq!(
            board.piece_list(player).len(),
            16,
            "{player:?} should have 16 pieces"
        );
    }
}

#[test]
fn test_starting_position_all_kings_on_correct_squares() {
    let board = Board::starting_position();

    // Red king at h1 (file 7, rank 0): d1=R, e1=N, f1=B, g1=Q, h1=K
    let rk = board.king_square(Player::Red);
    assert_eq!((file_of(rk), rank_of(rk)), (7, 0), "Red king wrong");

    // Blue king at a7 (file 0, rank 6): a4=R, a5=N, a6=B, a7=K
    let bk = board.king_square(Player::Blue);
    assert_eq!((file_of(bk), rank_of(bk)), (0, 6), "Blue king wrong");

    // Yellow king at g14 (file 6, rank 13): d14=R, e14=N, f14=B, g14=K
    let yk = board.king_square(Player::Yellow);
    assert_eq!((file_of(yk), rank_of(yk)), (6, 13), "Yellow king wrong");

    // Green king at n8 (file 13, rank 7): n4=R, n5=N, n6=B, n7=Q, n8=K
    let gk = board.king_square(Player::Green);
    assert_eq!((file_of(gk), rank_of(gk)), (13, 7), "Green king wrong");
}

#[test]
fn test_starting_position_zobrist_is_valid() {
    let board = Board::starting_position();
    assert!(
        board.verify_zobrist(),
        "starting position Zobrist verification failed"
    );
}

#[test]
fn test_starting_position_all_castling_rights() {
    let board = Board::starting_position();
    assert_eq!(
        board.castling_rights(),
        0xFF,
        "all 8 castling rights should be set"
    );
}

#[test]
fn test_starting_position_no_en_passant() {
    let board = Board::starting_position();
    assert!(board.en_passant().is_none(), "no en passant at game start");
}

#[test]
fn test_starting_position_red_to_move() {
    let board = Board::starting_position();
    assert_eq!(board.side_to_move(), Player::Red, "Red moves first");
}
