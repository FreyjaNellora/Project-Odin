// King safety heuristic for bootstrap evaluation.
//
// Components:
//   1. Pawn shield: friendly pawns in front of king (+35cp each, max 3 = 105cp total).
//   2. Open file penalty: no friendly pawn on a king-adjacent file within 3 forward ranks (-25cp each).
//   3. Attacker pressure: opponent attacks on king zone (-25cp base + -20cp per extra).
//
// Uses is_square_attacked_by (allocation-free) instead of attackers_of (returns Vec).

use crate::board::{file_of, is_valid_square, rank_of, Board, PieceStatus, PieceType, Player};
use crate::gamestate::PlayerStatus;
use crate::movegen::is_square_attacked_by;

/// Bonus per friendly pawn in the pawn shield (max 3 pawns, 150cp total).
const PAWN_SHIELD_BONUS: i16 = 50;

/// Base penalty when any opponent piece attacks the king zone.
const ATTACKER_BASE_PENALTY: i16 = 25;

/// Additional penalty per extra attack square hit (beyond the first).
const ATTACKER_EXTRA_PENALTY: i16 = 20;

/// Maximum pawn shield squares checked (3: forward-left, forward, forward-right).
const MAX_SHIELD_SQUARES: i16 = 3;

/// Penalty per open file adjacent to the king (no friendly pawn within 3 ranks forward).
const OPEN_KING_FILE_PENALTY: i16 = 40;

