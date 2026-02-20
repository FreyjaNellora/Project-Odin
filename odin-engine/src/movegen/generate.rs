// Move generation: pseudo-legal and legal moves.
//
// Pseudo-legal generation produces all candidate moves without checking
// if they leave the king in check. Legal filtering applies each move
// and verifies the king's safety.

use crate::board::{
    file_of, is_valid_square, rank_of, square_from, Board, PieceType, Player, Square,
};

use super::attacks::{is_in_check, is_square_attacked_by};
use super::moves::{
    castling_empty_squares, castling_king_path, get_castling_config, make_move, unmake_move, Move,
};
use super::tables::{global_attack_tables, AttackTables, NUM_DIRECTIONS};

use crate::board::BOARD_SIZE;

/// Check if (file, rank) is within board bounds.
#[inline]
fn in_bounds(f: i8, r: i8) -> bool {
    (0..BOARD_SIZE as i8).contains(&f) && (0..BOARD_SIZE as i8).contains(&r)
}

/// Pawn forward direction delta per player: (file_delta, rank_delta).
const PAWN_FORWARD: [(i8, i8); 4] = [
    (0, 1),  // Red: +rank
    (1, 0),  // Blue: +file
    (0, -1), // Yellow: -rank
    (-1, 0), // Green: -file
];

/// Pawn starting rank/file and promotion rank/file per player.
/// (start_coord, double_step_target_coord, promotion_coord)
/// For Red/Yellow: these are ranks. For Blue/Green: these are files.
const PAWN_CONFIG: [(u8, u8, u8); 4] = [
    (1, 3, 8),   // Red: start rank 1, double to rank 3, promote at rank 8
    (1, 3, 8),   // Blue: start file 1, double to file 3, promote at file 8
    (12, 10, 5), // Yellow: start rank 12, double to rank 10, promote at rank 5
    (12, 10, 5), // Green: start file 12, double to file 10, promote at file 5
];

/// Generate all pseudo-legal moves for the current side to move.
pub fn generate_pseudo_legal(board: &Board) -> Vec<Move> {
    let tables = global_attack_tables();
    let player = board.side_to_move();
    let mut moves = Vec::new();

    for &(piece_type, sq) in board.piece_list(player) {
        match piece_type {
            PieceType::Pawn => generate_pawn_moves(board, player, sq, &mut moves),
            PieceType::Knight => generate_knight_moves(board, player, sq, tables, &mut moves),
            PieceType::Bishop => generate_sliding_moves(
                board, player, sq, piece_type, tables, &mut moves, true, false,
            ),
            PieceType::Rook => generate_sliding_moves(
                board, player, sq, piece_type, tables, &mut moves, false, true,
            ),
            PieceType::Queen | PieceType::PromotedQueen => generate_sliding_moves(
                board, player, sq, piece_type, tables, &mut moves, true, true,
            ),
            PieceType::King => generate_king_moves(board, player, sq, tables, &mut moves),
        }
    }

    // Castling
    generate_castling(board, player, &mut moves);

    moves
}

/// Generate all legal moves for the current side to move.
pub fn generate_legal(board: &mut Board) -> Vec<Move> {
    let pseudo = generate_pseudo_legal(board);
    let player = board.side_to_move();
    let mut legal = Vec::new();

    for mv in pseudo {
        let undo = make_move(board, mv);

        // After making the move, check if the player who just moved is in check
        // (which would make the move illegal). The side_to_move has already advanced,
        // so we check the PREVIOUS player.
        if !is_in_check(player, board) {
            legal.push(mv);
        }

        unmake_move(board, mv, undo);
    }

    legal
}

