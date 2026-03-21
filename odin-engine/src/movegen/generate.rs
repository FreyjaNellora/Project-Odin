// Move generation: pseudo-legal and legal moves.
//
// Pseudo-legal generation produces all candidate moves without checking
// if they leave the king in check. Legal filtering applies each move
// and verifies the king's safety.

use arrayvec::ArrayVec;

use crate::board::{
    file_of, is_valid_square, rank_of, square_from, Board, PieceType, Player, Square,
};

use super::attacks::{is_in_check, is_square_attacked_by};
use super::moves::{
    castling_empty_squares, castling_king_path, find_ep_captured_pawn_sq, get_castling_config,
    make_move, unmake_move, Move,
};
use super::tables::{global_attack_tables, AttackTables, NUM_DIRECTIONS};

use crate::board::BOARD_SIZE;

/// Maximum moves per position. 256 is safe for 4-player 14×14 chess.
pub const MAX_MOVES: usize = 256;

/// Trait for push-compatible move buffers (Vec<Move> and ArrayVec<Move, N>).
/// Monomorphized at compile time — zero runtime cost.
pub trait MoveBuffer {
    fn push_move(&mut self, mv: Move);
}

impl MoveBuffer for Vec<Move> {
    #[inline]
    fn push_move(&mut self, mv: Move) {
        self.push(mv);
    }
}

impl MoveBuffer for ArrayVec<Move, MAX_MOVES> {
    #[inline]
    fn push_move(&mut self, mv: Move) {
        self.push(mv);
    }
}

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
    let mut moves = Vec::new();
    generate_pseudo_legal_generic(board, &mut moves);
    moves
}

