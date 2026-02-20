// Attack query API — the board's public query interface.
//
// is_square_attacked_by(square, player, board) -> bool
// attackers_of(square, player, board) -> Vec<(PieceType, Square)>
//
// These are the foundation that everything downstream reuses:
// legal move filtering, check detection, castling legality, and
// the cheap interaction filter in Stage 8.
//
// Per ADR-001: Nothing above Stage 2 reads board.squares[] directly.
// All board queries go through this API.

use crate::board::{Board, PieceType, Player, Square};

use super::tables::{
    global_attack_tables, is_diagonal, is_orthogonal, AttackTables, NUM_DIRECTIONS,
};

/// Check if a given square is attacked by any piece of the specified player.
pub fn is_square_attacked_by(sq: Square, attacker: Player, board: &Board) -> bool {
    let tables = global_attack_tables();
    is_square_attacked_by_with_tables(sq, attacker, board, tables)
}

/// Check if a given square is attacked by any piece of the specified player (with explicit tables).
fn is_square_attacked_by_with_tables(
    sq: Square,
    attacker: Player,
    board: &Board,
    tables: &AttackTables,
) -> bool {
    // Check pawn attacks (can this square be attacked by a pawn of `attacker`?)
    // We need the REVERSE of the pawn's capture direction. If attacker's pawns capture
    // at deltas D, the reverse lookup uses -D, which is the opposite-facing player's
    // capture deltas: Red(0)↔Yellow(2), Blue(1)↔Green(3).
    // Terrain pieces are inert — they do not attack.
    let reverse_pawn_idx = (attacker.index() + 2) % 4;
    for &pawn_sq in tables.pawn_attack_squares(reverse_pawn_idx, sq) {
        if let Some(piece) = board.piece_at(pawn_sq) {
            if piece.owner == attacker && piece.piece_type == PieceType::Pawn && !piece.is_terrain()
            {
                return true;
            }
        }
    }

    // Check knight attacks (terrain knights do not attack)
    for &knight_sq in tables.knight_destinations(sq) {
        if let Some(piece) = board.piece_at(knight_sq) {
            if piece.owner == attacker
                && piece.piece_type == PieceType::Knight
                && !piece.is_terrain()
            {
                return true;
            }
        }
    }

    // Check king attacks (terrain kings do not attack)
    for &king_sq in tables.king_destinations(sq) {
        if let Some(piece) = board.piece_at(king_sq) {
            if piece.owner == attacker && piece.piece_type == PieceType::King && !piece.is_terrain()
            {
                return true;
            }
        }
    }

    // Check sliding piece attacks (bishop, rook, queen, promoted queen)
    // Terrain pieces block rays but do not attack.
    for dir in 0..NUM_DIRECTIONS {
        for &ray_sq in tables.ray(sq, dir) {
            match board.piece_at(ray_sq) {
                Some(piece) if piece.is_terrain() => break, // Terrain blocks ray
                Some(piece) if piece.owner == attacker => {
                    let pt = piece.piece_type;
                    // Diagonal: bishop, queen, promoted queen
                    if is_diagonal(dir)
                        && (pt == PieceType::Bishop
                            || pt == PieceType::Queen
                            || pt == PieceType::PromotedQueen)
                    {
                        return true;
                    }
                    // Orthogonal: rook, queen, promoted queen
                    if is_orthogonal(dir)
                        && (pt == PieceType::Rook
                            || pt == PieceType::Queen
                            || pt == PieceType::PromotedQueen)
                    {
                        return true;
                    }
                    break; // Blocked by own piece (wrong type for this direction)
                }
                Some(_) => break, // Blocked by opponent's piece
                None => continue, // Empty square, continue along ray
            }
        }
    }

    false
}