/// King zone: the 8 squares adjacent to the king plus the king square itself.
const ADJACENT_DELTAS: [(i8, i8); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// King safety score for a player. Positive = safe, negative = unsafe.
pub(crate) fn king_safety_score(
    board: &Board,
    player: Player,
    player_statuses: &[PlayerStatus; 4],
) -> i16 {
    let king_sq = board.king_square(player);
    let king_file = file_of(king_sq) as i8;
    let king_rank = rank_of(king_sq) as i8;

    let mut score: i16 = 0;

    // 1. Pawn shield: count friendly pawns in front of king.
    score = score.saturating_add(pawn_shield_score(board, player, king_file, king_rank));

    // 2. Open file penalty: penalise each of the 3 king-adjacent files
    //    that lacks a friendly pawn within 3 ranks forward.
    score = score.saturating_sub(open_file_penalty(board, player, king_file, king_rank));

    // 3. Attacker pressure from each active opponent.
    for &opp in &Player::ALL {
        if opp == player {
            continue;
        }
        if player_statuses[opp.index()] == PlayerStatus::Eliminated {
            continue;
        }

        let attacks = count_king_zone_attacks(board, king_sq, king_file, king_rank, opp);
        if attacks > 0 {
            let penalty = ATTACKER_BASE_PENALTY
                .saturating_add(ATTACKER_EXTRA_PENALTY.saturating_mul(attacks - 1));
            score = score.saturating_sub(penalty);
        }
    }

    score
}

/// Count friendly pawns in the pawn shield (3 squares immediately in front of king).
fn pawn_shield_score(board: &Board, player: Player, king_file: i8, king_rank: i8) -> i16 {
    let mut count: i16 = 0;

    let shield_squares = shield_squares_for_player(player, king_file, king_rank);

    for (f, r) in shield_squares {
        if !(0..14).contains(&f) || !(0..14).contains(&r) {
            continue;
        }
        let sq = (r as u8) * 14 + (f as u8);
        if !is_valid_square(sq) {
            continue;
        }
        if let Some(piece) = board.piece_at(sq) {
            if piece.piece_type == PieceType::Pawn
                && piece.owner == player
                && piece.status == PieceStatus::Alive
            {
                count += 1;
            }
        }
    }

    count.min(MAX_SHIELD_SQUARES) * PAWN_SHIELD_BONUS
}

/// Penalty for semi-open files near the king.
///
/// For each of the 3 king-adjacent files, if there is no friendly pawn
/// within 3 squares forward, apply OPEN_KING_FILE_PENALTY.
/// Starting at the shield square and scanning forward covers pushed pawns too.
fn open_file_penalty(board: &Board, player: Player, king_file: i8, king_rank: i8) -> i16 {
    let mut penalty: i16 = 0;
    let shield = shield_squares_for_player(player, king_file, king_rank);
    let (df, dr) = forward_delta(player);

    for &(sf, sr) in &shield {
        let mut found_pawn = false;
        for depth in 0..3i8 {
            let f = sf + df * depth;
            let r = sr + dr * depth;
            if !(0..14).contains(&f) || !(0..14).contains(&r) {
                break;
            }
            let sq = (r as u8) * 14 + (f as u8);
            if !is_valid_square(sq) {
                continue;
            }
            if let Some(piece) = board.piece_at(sq) {
                if piece.piece_type == PieceType::Pawn
                    && piece.owner == player
                    && piece.status == PieceStatus::Alive
                {
                    found_pawn = true;
                    break;
                }
            }
        }
        if !found_pawn {
            penalty += OPEN_KING_FILE_PENALTY;
        }
    }
    penalty
}

/// Per-player forward direction delta (file_delta, rank_delta).
/// Red advances by rank (+rank), Blue by file (+file),
/// Yellow back by rank (-rank), Green back by file (-file).
fn forward_delta(player: Player) -> (i8, i8) {
    match player {
        Player::Red => (0, 1),
        Player::Blue => (1, 0),
        Player::Yellow => (0, -1),
        Player::Green => (-1, 0),
    }
}

/// Get the 3 shield squares immediately in front of the king for a given player.
/// Red faces +rank, Blue faces +file, Yellow faces -rank, Green faces -file.
fn shield_squares_for_player(player: Player, king_file: i8, king_rank: i8) -> [(i8, i8); 3] {
    match player {
        Player::Red => [
            (king_file - 1, king_rank + 1),
            (king_file, king_rank + 1),
            (king_file + 1, king_rank + 1),
        ],
        Player::Blue => [
            (king_file + 1, king_rank - 1),
            (king_file + 1, king_rank),
            (king_file + 1, king_rank + 1),
        ],
        Player::Yellow => [
            (king_file - 1, king_rank - 1),
            (king_file, king_rank - 1),
            (king_file + 1, king_rank - 1),
        ],
        Player::Green => [
            (king_file - 1, king_rank - 1),
            (king_file - 1, king_rank),
            (king_file - 1, king_rank + 1),
        ],
    }
}

/// Count the number of king zone squares attacked by an opponent.
/// King zone = king square + 8 adjacent squares.
fn count_king_zone_attacks(
    board: &Board,
    king_sq: u8,
    king_file: i8,
    king_rank: i8,
    opponent: Player,
) -> i16 {
    let mut attacks: i16 = 0;

    // Check king square itself.
    if is_square_attacked_by(king_sq, opponent, board) {
        attacks += 1;
    }

    // Check 8 adjacent squares.
    for &(df, dr) in &ADJACENT_DELTAS {
        let f = king_file + df;
        let r = king_rank + dr;
        if !(0..14).contains(&f) || !(0..14).contains(&r) {
            continue;
        }
        let sq = (r as u8) * 14 + (f as u8);
        if is_valid_square(sq) && is_square_attacked_by(sq, opponent, board) {
            attacks += 1;
        }
    }

    attacks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn test_starting_position_king_safety_reasonable() {
        let board = Board::starting_position();
        let statuses = [PlayerStatus::Active; 4];

        for &player in &Player::ALL {
            let score = king_safety_score(&board, player, &statuses);
            // At start, all 3 shield pawns are in place (105cp), open file penalty = 0
            // (shield pawns cover all 3 files), no opponent attacks yet.
            assert!(
                score >= 100,
                "Starting position king safety for {player:?} should be >= 100, got {score}"
            );
        }
    }

    #[test]
    fn test_eliminated_opponents_dont_threaten() {
        let board = Board::starting_position();
        let all_active = [PlayerStatus::Active; 4];
        let some_eliminated = [
            PlayerStatus::Active,
            PlayerStatus::Eliminated,
            PlayerStatus::Eliminated,
            PlayerStatus::Eliminated,
        ];

        let safety_all = king_safety_score(&board, Player::Red, &all_active);
        let safety_few = king_safety_score(&board, Player::Red, &some_eliminated);

        // With fewer active opponents, Red's king should be at least as safe.
        assert!(
            safety_few >= safety_all,
            "Fewer opponents should mean safer king: {safety_few} >= {safety_all}"
        );
    }

    #[test]
    fn test_shield_squares_direction() {
        // Red faces +rank: shield should be at rank+1.
        let shields = shield_squares_for_player(Player::Red, 7, 0);
        assert_eq!(shields[0], (6, 1));
        assert_eq!(shields[1], (7, 1));
        assert_eq!(shields[2], (8, 1));

        // Blue faces +file: shield should be at file+1.
        let shields = shield_squares_for_player(Player::Blue, 0, 6);
        assert_eq!(shields[0], (1, 5));
        assert_eq!(shields[1], (1, 6));
        assert_eq!(shields[2], (1, 7));

        // Yellow faces -rank: shield should be at rank-1.
        let shields = shield_squares_for_player(Player::Yellow, 6, 13);
        assert_eq!(shields[0], (5, 12));
        assert_eq!(shields[1], (6, 12));
        assert_eq!(shields[2], (7, 12));

        // Green faces -file: shield should be at file-1.
        let shields = shield_squares_for_player(Player::Green, 13, 7);
        assert_eq!(shields[0], (12, 6));
        assert_eq!(shields[1], (12, 7));
        assert_eq!(shields[2], (12, 8));
    }

    #[test]
    fn test_open_file_penalty_starting_position() {
        let board = Board::starting_position();
        let statuses = [PlayerStatus::Active; 4];
        // At start, all shield pawns are in place — open file penalty should be 0.
        // King safety = shield bonus (105) - open penalty (0) - attacker pressure (0).
        for &player in &Player::ALL {
            let score = king_safety_score(&board, player, &statuses);
            assert_eq!(
                score, 150,
                "Starting king safety for {player:?} should be exactly 150cp (3 shield pawns × 50), got {score}"
            );
        }
    }
}