/// Generate pseudo-legal moves into any MoveBuffer (Vec or ArrayVec).
fn generate_pseudo_legal_generic(board: &Board, moves: &mut impl MoveBuffer) {
    let tables = global_attack_tables();
    let player = board.side_to_move();

    for &(piece_type, sq) in board.piece_list(player) {
        match piece_type {
            PieceType::Pawn => generate_pawn_moves(board, player, sq, moves),
            PieceType::Knight => generate_knight_moves(board, player, sq, tables, moves),
            PieceType::Bishop => generate_sliding_moves(
                board, player, sq, piece_type, tables, moves, true, false,
            ),
            PieceType::Rook => generate_sliding_moves(
                board, player, sq, piece_type, tables, moves, false, true,
            ),
            PieceType::Queen | PieceType::PromotedQueen => generate_sliding_moves(
                board, player, sq, piece_type, tables, moves, true, true,
            ),
            PieceType::King => generate_king_moves(board, player, sq, tables, moves),
        }
    }

    generate_castling(board, player, moves);
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

/// Generate all legal moves into a stack-allocated ArrayVec (zero heap allocation).
pub fn generate_legal_into(board: &mut Board, out: &mut ArrayVec<Move, MAX_MOVES>) {
    let mut pseudo = ArrayVec::<Move, MAX_MOVES>::new();
    generate_pseudo_legal_generic(board, &mut pseudo);
    let player = board.side_to_move();

    for mv in pseudo {
        let undo = make_move(board, mv);
        if !is_in_check(player, board) {
            out.push(mv);
        }
        unmake_move(board, mv, undo);
    }
}

/// Generate only legal capture moves into a stack-allocated ArrayVec (for quiescence search).
pub fn generate_legal_captures_into(board: &mut Board, out: &mut ArrayVec<Move, MAX_MOVES>) {
    let mut pseudo = ArrayVec::<Move, MAX_MOVES>::new();
    generate_pseudo_legal_generic(board, &mut pseudo);
    let player = board.side_to_move();

    for mv in pseudo {
        if !mv.is_capture() {
            continue;
        }
        let undo = make_move(board, mv);
        if !is_in_check(player, board) {
            out.push(mv);
        }
        unmake_move(board, mv, undo);
    }
}

/// Generate pawn moves for a single pawn.
fn generate_pawn_moves(board: &Board, player: Player, sq: Square, moves: &mut impl MoveBuffer) {
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
                moves.push_move(Move::new_promotion(
                    sq,
                    fwd_sq,
                    None,
                    PieceType::PromotedQueen,
                ));
                // Also allow underpromotion to knight, bishop, rook
                moves.push_move(Move::new_promotion(sq, fwd_sq, None, PieceType::Knight));
                moves.push_move(Move::new_promotion(sq, fwd_sq, None, PieceType::Rook));
                moves.push_move(Move::new_promotion(sq, fwd_sq, None, PieceType::Bishop));
            } else {
                moves.push_move(Move::new(sq, fwd_sq, PieceType::Pawn));
            }

            // Double step (only if forward square is empty and pawn is on starting position)
            if is_on_start {
                let dbl_file = fwd_file + df;
                let dbl_rank = fwd_rank + dr;
                if in_bounds(dbl_file, dbl_rank) {
                    let dbl_sq = square_from(dbl_file as u8, dbl_rank as u8).unwrap();
                    if is_valid_square(dbl_sq) && board.piece_at(dbl_sq).is_none() {
                        moves.push_move(Move::new_double_push(sq, dbl_sq));
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
                    moves.push_move(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::PromotedQueen,
                    ));
                    moves.push_move(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::Knight,
                    ));
                    moves.push_move(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::Rook,
                    ));
                    moves.push_move(Move::new_promotion(
                        sq,
                        target_sq,
                        Some(target_piece.piece_type),
                        PieceType::Bishop,
                    ));
                } else {
                    moves.push_move(Move::new_capture(
                        sq,
                        target_sq,
                        PieceType::Pawn,
                        target_piece.piece_type,
                    ));
                }
            }
        }

        // En passant capture: only generate if an enemy pawn actually exists
        // near ep_target. This prevents an invalid self-capture when the player
        // who just double-pushed is later tested for checkmate in
        // check_elimination_chain (their own pawn set ep_sq, but the scan
        // skips the current player, finds nothing, and the fallback crashes).
        if let Some(ep_sq) = board.en_passant() {
            if target_sq == ep_sq && find_ep_captured_pawn_sq(board, ep_sq, player).is_some() {
                moves.push_move(Move::new_en_passant(sq, ep_sq));
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
    moves: &mut impl MoveBuffer,
) {
    for &target in tables.knight_destinations(sq) {
        match board.piece_at(target) {
            None => moves.push_move(Move::new(sq, target, PieceType::Knight)),
            Some(piece) if piece.is_terrain() => {} // Terrain — impassable
            Some(piece) if piece.owner != player => moves.push_move(Move::new_capture(
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
    moves: &mut impl MoveBuffer,
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
                None => moves.push_move(Move::new(sq, target, piece_type)),
                Some(piece) if piece.is_terrain() => break, // Terrain — blocks ray
                Some(piece) if piece.owner != player => {
                    moves.push_move(Move::new_capture(sq, target, piece_type, piece.piece_type));
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
    moves: &mut impl MoveBuffer,
) {
    for &target in tables.king_destinations(sq) {
        match board.piece_at(target) {
            None => moves.push_move(Move::new(sq, target, PieceType::King)),
            Some(piece) if piece.is_terrain() => {} // Terrain — impassable
            Some(piece) if piece.owner != player => moves.push_move(Move::new_capture(
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
fn generate_castling(board: &Board, player: Player, moves: &mut impl MoveBuffer) {
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
    ) = get_castling_config(player, board);

    // Kingside
    if rights & ks_bit != 0 {
        let empty = castling_empty_squares(player, true, board);
        let path = castling_king_path(player, true, board);

        // All squares on the travel paths must be empty
        let all_empty = empty.iter().all(|&s| board.piece_at(s).is_none());

        // King must not be in check, and must not pass through check
        let path_safe = all_empty
            && path.iter().all(|&s| {
                !Player::ALL
                    .iter()
                    .any(|&opp| opp != player && is_square_attacked_by(s, opp, board))
            });

        if path_safe {
            moves.push_move(Move::new_castle_king(king_sq, king_target_ks));
        }
    }

    // Queenside
    if rights & qs_bit != 0 {
        let empty = castling_empty_squares(player, false, board);
        let path = castling_king_path(player, false, board);

        let all_empty = empty.iter().all(|&s| board.piece_at(s).is_none());

        let path_safe = all_empty
            && path.iter().all(|&s| {
                !Player::ALL
                    .iter()
                    .any(|&opp| opp != player && is_square_attacked_by(s, opp, board))
            });

        if path_safe {
            moves.push_move(Move::new_castle_queen(king_sq, king_target_qs));
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

    /// Exhaustive EP test for all 8 adjacent-player-corner combinations.
    /// For each pair (capturer, pusher) near each corner, we walk the capturer
    /// along the corner edge and verify EP is generated for every valid position.
    ///
    /// The 8 pairs (capturer → pusher, corner):
    ///   Red → Blue  (SW)    Blue → Red  (SW)
    ///   Blue → Yellow (NW)  Yellow → Blue (NW)
    ///   Yellow → Green (NE) Green → Yellow (NE)
    ///   Green → Red (SE)    Red → Green (SE)
    #[test]
    fn test_ep_near_all_corners_exhaustive() {
        // For each test case: (capturer, pusher, capturer's king, pusher's king,
        //   list of (capturer_sq, pusher_from, pusher_to, ep_target))
        //
        // Pawn configs:
        //   Red:    moves +rank, starts rank 1, double to rank 3, captures at (±1 file, +1 rank)
        //   Blue:   moves +file, starts file 1, double to file 3, captures at (+1 file, ±1 rank)
        //   Yellow: moves -rank, starts rank 12, double to rank 10, captures at (±1 file, -1 rank)
        //   Green:  moves -file, starts file 12, double to file 10, captures at (-1 file, ±1 rank)

        struct EpCase {
            capturer: Player,
            pusher: Player,
            capturer_sq: (u8, u8),  // (file, rank)
            pusher_from: (u8, u8),
            pusher_to: (u8, u8),
            ep_target: (u8, u8),
            label: &'static str,
        }

        let mut cases: Vec<EpCase> = Vec::new();

        // === SW CORNER (files 0-2, ranks 0-2) ===

        // Red captures Blue's eastward push (Blue pushes +file from file 1 to file 3)
        // Blue pawns on file 1 (start), ranks 3-10. Double push to file 3. EP target file 2.
        // Red pawn must be on file 3 (adjacent to Blue's landing), at rank where Red can
        // capture diagonally to (file 2, rank±0) matching the EP target rank.
        // Red captures at (-1 file, +1 rank), so Red at (3, ep_rank-1) captures to (2, ep_rank).
        for r in 3..=10u8 {
            // Blue pushes from (1, r) to (3, r), ep target (2, r)
            // Red at (3, r-1) captures NW to (2, r) — only if r-1 >= 0 and (3, r-1) is valid
            if r >= 1 {
                let capturer_rank = r - 1;
                if is_valid_square(square_from(3, capturer_rank).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Red,
                        pusher: Player::Blue,
                        capturer_sq: (3, capturer_rank),
                        pusher_from: (1, r),
                        pusher_to: (3, r),
                        ep_target: (2, r),
                        label: "Red→Blue SW",
                    });
                }
            }
        }

        // Blue captures Red's northward push (Red pushes +rank from rank 1 to rank 3)
        // Red pawns on rank 1 (start), files 3-10. Double push to rank 3. EP target rank 2.
        // Blue captures at (+1 file, ±1 rank), so Blue at (file-1, rank±1) captures to (file, 2).
        // Blue at (ep_file-1, 2+1) = (ep_file-1, 3) captures to (ep_file, 2) via (+1, -1)
        for f in 3..=10u8 {
            // Red pushes from (f, 1) to (f, 3), ep target (f, 2)
            // Blue at (f-1, 3) captures via (+1, -1) to (f, 2)
            if f >= 1 {
                let capturer_file = f - 1;
                if is_valid_square(square_from(capturer_file, 3).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Blue,
                        pusher: Player::Red,
                        capturer_sq: (capturer_file, 3),
                        pusher_from: (f, 1),
                        pusher_to: (f, 3),
                        ep_target: (f, 2),
                        label: "Blue→Red SW",
                    });
                }
            }
        }

        // === NW CORNER (files 0-2, ranks 11-13) ===

        // Blue captures Yellow's southward push (Yellow pushes -rank from rank 12 to rank 10)
        // Yellow pawns on rank 12, files 3-10. EP target rank 11.
        // Blue captures at (+1 file, +1 rank) to (ep_file, 11), Blue at (ep_file-1, 10)
        for f in 3..=10u8 {
            // Yellow pushes from (f, 12) to (f, 10), ep target (f, 11)
            // Blue at (f-1, 10) captures via (+1, +1) to (f, 11)
            if f >= 1 {
                let capturer_file = f - 1;
                if is_valid_square(square_from(capturer_file, 10).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Blue,
                        pusher: Player::Yellow,
                        capturer_sq: (capturer_file, 10),
                        pusher_from: (f, 12),
                        pusher_to: (f, 10),
                        ep_target: (f, 11),
                        label: "Blue→Yellow NW",
                    });
                }
            }
        }

        // Yellow captures Blue's eastward push near NW corner
        // Blue pushes from file 1 to file 3, ranks 3-10. EP target file 2.
        // Yellow captures at (+1 file, -1 rank), so Yellow at (1, ep_rank+1) captures to (2, ep_rank)
        for r in 3..=10u8 {
            // Blue pushes from (1, r) to (3, r), ep target (2, r)
            // Yellow at (1, r+1) captures via (+1, -1) to (2, r) — only if r+1 <= 13
            if r + 1 <= 13 {
                let capturer_rank = r + 1;
                if is_valid_square(square_from(1, capturer_rank).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Yellow,
                        pusher: Player::Blue,
                        capturer_sq: (1, capturer_rank),
                        pusher_from: (1, r),
                        pusher_to: (3, r),
                        ep_target: (2, r),
                        label: "Yellow→Blue NW",
                    });
                }
            }
        }

        // === NE CORNER (files 11-13, ranks 11-13) ===

        // Yellow captures Green's westward push (Green pushes -file from file 12 to file 10)
        // Green pawns on file 12, ranks 3-10. EP target file 11.
        // Yellow captures at (-1 file, -1 rank), so Yellow at (12, ep_rank+1) captures to (11, ep_rank)
        for r in 3..=10u8 {
            // Green pushes from (12, r) to (10, r), ep target (11, r)
            // Yellow at (12, r+1) captures via (-1, -1) to (11, r) — only if r+1 <= 13
            if r + 1 <= 13 {
                let capturer_rank = r + 1;
                if is_valid_square(square_from(12, capturer_rank).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Yellow,
                        pusher: Player::Green,
                        capturer_sq: (12, capturer_rank),
                        pusher_from: (12, r),
                        pusher_to: (10, r),
                        ep_target: (11, r),
                        label: "Yellow→Green NE",
                    });
                }
            }
        }

        // Green captures Yellow's southward push near NE corner
        // Yellow pushes from rank 12 to rank 10, files 3-10. EP target rank 11.
        // Green captures at (-1 file, -1 rank), so Green at (ep_file+1, 12) captures to (ep_file, 11)
        for f in 3..=10u8 {
            // Yellow pushes from (f, 12) to (f, 10), ep target (f, 11)
            // Green at (f+1, 12) captures via (-1, -1) to (f, 11) — only if f+1 <= 13
            if f + 1 <= 13 {
                let capturer_file = f + 1;
                if is_valid_square(square_from(capturer_file, 12).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Green,
                        pusher: Player::Yellow,
                        capturer_sq: (capturer_file, 12),
                        pusher_from: (f, 12),
                        pusher_to: (f, 10),
                        ep_target: (f, 11),
                        label: "Green→Yellow NE",
                    });
                }
            }
        }

        // === SE CORNER (files 11-13, ranks 0-2) ===

        // Green captures Red's northward push near SE corner
        // Red pushes from rank 1 to rank 3, files 3-10. EP target rank 2.
        // Green captures at (-1 file, +1 rank), so Green at (ep_file+1, 1) captures to (ep_file, 2)
        for f in 3..=10u8 {
            // Red pushes from (f, 1) to (f, 3), ep target (f, 2)
            // Green at (f+1, 1) captures via (-1, +1) to (f, 2) — only if f+1 <= 13
            if f + 1 <= 13 {
                let capturer_file = f + 1;
                if is_valid_square(square_from(capturer_file, 1).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Green,
                        pusher: Player::Red,
                        capturer_sq: (capturer_file, 1),
                        pusher_from: (f, 1),
                        pusher_to: (f, 3),
                        ep_target: (f, 2),
                        label: "Green→Red SE",
                    });
                }
            }
        }

        // Red captures Green's westward push near SE corner
        // Green pushes from file 12 to file 10, ranks 3-10. EP target file 11.
        // Red captures at (+1 file, +1 rank), so Red at (12, ep_rank-1) captures to (11, ep_rank)... wait
        // Red captures at (-1, +1) and (+1, +1). To reach (11, r), Red at (12, r-1) via (-1, +1).
        for r in 3..=10u8 {
            // Green pushes from (12, r) to (10, r), ep target (11, r)
            // Red at (12, r-1) captures via (-1, +1) to (11, r) — only if r-1 >= 0
            if r >= 1 {
                let capturer_rank = r - 1;
                if is_valid_square(square_from(12, capturer_rank).unwrap()) {
                    cases.push(EpCase {
                        capturer: Player::Red,
                        pusher: Player::Green,
                        capturer_sq: (12, capturer_rank),
                        pusher_from: (12, r),
                        pusher_to: (10, r),
                        ep_target: (11, r),
                        label: "Red→Green SE",
                    });
                }
            }
        }

        // Now run all cases
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for case in &cases {
            // Validate all squares exist and are valid
            let cap_sq = match square_from(case.capturer_sq.0, case.capturer_sq.1) {
                Some(s) if is_valid_square(s) => s,
                _ => { skipped += 1; continue; }
            };
            let push_from = match square_from(case.pusher_from.0, case.pusher_from.1) {
                Some(s) if is_valid_square(s) => s,
                _ => { skipped += 1; continue; }
            };
            let push_to = match square_from(case.pusher_to.0, case.pusher_to.1) {
                Some(s) if is_valid_square(s) => s,
                _ => { skipped += 1; continue; }
            };
            let ep_sq = match square_from(case.ep_target.0, case.ep_target.1) {
                Some(s) if is_valid_square(s) => s,
                _ => { skipped += 1; continue; }
            };

            // Build board: capturer pawn, pusher pawn at landing, kings, EP set
            let mut board = Board::empty();
            board.place_piece(cap_sq, Piece::new(PieceType::Pawn, case.capturer));
            board.place_piece(push_to, Piece::new(PieceType::Pawn, case.pusher));

            // Place kings far from the action
            board.place_piece(
                square_from(7, 0).unwrap(),
                Piece::new(PieceType::King, Player::Red),
            );
            board.place_piece(
                square_from(0, 7).unwrap(),
                Piece::new(PieceType::King, Player::Blue),
            );
            board.place_piece(
                square_from(7, 13).unwrap(),
                Piece::new(PieceType::King, Player::Yellow),
            );
            board.place_piece(
                square_from(13, 7).unwrap(),
                Piece::new(PieceType::King, Player::Green),
            );

            board.set_en_passant(Some(ep_sq));
            board.set_side_to_move(case.capturer);

            let moves = generate_pseudo_legal(&board);
            let ep_moves: Vec<_> = moves.iter().filter(|m| m.is_en_passant()).collect();

            if ep_moves.len() == 1 && ep_moves[0].to_sq() == ep_sq {
                passed += 1;
            } else {
                failed += 1;
                eprintln!(
                    "FAILED {}: capturer {:?} at ({},{}), pusher {:?} ({},{})→({},{}), ep ({},{}), got {} ep moves",
                    case.label,
                    case.capturer, case.capturer_sq.0, case.capturer_sq.1,
                    case.pusher, case.pusher_from.0, case.pusher_from.1,
                    case.pusher_to.0, case.pusher_to.1,
                    case.ep_target.0, case.ep_target.1,
                    ep_moves.len(),
                );
            }
        }

        eprintln!(
            "EP corner test: {} passed, {} failed, {} skipped (invalid squares)",
            passed, failed, skipped
        );
        assert_eq!(failed, 0, "{} EP corner cases failed", failed);
        assert!(passed > 0, "no EP cases were tested");
    }
}