/// Find all pieces of a given player that attack a given square.
/// Returns a list of (PieceType, Square) for each attacker.
pub fn attackers_of(sq: Square, attacker: Player, board: &Board) -> Vec<(PieceType, Square)> {
    let tables = global_attack_tables();
    let mut result = Vec::new();

    // Check pawn attacks (reverse lookup — see is_square_attacked_by_with_tables)
    // Terrain pieces are inert — they do not attack.
    let reverse_pawn_idx = (attacker.index() + 2) % 4;
    for &pawn_sq in tables.pawn_attack_squares(reverse_pawn_idx, sq) {
        if let Some(piece) = board.piece_at(pawn_sq) {
            if piece.owner == attacker && piece.piece_type == PieceType::Pawn && !piece.is_terrain()
            {
                result.push((PieceType::Pawn, pawn_sq));
            }
        }
    }

    // Check knight attacks (terrain knights do not attack)
    for &knight_sq in tables.knight_destinations(sq) {
        if let Some(piece) = board.piece_at(knight_sq) {
            if piece.owner == attacker
                && piece.piece_type == PieceType::Knight
                && !piece.is_terrain()
            {
                result.push((PieceType::Knight, knight_sq));
            }
        }
    }

    // Check king attacks (terrain kings do not attack)
    for &king_sq in tables.king_destinations(sq) {
        if let Some(piece) = board.piece_at(king_sq) {
            if piece.owner == attacker && piece.piece_type == PieceType::King && !piece.is_terrain()
            {
                result.push((PieceType::King, king_sq));
            }
        }
    }

    // Check sliding piece attacks (terrain pieces block rays but do not attack)
    for dir in 0..NUM_DIRECTIONS {
        for &ray_sq in tables.ray(sq, dir) {
            match board.piece_at(ray_sq) {
                Some(piece) if piece.is_terrain() => break,
                Some(piece) if piece.owner == attacker => {
                    let pt = piece.piece_type;
                    if is_diagonal(dir)
                        && (pt == PieceType::Bishop
                            || pt == PieceType::Queen
                            || pt == PieceType::PromotedQueen)
                    {
                        result.push((pt, ray_sq));
                    }
                    if is_orthogonal(dir)
                        && (pt == PieceType::Rook
                            || pt == PieceType::Queen
                            || pt == PieceType::PromotedQueen)
                    {
                        result.push((pt, ray_sq));
                    }
                    break;
                }
                Some(_) => break,
                None => continue,
            }
        }
    }

    result
}