/// Generate pawn moves for a single pawn.
fn generate_pawn_moves(board: &Board, player: Player, sq: Square, moves: &mut Vec<Move>) {
    let tables = global_attack_tables();
    let pidx = player.index();
    let (df, dr) = PAWN_FORWARD[pidx];
    let (start_coord, _double_target, promo_coord) = (
        PAWN_CONFIG[pidx].0,
        PAWN_CONFIG[pidx].1,
        PAWN_CONFIG[pidx].2,
    );

    let file = file_of(sq) as i8;
    let rank = rank_of(sq) as i8;

    // Determine if this pawn is on its starting rank/file
    let is_on_start = match pidx {
        0 | 2 => rank_of(sq) == start_coord, // Red/Yellow: check rank
        1 | 3 => file_of(sq) == start_coord, // Blue/Green: check file
        _ => unreachable!(),
    };

    // Forward target
    let fwd_file = file + df;
    let fwd_rank = rank + dr;

    if in_bounds(fwd_file, fwd_rank) {
        let fwd_sq = square_from(fwd_file as u8, fwd_rank as u8).unwrap();
        if is_valid_square(fwd_sq) && board.piece_at(fwd_sq).is_none() {
            // Check if this is a promotion square
            let is_promotion = match pidx {
                0 | 2 => fwd_rank as u8 == promo_coord,
                1 | 3 => fwd_file as u8 == promo_coord,
                _ => unreachable!(),
            };

            if is_promotion {
                // In FFA: promote to PromotedQueen (1-pt queen)
                moves.push(Move::new_promotion(
                    sq,
                    fwd_sq,
                    None,
                    PieceType::PromotedQueen,
                ));
                // Also allow underpromotion to knight, bishop, rook
                moves.push(Move::new_promotion(sq, fwd_sq, None, PieceType::Knight));
                moves.push(Move::new_promotion(sq, fwd_sq, None, PieceType::Rook));
                moves.push(Move::new_promotion(sq, fwd_sq, None, PieceType::Bishop));
            } else {
                moves.push(Move::new(sq, fwd_sq, PieceType::Pawn));
            }

            // Double step (only if forward square is empty and pawn is on starting position)
            if is_on_start {
                let dbl_file = fwd_file + df;
                let dbl_rank = fwd_rank + dr;
                if in_bounds(dbl_file, dbl_rank) {
                    let dbl_sq = square_from(dbl_file as u8, dbl_rank as u8).unwrap();
                    if is_valid_square(dbl_sq) && board.piece_at(dbl_sq).is_none() {
                        moves.push(Move::new_double_push(sq, dbl_sq));
                    }
                }
            }
        }
    }

    // Captures (diagonal in the pawn's forward direction)
    // Terrain pieces cannot be captured — skip them.
    for &target_sq in tables.pawn_attack_squares(pidx, sq) {
        if let Some(target_piece) = board.piece_at(target_sq) {
            if target_piece.owner != player && !target_piece.is_terrain() {
                // Check if this is a promotion capture
                let target_file = file_of(target_sq);
                let target_rank = rank_of(target_sq);
                let is_promotion = match pidx {
                    0 | 2 => target_rank == promo_coord,
                    1 | 3 => target_file == promo_coord,
                    _ => unreachable!(),
                };

                if is_promotion {
                    moves.push(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::PromotedQueen,
                    ));
                    moves.push(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::Knight,
                    ));
                    moves.push(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::Rook,
                    ));
                    moves.push(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::Bishop,
                    ));
                } else {
                    moves.push(Move::new_capture(
                        sq,
                        target_sq,
                        PieceType::Pawn,
                        target_piece.piece_type,
                    ));
                }
            }
        }

        // En passant capture
        if let Some(ep_sq) = board.en_passant() {
            if target_sq == ep_sq {
                moves.push(Move::new_en_passant(sq, ep_sq));
            }
        }
    }
}

/// Generate knight moves.
fn generate_knight_moves(
    board: &Board,
    player: Player,
    sq: Square,
    tables: &AttackTables,
    moves: &mut Vec<Move>,
) {
    for &target in tables.knight_destinations(sq) {
        match board.piece_at(target) {
            None => moves.push(Move::new(sq, target, PieceType::Knight)),
            Some(piece) if piece.is_terrain() => {} // Terrain — impassable
            Some(piece) if piece.owner != player => moves.push(Move::new_capture(
                sq,
                target,
                PieceType::Knight,
                piece.piece_type,
            )),
            _ => {} // Own piece — blocked
        }
    }
}

