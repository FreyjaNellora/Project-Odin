// Game rules: check detection, checkmate/stalemate, DKW, terrain conversion.

use crate::board::{Board, PieceStatus, PieceType, Player, Square};
use crate::movegen::{generate_legal, is_in_check, is_square_attacked_by, Move};

use super::PlayerStatus;

/// Result of evaluating a player's position when their turn arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnDetermination {
    /// Player has legal moves — normal turn.
    HasMoves,
    /// Player has no legal moves and is in check — checkmate.
    Checkmate,
    /// Player has no legal moves and is NOT in check — stalemate.
    Stalemate,
}

/// Determine whether a player is checkmated, stalemated, or has moves.
/// Only called when the player's turn arrives.
pub fn determine_status_at_turn(board: &mut Board, player: Player) -> TurnDetermination {
    // Ensure we're generating moves for the right player
    let saved_stm = board.side_to_move();
    board.set_side_to_move(player);

    let legal = generate_legal(board);
    board.set_side_to_move(saved_stm);

    if legal.is_empty() {
        if is_in_check(player, board) {
            TurnDetermination::Checkmate
        } else {
            TurnDetermination::Stalemate
        }
    } else {
        TurnDetermination::HasMoves
    }
}

/// Find which active opponent kings are checked after a move by the given player.
/// Returns the list of checked players.
pub fn kings_checked_by_move(
    board: &Board,
    attacker: Player,
    player_statuses: &[PlayerStatus; 4],
) -> Vec<Player> {
    let mut checked = Vec::new();
    for &opponent in &Player::ALL {
        if opponent == attacker {
            continue;
        }
        // Only check Active and DeadKingWalking players (they still have kings)
        if player_statuses[opponent.index()] == PlayerStatus::Eliminated {
            continue;
        }
        let king_sq = board.king_square(opponent);
        if is_square_attacked_by(king_sq, attacker, board) {
            checked.push(opponent);
        }
    }
    checked
}

/// Generate a random DKW king move.
/// Returns None if the DKW king has no legal moves (stuck).
pub fn generate_dkw_move(board: &mut Board, player: Player, seed: &mut u64) -> Option<Move> {
    // Save and set side to move
    let saved_stm = board.side_to_move();
    board.set_side_to_move(player);

    let legal = generate_legal(board);
    board.set_side_to_move(saved_stm);

    // Filter to king-only moves
    let king_moves: Vec<Move> = legal
        .into_iter()
        .filter(|m| m.piece_type() == PieceType::King && !m.is_castle())
        .collect();

    if king_moves.is_empty() {
        return None;
    }

    // LCG random selection
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let idx = (*seed >> 33) as usize % king_moves.len();
    Some(king_moves[idx])
}

/// Convert all of a player's pieces to terrain (immovable walls).
/// The king is removed from the board entirely.
pub fn convert_to_terrain(board: &mut Board, player: Player) {
    // Collect squares first to avoid borrow issues
    let pieces: Vec<(PieceType, Square)> = board
        .piece_list(player)
        .iter()
        .map(|&(pt, sq)| (pt, sq))
        .collect();

    for (pt, sq) in pieces {
        if pt == PieceType::King {
            board.remove_piece(sq);
        } else {
            board.set_piece_status(sq, PieceStatus::Terrain);
        }
    }
}

/// Convert all of a player's pieces to Dead status (DKW).
/// King remains — it will make random moves.
pub fn convert_to_dead(board: &mut Board, player: Player) {
    let squares: Vec<Square> = board.piece_list(player).iter().map(|&(_, sq)| sq).collect();

    for sq in squares {
        board.set_piece_status(sq, PieceStatus::Dead);
    }
}

/// Remove a player's king from the board (used after DKW king is captured or stuck).
pub fn remove_king(board: &mut Board, player: Player) {
    let king_sq = board.king_square(player);
    // Only remove if there's actually a piece there
    if board.piece_at(king_sq).is_some() {
        board.remove_piece(king_sq);
    }
}

/// Check if the game should end.
/// Returns Some(reason) if game over.
pub enum GameOverReason {
    /// Only 0 or 1 active player remains.
    LastPlayerStanding,
    /// Two active players, one leads by 21+ points.
    ClaimWin(Player),
    /// Draw by repetition or 50-move rule.
    Draw,
}

/// Check if draw by repetition (3-fold).
pub fn is_draw_by_repetition(position_history: &[u64], current_hash: u64) -> bool {
    let count = position_history
        .iter()
        .filter(|&&h| h == current_hash)
        .count();
    count >= 3
}