/// Check if a player's king is in check (attacked by any opponent).
pub fn is_in_check(player: Player, board: &Board) -> bool {
    let king_sq = board.king_square(player);
    for &opponent in &Player::ALL {
        if opponent != player && is_square_attacked_by(king_sq, opponent, board) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{square_from, Board, Piece, PieceType, Player};

    #[test]
    fn test_rook_attacks_along_rank() {
        let mut board = Board::empty();
        let rook_sq = square_from(5, 5).unwrap();
        board.place_piece(rook_sq, Piece::new(PieceType::Rook, Player::Red));

        // Rook attacks along rank and file
        let target = square_from(5, 8).unwrap(); // same file, different rank
        assert!(is_square_attacked_by(target, Player::Red, &board));

        let target2 = square_from(8, 5).unwrap(); // same rank, different file
        assert!(is_square_attacked_by(target2, Player::Red, &board));

        // Rook does NOT attack diagonally
        let diag = square_from(7, 7).unwrap();
        assert!(!is_square_attacked_by(diag, Player::Red, &board));
    }

    #[test]
    fn test_bishop_attacks_diagonal() {
        let mut board = Board::empty();
        let bishop_sq = square_from(5, 5).unwrap();
        board.place_piece(bishop_sq, Piece::new(PieceType::Bishop, Player::Blue));

        // Bishop attacks diagonally
        let target = square_from(7, 7).unwrap();
        assert!(is_square_attacked_by(target, Player::Blue, &board));

        // Bishop does NOT attack orthogonally
        let ortho = square_from(5, 8).unwrap();
        assert!(!is_square_attacked_by(ortho, Player::Blue, &board));
    }

    #[test]
    fn test_queen_attacks_all_directions() {
        let mut board = Board::empty();
        let queen_sq = square_from(6, 6).unwrap();
        board.place_piece(queen_sq, Piece::new(PieceType::Queen, Player::Yellow));

        // Orthogonal
        assert!(is_square_attacked_by(
            square_from(6, 10).unwrap(),
            Player::Yellow,
            &board
        ));
        // Diagonal
        assert!(is_square_attacked_by(
            square_from(8, 8).unwrap(),
            Player::Yellow,
            &board
        ));
    }

    #[test]
    fn test_sliding_piece_blocked() {
        let mut board = Board::empty();
        let rook_sq = square_from(5, 3).unwrap();
        board.place_piece(rook_sq, Piece::new(PieceType::Rook, Player::Red));

        // Place a blocking piece
        let blocker = square_from(5, 6).unwrap();
        board.place_piece(blocker, Piece::new(PieceType::Pawn, Player::Blue));

        // Target behind blocker — NOT attacked
        let behind = square_from(5, 8).unwrap();
        assert!(!is_square_attacked_by(behind, Player::Red, &board));

        // Blocker square IS attacked
        assert!(is_square_attacked_by(blocker, Player::Red, &board));
    }

    #[test]
    fn test_knight_attacks() {
        let mut board = Board::empty();
        let knight_sq = square_from(6, 6).unwrap();
        board.place_piece(knight_sq, Piece::new(PieceType::Knight, Player::Green));

        // Knight L-shape attacks
        assert!(is_square_attacked_by(
            square_from(7, 8).unwrap(),
            Player::Green,
            &board
        ));
        assert!(is_square_attacked_by(
            square_from(8, 7).unwrap(),
            Player::Green,
            &board
        ));

        // Adjacent square — NOT attacked by knight
        assert!(!is_square_attacked_by(
            square_from(6, 7).unwrap(),
            Player::Green,
            &board
        ));
    }

    #[test]
    fn test_pawn_attacks_red_forward() {
        let mut board = Board::empty();
        // Red pawn on e4 (file 4, rank 3)
        let pawn_sq = square_from(4, 3).unwrap();
        board.place_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));

        // Red pawn attacks d5 and f5 (diag +rank)
        assert!(is_square_attacked_by(
            square_from(3, 4).unwrap(),
            Player::Red,
            &board
        )); // d5
        assert!(is_square_attacked_by(
            square_from(5, 4).unwrap(),
            Player::Red,
            &board
        )); // f5

        // Does NOT attack e5 (forward, not diagonal)
        assert!(!is_square_attacked_by(
            square_from(4, 4).unwrap(),
            Player::Red,
            &board
        ));
    }

    #[test]
    fn test_king_attacks_adjacent() {
        let mut board = Board::empty();
        let king_sq = square_from(6, 6).unwrap();
        board.place_piece(king_sq, Piece::new(PieceType::King, Player::Red));

        // King attacks all 8 adjacent squares
        for &(df, dr) in &[
            (0, 1),
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
            (-1, -1),
            (-1, 0),
            (-1, 1),
        ] {
            let target = square_from((6 + df) as u8, (6 + dr) as u8).unwrap();
            assert!(
                is_square_attacked_by(target, Player::Red, &board),
                "king should attack adjacent square"
            );
        }

        // Does NOT attack 2 squares away
        assert!(!is_square_attacked_by(
            square_from(6, 8).unwrap(),
            Player::Red,
            &board
        ));
    }

    #[test]
    fn test_attackers_of_finds_multiple() {
        let mut board = Board::empty();
        // Red rook on e1 and Red bishop on g3 — both attack f2
        board.place_piece(
            square_from(4, 0).unwrap(),
            Piece::new(PieceType::Rook, Player::Red),
        );
        board.place_piece(
            square_from(6, 2).unwrap(),
            Piece::new(PieceType::Bishop, Player::Red),
        );

        // Need a valid target — f2 is (5, 1) which is actually invalid (corner)
        // Let me use f4 (5, 3) instead
        // e1 rook doesn't attack f4. Let me rethink.
        // Red rook on e4 (4, 3) and Red knight on d6 (3, 5) — both attack e6 (4, 5)?
        // Rook on e4 attacks e6 (same file). Knight from d6 attacks e4? No.
        // Let me use a cleaner example:
        // Rook on e5 (4, 4) and Bishop on g7 (6, 6) — both attack f6 (5, 5)?
        // Rook on e5: attacks f5 (5, 4), not f6. No.
        // Let me just test with two pieces that converge.
        let mut board2 = Board::empty();
        // Red rook on f3 (5, 2) and Red bishop on h5 (7, 4)
        // Both attack f5 (5, 4)? Rook: same file f, yes. Bishop: g4? No.
        // Rook on f3: attacks f4, f5, f6... yes attacks f5
        // Bishop on d3 (3, 2): attacks e4, f5... yes attacks f5!
        board2.place_piece(
            square_from(5, 2).unwrap(),
            Piece::new(PieceType::Rook, Player::Red),
        );
        board2.place_piece(
            square_from(3, 2).unwrap(),
            Piece::new(PieceType::Bishop, Player::Red),
        );
        let target = square_from(5, 4).unwrap(); // f5
        let attackers = attackers_of(target, Player::Red, &board2);
        // Rook on f3 attacks f5 orthogonally (same file)
        // Bishop on d3 attacks f5 diagonally (NE ray: e4, f5)
        assert_eq!(
            attackers.len(),
            2,
            "expected 2 attackers, got {}",
            attackers.len()
        );
    }

    #[test]
    fn test_is_in_check_starting_position() {
        let board = Board::starting_position();
        // No player should be in check at the start
        for &player in &Player::ALL {
            assert!(!is_in_check(player, &board));
        }
    }

    #[test]
    fn test_promoted_queen_attacks_like_queen() {
        let mut board = Board::empty();
        let sq = square_from(6, 6).unwrap();
        board.place_piece(sq, Piece::new(PieceType::PromotedQueen, Player::Red));

        // Should attack orthogonally and diagonally
        assert!(is_square_attacked_by(
            square_from(6, 10).unwrap(),
            Player::Red,
            &board
        ));
        assert!(is_square_attacked_by(
            square_from(8, 8).unwrap(),
            Player::Red,
            &board
        ));
    }
}