/// Generate sliding piece moves (bishop, rook, queen, promoted queen).
#[allow(clippy::too_many_arguments)]
fn generate_sliding_moves(
    board: &Board,
    player: Player,
    sq: Square,
    piece_type: PieceType,
    tables: &AttackTables,
    moves: &mut Vec<Move>,
    diagonals: bool,
    orthogonals: bool,
) {
    for dir in 0..NUM_DIRECTIONS {
        let is_diag = dir % 2 == 1;
        if is_diag && !diagonals {
            continue;
        }
        if !is_diag && !orthogonals {
            continue;
        }

        for &target in tables.ray(sq, dir) {
            match board.piece_at(target) {
                None => moves.push(Move::new(sq, target, piece_type)),
                Some(piece) if piece.is_terrain() => break, // Terrain — blocks ray
                Some(piece) if piece.owner != player => {
                    moves.push(Move::new_capture(sq, target, piece_type, piece.piece_type));
                    break; // Can't go past a capture
                }
                _ => break, // Own piece — blocked
            }
        }
    }
}

/// Generate king moves (non-castling).
fn generate_king_moves(
    board: &Board,
    player: Player,
    sq: Square,
    tables: &AttackTables,
    moves: &mut Vec<Move>,
) {
    for &target in tables.king_destinations(sq) {
        match board.piece_at(target) {
            None => moves.push(Move::new(sq, target, PieceType::King)),
            Some(piece) if piece.is_terrain() => {} // Terrain — impassable
            Some(piece) if piece.owner != player => moves.push(Move::new_capture(
                sq,
                target,
                PieceType::King,
                piece.piece_type,
            )),
            _ => {} // Own piece — blocked
        }
    }
}

/// Generate castling moves.
fn generate_castling(board: &Board, player: Player, moves: &mut Vec<Move>) {
    let rights = board.castling_rights();
    let (
        king_sq,
        _ks_rook,
        _qs_rook,
        king_target_ks,
        _rook_target_ks,
        king_target_qs,
        _rook_target_qs,
        ks_bit,
        qs_bit,
    ) = get_castling_config(player);

    // Kingside
    if rights & ks_bit != 0 {
        let empty = castling_empty_squares(player, true);
        let path = castling_king_path(player, true);

        // All squares between king and rook must be empty
        let all_empty = empty.iter().all(|&s| board.piece_at(s).is_none());

        // King must not be in check, and must not pass through check
        let path_safe = all_empty
            && path.iter().all(|&s| {
                !Player::ALL
                    .iter()
                    .any(|&opp| opp != player && is_square_attacked_by(s, opp, board))
            });

        if path_safe {
            moves.push(Move::new_castle_king(king_sq, king_target_ks));
        }
    }

    // Queenside
    if rights & qs_bit != 0 {
        let empty = castling_empty_squares(player, false);
        let path = castling_king_path(player, false);

        let all_empty = empty.iter().all(|&s| board.piece_at(s).is_none());

        let path_safe = all_empty
            && path.iter().all(|&s| {
                !Player::ALL
                    .iter()
                    .any(|&opp| opp != player && is_square_attacked_by(s, opp, board))
            });

        if path_safe {
            moves.push(Move::new_castle_queen(king_sq, king_target_qs));
        }
    }
}

/// Perft: recursive move count at given depth. Returns total leaf node count.
pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = generate_legal(board);

    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0u64;
    for mv in moves {
        let undo = make_move(board, mv);
        nodes += perft(board, depth - 1);
        unmake_move(board, mv, undo);
    }
    nodes
}