/// Check if draw by 50-move rule (200 half-moves for 4 players).
pub fn is_draw_by_fifty_moves(halfmove_clock: u16) -> bool {
    halfmove_clock >= 200
}

/// Check if any player can claim a win (21+ point lead with exactly 2 active).
pub fn check_claim_win(scores: &[i32; 4], player_statuses: &[PlayerStatus; 4]) -> Option<Player> {
    let active: Vec<(Player, i32)> = Player::ALL
        .iter()
        .filter(|&&p| player_statuses[p.index()] == PlayerStatus::Active)
        .map(|&p| (p, scores[p.index()]))
        .collect();

    if active.len() != 2 {
        return None;
    }

    let (p1, s1) = active[0];
    let (p2, s2) = active[1];

    if s1 - s2 >= super::scoring::CLAIM_WIN_LEAD {
        Some(p1)
    } else if s2 - s1 >= super::scoring::CLAIM_WIN_LEAD {
        Some(p2)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{square_from, Board, Piece, PieceType, Player};

    #[test]
    fn test_determine_status_has_moves() {
        let mut board = Board::starting_position();
        assert_eq!(
            determine_status_at_turn(&mut board, Player::Red),
            TurnDetermination::HasMoves
        );
    }

    #[test]
    fn test_kings_checked_none_at_start() {
        let board = Board::starting_position();
        let statuses = [PlayerStatus::Active; 4];
        let checked = kings_checked_by_move(&board, Player::Red, &statuses);
        assert!(checked.is_empty());
    }

    #[test]
    fn test_check_bonus_for_checked_kings() {
        let mut board = Board::empty();
        // Red queen on e5, Blue king on e8, Yellow king on e2 — both checked along file
        board.place_piece(
            square_from(4, 4).unwrap(),
            Piece::new(PieceType::Queen, Player::Red),
        );
        board.place_piece(
            square_from(4, 7).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.place_piece(
            square_from(4, 1).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.place_piece(
            square_from(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );

        let statuses = [PlayerStatus::Active; 4];
        let checked = kings_checked_by_move(&board, Player::Red, &statuses);
        assert_eq!(checked.len(), 2);
    }

    #[test]
    fn test_draw_by_repetition() {
        let history = vec![100, 200, 100, 300, 100];
        assert!(is_draw_by_repetition(&history, 100));
        assert!(!is_draw_by_repetition(&history, 200));
    }

    #[test]
    fn test_draw_by_fifty_moves() {
        assert!(!is_draw_by_fifty_moves(199));
        assert!(is_draw_by_fifty_moves(200));
        assert!(is_draw_by_fifty_moves(201));
    }

    #[test]
    fn test_claim_win() {
        let statuses = [
            PlayerStatus::Active,
            PlayerStatus::Active,
            PlayerStatus::Eliminated,
            PlayerStatus::Eliminated,
        ];

        // No claim if lead < 21
        let scores = [30, 10, 0, 0];
        assert!(check_claim_win(&scores, &statuses).is_none());

        // Claim if lead >= 21
        let scores = [31, 10, 0, 0];
        assert_eq!(check_claim_win(&scores, &statuses), Some(Player::Red));

        // Other player claims
        let scores = [10, 31, 0, 0];
        assert_eq!(check_claim_win(&scores, &statuses), Some(Player::Blue));
    }

    #[test]
    fn test_convert_to_terrain() {
        let mut board = Board::empty();
        board.place_piece(
            square_from(5, 5).unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );
        board.place_piece(
            square_from(6, 6).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.place_piece(
            square_from(4, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );

        convert_to_terrain(&mut board, Player::Blue);

        // Rook and pawn should be terrain
        assert!(board
            .piece_at(square_from(5, 5).unwrap())
            .unwrap()
            .is_terrain());
        assert!(board
            .piece_at(square_from(4, 4).unwrap())
            .unwrap()
            .is_terrain());
        // King should be removed
        assert!(board.piece_at(square_from(6, 6).unwrap()).is_none());
    }

    #[test]
    fn test_convert_to_dead() {
        let mut board = Board::empty();
        board.place_piece(
            square_from(5, 5).unwrap(),
            Piece::new(PieceType::Queen, Player::Green),
        );
        board.place_piece(
            square_from(6, 6).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );

        convert_to_dead(&mut board, Player::Green);

        let queen = board.piece_at(square_from(5, 5).unwrap()).unwrap();
        assert_eq!(queen.status, PieceStatus::Dead);
        let king = board.piece_at(square_from(6, 6).unwrap()).unwrap();
        assert_eq!(king.status, PieceStatus::Dead);
    }
}