/// Divide perft: show node count for each move at root. Useful for debugging.
pub fn perft_divide(board: &mut Board, depth: u32) -> Vec<(Move, u64)> {
    let moves = generate_legal(board);
    let mut results = Vec::new();

    for mv in moves {
        let undo = make_move(board, mv);
        let nodes = if depth <= 1 {
            1
        } else {
            perft(board, depth - 1)
        };
        unmake_move(board, mv, undo);
        results.push((mv, nodes));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{square_from, Board, Piece, PieceType, Player};

    #[test]
    fn test_pawn_single_step_red() {
        let mut board = Board::empty();
        // Red pawn on e2 (file 4, rank 1)
        let sq = square_from(4, 1).unwrap();
        board.place_piece(sq, Piece::new(PieceType::Pawn, Player::Red));
        // Need a king for legal move gen to not crash
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );

        let moves = generate_pseudo_legal(&board);
        let pawn_moves: Vec<_> = moves.iter().filter(|m| m.from_sq() == sq).collect();

        // Should have single step (e3) and double step (e4)
        assert!(pawn_moves.len() >= 2, "pawn should have at least 2 moves");
    }

    #[test]
    fn test_pawn_blocked() {
        let mut board = Board::empty();
        let pawn_sq = square_from(4, 1).unwrap();
        let blocker = square_from(4, 2).unwrap();
        board.place_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        board.place_piece(blocker, Piece::new(PieceType::Pawn, Player::Blue));
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );

        let moves = generate_pseudo_legal(&board);
        let pawn_fwd: Vec<_> = moves
            .iter()
            .filter(|m| m.from_sq() == pawn_sq && !m.is_capture())
            .collect();

        assert!(
            pawn_fwd.is_empty(),
            "pawn blocked by piece should not move forward"
        );
    }

    #[test]
    fn test_knight_moves_from_center() {
        let mut board = Board::empty();
        let sq = square_from(6, 6).unwrap();
        board.place_piece(sq, Piece::new(PieceType::Knight, Player::Red));
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );

        let moves = generate_pseudo_legal(&board);
        let knight_moves: Vec<_> = moves.iter().filter(|m| m.from_sq() == sq).collect();
        assert_eq!(knight_moves.len(), 8, "center knight should have 8 moves");
    }

    #[test]
    fn test_rook_moves_empty_board() {
        let mut board = Board::empty();
        let sq = square_from(6, 6).unwrap();
        board.place_piece(sq, Piece::new(PieceType::Rook, Player::Red));
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );

        let moves = generate_pseudo_legal(&board);
        let rook_moves: Vec<_> = moves.iter().filter(|m| m.from_sq() == sq).collect();
        // From g7 center: N=7, S=6, E=7, W=6 = 26 moves
        assert_eq!(rook_moves.len(), 26, "center rook should have 26 moves");
    }

    #[test]
    fn test_starting_position_move_count() {
        let board = Board::starting_position();
        let moves = generate_pseudo_legal(&board);
        // Red has 8 pawns with 2 moves each (single + double) = 16
        // Plus knight moves: 2 knights, each with some moves from starting position
        // Total should be reasonable (around 20-30 for starting position)
        assert!(
            moves.len() >= 16,
            "starting position should have at least 16 pawn moves, got {}",
            moves.len()
        );
    }

    #[test]
    fn test_legal_moves_starting_position() {
        let mut board = Board::starting_position();
        let legal = generate_legal(&mut board);
        // Starting position should have legal moves and zobrist should be unchanged
        assert!(!legal.is_empty(), "starting position must have legal moves");
        assert!(board.verify_zobrist());
        assert!(board.verify_piece_lists());
    }

    #[test]
    fn test_perft_depth_0() {
        let mut board = Board::starting_position();
        assert_eq!(perft(&mut board, 0), 1);
    }

    #[test]
    fn test_perft_depth_1() {
        let mut board = Board::starting_position();
        let count = perft(&mut board, 1);
        // Red's moves from starting position
        assert!(count > 0, "perft(1) should be > 0");
        // Verify board is restored
        assert!(board.verify_zobrist());
        assert!(board.verify_piece_lists());
        assert_eq!(board.piece_count(), 64);
    }

    #[test]
    fn test_zobrist_restored_after_legal_gen() {
        let mut board = Board::starting_position();
        let hash_before = board.zobrist();
        let _moves = generate_legal(&mut board);
        assert_eq!(
            board.zobrist(),
            hash_before,
            "zobrist must be restored after legal move generation"
        );
    }

    #[test]
    fn test_en_passant_move_generated() {
        let mut board = Board::empty();
        // Set up en passant scenario: Red pawn on e5, Yellow pawn just double-stepped d7->d5
        // ep target = d6
        board.place_piece(
            square_from(4, 4).unwrap(), // e5
            Piece::new(PieceType::Pawn, Player::Red),
        );
        // Yellow pawn on d5 (just double-stepped)
        board.place_piece(
            square_from(3, 4).unwrap(), // d5
            Piece::new(PieceType::Pawn, Player::Yellow),
        );
        // Kings
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.place_piece(
            square_from(6, 13).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );

        // Set en passant target (d6 = file 3, rank 5... wait)
        // Yellow's double step was d13->d11 (rank 12 -> rank 10). EP target = d12 (rank 11).
        // But I set up the pawns differently. Let me just test the basic mechanic.
        // Actually, for Red to capture ep, the ep target must be diagonally forward from Red.
        // Red is at e5 (4, 4), captures diag to d6 (3, 5). So ep target is d6 (3, 5).
        // The captured pawn would be at d5 (3, 4) — backward from Red's perspective.
        let ep_target = square_from(3, 5).unwrap(); // d6
        board.set_en_passant(Some(ep_target));

        let moves = generate_pseudo_legal(&board);
        let ep_moves: Vec<_> = moves.iter().filter(|m| m.is_en_passant()).collect();
        assert_eq!(ep_moves.len(), 1, "should have exactly 1 en passant move");
        assert_eq!(ep_moves[0].to_sq(), ep_target);
    }

    #[test]
    fn test_pawn_promotion_red() {
        let mut board = Board::empty();
        // Red pawn on e8 (file 4, rank 7) — one step from promotion rank 8
        let pawn_sq = square_from(4, 7).unwrap();
        board.place_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );

        let moves = generate_pseudo_legal(&board);
        let promo_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.from_sq() == pawn_sq && m.is_promotion())
            .collect();
        // Should have 4 promotion options (PromotedQueen, Knight, Rook, Bishop)
        assert_eq!(promo_moves.len(), 4, "should have 4 promotion options");
    }

    #[test]
    fn test_blue_pawn_forward() {
        let mut board = Board::empty();
        // Blue pawn on b5 (file 1, rank 4) — moves +file
        let pawn_sq = square_from(1, 4).unwrap();
        board.place_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Blue));
        board.place_piece(
            square_from(0, 6).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.set_side_to_move(Player::Blue);

        let moves = generate_pseudo_legal(&board);
        let pawn_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.from_sq() == pawn_sq && !m.is_capture())
            .collect();

        // Single step to c5 (file 2, rank 4) + double step to d5 (file 3, rank 4)
        assert_eq!(
            pawn_moves.len(),
            2,
            "Blue pawn on starting file should have 2 forward moves"
        );

        // Verify the single step is +file
        let single = pawn_moves
            .iter()
            .find(|m| !m.is_double_push())
            .expect("should have single step");
        assert_eq!(single.to_sq(), square_from(2, 4).unwrap()); // c5
    }

    #[test]
    fn test_green_pawn_forward() {
        let mut board = Board::empty();
        // Green pawn on m5 (file 12, rank 4) — moves -file
        let pawn_sq = square_from(12, 4).unwrap();
        board.place_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Green));
        board.place_piece(
            square_from(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Green);

        let moves = generate_pseudo_legal(&board);
        let pawn_moves: Vec<_> = moves
            .iter()
            .filter(|m| m.from_sq() == pawn_sq && !m.is_capture())
            .collect();

        // Single step to l5 (file 11, rank 4) + double step to k5 (file 10, rank 4)
        assert_eq!(
            pawn_moves.len(),
            2,
            "Green pawn on starting file should have 2 forward moves"
        );
    }
}
