// Piece-square tables for positional evaluation.
//
// Tables are defined from Red's perspective (south, facing north/+rank).
// Other players' squares are rotated to Red's frame before lookup.
//
// Rotation scheme:
//   Red:    identity              (file, rank)
//   Blue:   90 CW (faces east)   canonical = (rank, file)
//   Yellow: 180   (faces south)  canonical = (13-file, 13-rank)
//   Green:  270 CW (faces west)  canonical = (13-rank, 13-file)
//
// Each table is 196 entries (14x14 flat). Invalid corner entries are 0.
//
// DESIGN PHILOSOPHY (v0.5.1):
// PSTs are tiebreakers, not decision-makers. Max ±10cp for pieces, ±25cp for king.
// The eval pipeline has stronger signals (material 100-900cp, mobility 10-60cp,
// king safety 10-50cp). PSTs whisper gentle preferences — they should never
// override a mobility or safety signal. In 4-player chess, advanced pawns are
// targets for 3 opponents, not assets approaching promotion.

use crate::board::{
    Board, PieceStatus, PieceType, Player, Square, PIECE_TYPE_COUNT, TOTAL_SQUARES,
};

/// Pre-computed rotation tables: ROTATION[player_index][square] -> canonical square.
/// Computed at compile time.
const ROTATION: [[u8; TOTAL_SQUARES]; 4] = build_rotation_tables();

/// Build rotation tables at compile time.
const fn build_rotation_tables() -> [[u8; TOTAL_SQUARES]; 4] {
    let mut tables = [[0u8; TOTAL_SQUARES]; 4];
    let mut sq: usize = 0;
    while sq < TOTAL_SQUARES {
        let file = (sq % 14) as u8;
        let rank = (sq / 14) as u8;

        // Red: identity
        tables[0][sq] = rank * 14 + file;

        // Blue: canonical = (rank, file) -> index = file * 14 + rank
        tables[1][sq] = file * 14 + rank;

        // Yellow: canonical = (13 - file, 13 - rank) -> index = (13-rank)*14 + (13-file)
        tables[2][sq] = (13 - rank) * 14 + (13 - file);

        // Green: canonical = (13 - rank, 13 - file) -> index = (13-file)*14 + (13-rank)
        tables[3][sq] = (13 - file) * 14 + (13 - rank);

        sq += 1;
    }
    tables
}

/// Look up PST value for a piece of the given type on the given square,
/// from the given player's perspective.
fn pst_value(piece_type: PieceType, sq: Square, player: Player) -> i16 {
    let canonical_sq = ROTATION[player.index()][sq as usize] as usize;
    RED_PST[piece_type.index()][canonical_sq]
}

/// Total positional score for a player (sum of PST values for all their alive pieces).
pub(crate) fn positional_score(board: &Board, player: Player) -> i16 {
    let mut score: i16 = 0;
    for &(pt, sq) in board.piece_list(player) {
        if let Some(piece) = board.piece_at(sq) {
            if piece.status == PieceStatus::Alive {
                score = score.saturating_add(pst_value(pt, sq, player));
            }
        }
    }
    score
}

// ─────────────────────────────────────────────────
// Piece-Square Tables — Red's perspective (v0.5.1 rework)
// ─────────────────────────────────────────────────
//
// Layout: rank 0 (Red's back rank) at bottom, rank 13 at top.
// Index = rank * 14 + file.
// Invalid corners: files 0-2/ranks 0-2, files 11-13/ranks 0-2,
//                  files 0-2/ranks 11-13, files 11-13/ranks 11-13.
// Those entries are 0 (never accessed in practice).
//
// All values are intentionally small — PSTs are gentle guides, not demands.
// Mobility, material, and king safety do the heavy lifting.

/// Helper to build a 196-entry PST from a 14x14 grid expressed as [rank][file].
/// rank 0 = Red's back rank (bottom), rank 13 = opposite side (top).
const fn flatten_pst(grid: [[i16; 14]; 14]) -> [i16; TOTAL_SQUARES] {
    let mut table = [0i16; TOTAL_SQUARES];
    let mut rank = 0usize;
    while rank < 14 {
        let mut file = 0usize;
        while file < 14 {
            table[rank * 14 + file] = grid[rank][file];
            file += 1;
        }
        rank += 1;
    }
    table
}

// Pawn PST: nearly flat advancement curve, max +8cp.
// In 4PC, advanced pawns are exposed to 3 opponents — pushing is risky, not free.
// Center files get +1-2cp bonus (control more useful squares).
// This is a tiebreaker, not a motivation to advance.
#[rustfmt::skip]
const PAWN_GRID: [[i16; 14]; 14] = [
    // rank 0: back rank (no pawns here normally)
    [0, 0, 0,  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    // rank 1: starting rank for Red pawns
    [0, 0, 0,  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    // rank 2: first push — tiny nudge
    [0, 0, 0,  1, 1, 2, 3, 3, 2, 1, 1, 0, 0, 0],
    // rank 3
    [0, 0, 0,  2, 2, 3, 4, 4, 3, 2, 2, 0, 0, 0],
    // rank 4: midboard
    [0, 0, 0,  2, 3, 4, 5, 5, 4, 3, 2, 0, 0, 0],
    // rank 5: center — slight plateau
    [0, 0, 0,  3, 3, 5, 6, 6, 5, 3, 3, 0, 0, 0],
    // rank 6: no increase — pushing further is risky without support
    [0, 0, 0,  3, 4, 5, 6, 6, 5, 4, 3, 0, 0, 0],
    // rank 7: near promotion
    [0, 0, 0,  4, 4, 5, 7, 7, 5, 4, 4, 0, 0, 0],
    // rank 8: promotion rank — slight bonus if you get here
    [0, 0, 0,  5, 5, 6, 8, 8, 6, 5, 5, 0, 0, 0],
    // rank 9-13: beyond promotion, not relevant for Red pawns
    [0, 0, 0,  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0,  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0,  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0,  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0,  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];

// Knight PST: gentle development nudge. Back rank = -3, center = +4.
// First develop hop (rank 0 → rank 2) ≈ +5cp — a tiebreaker, not a demand.
// Mobility already strongly rewards developed knights (2 moves → 8 moves).
#[rustfmt::skip]
const KNIGHT_GRID: [[i16; 14]; 14] = [
    // rank 0: back-rank nudge — develop me
    [0, 0, 0, -3, -2, -1, -1, -1, -1, -2, -3, 0, 0, 0],
    // rank 1: transitional
    [0, 0, 0, -2, -1,  1,  1,  1,  1, -1, -2, 0, 0, 0],
    // rank 2: active
    [0, 0, 0, -2,  1,  2,  3,  3,  2,  1, -2, 0, 0, 0],
    // rank 3: outpost zone
    [0, 0, 0, -1,  1,  3,  4,  4,  3,  1, -1, 0, 0, 0],
    // rank 4: outpost
    [-1,-1,-2, -1,  1,  3,  4,  4,  3,  1, -1,-2,-1,-1],
    // rank 5: approaching center
    [-1, 1, 1,  1,  2,  3,  3,  3,  3,  2,  1, 1, 1,-1],
    // rank 6: center — peak
    [-1, 1, 2,  3,  3,  3,  4,  4,  3,  3,  3, 2, 1,-1],
    // rank 7: center — peak
    [-1, 1, 2,  3,  3,  3,  4,  4,  3,  3,  3, 2, 1,-1],
    // rank 8: mirror of rank 5
    [-1, 1, 1,  1,  2,  3,  3,  3,  3,  2,  1, 1, 1,-1],
    // rank 9: mirror of rank 4
    [-1,-1,-2, -1,  1,  3,  4,  4,  3,  1, -1,-2,-1,-1],
    // rank 10: deep infiltration — still useful
    [0, 0, 0, -1,  1,  3,  3,  3,  3,  1, -1, 0, 0, 0],
    // rank 11: deep
    [0, 0, 0,  0,  0,  1,  2,  2,  1,  0,  0, 0, 0, 0],
    // rank 12: near enemy back rank — neutral
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    // rank 13: enemy back rank — neutral
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
];

// Bishop PST: gentle development nudge. Back rank = -4, center = +8.
// Mobility already massively rewards developed bishops (0 moves blocked → 10+ open).
// PST just whispers "don't stay home."
#[rustfmt::skip]
const BISHOP_GRID: [[i16; 14]; 14] = [
    // rank 0: back rank — blocked by pawns, gentle nudge to develop
    [0, 0, 0, -3, -4, -4, -4, -4, -4, -4, -3, 0, 0, 0],
    // rank 1: first development step
    [0, 0, 0, -1,  2,  3,  4,  4,  3,  2, -1, 0, 0, 0],
    // rank 2: developing
    [0, 0, 0, -1,  2,  5,  6,  6,  5,  2, -1, 0, 0, 0],
    // rank 3: outpost — controls long diagonal
    [0, 0, 0, -1,  2,  5,  6,  6,  5,  2, -1, 0, 0, 0],
    // rank 4: deep development
    [-1, 0, 0,  0,  3,  6,  7,  7,  6,  3,  0, 0, 0,-1],
    // rank 5: approaching center
    [-1, 1, 1,  2,  4,  6,  8,  8,  6,  4,  2, 1, 1,-1],
    // rank 6: center — peak diagonal range
    [-1, 2, 3,  5,  6,  7,  8,  8,  7,  6,  5, 3, 2,-1],
    // rank 7: center — peak
    [-1, 2, 3,  5,  6,  7,  8,  8,  7,  6,  5, 3, 2,-1],
    // rank 8: mirror of rank 5
    [-1, 1, 1,  2,  4,  6,  8,  8,  6,  4,  2, 1, 1,-1],
    // rank 9: mirror of rank 4
    [-1, 0, 0,  0,  3,  6,  7,  7,  6,  3,  0, 0, 0,-1],
    // rank 10: deep infiltration — still controls diagonals
    [0, 0, 0,  0,  2,  5,  6,  6,  5,  2,  0, 0, 0, 0],
    // rank 11: behind enemy lines
    [0, 0, 0,  0,  1,  3,  4,  4,  3,  1,  0, 0, 0, 0],
    // rank 12: near enemy back rank
    [0, 0, 0,  0,  1,  2,  3,  3,  2,  1,  0, 0, 0, 0],
    // rank 13: enemy back rank — neutral
    [0, 0, 0,  0,  0,  1,  1,  1,  1,  0,  0, 0, 0, 0],
];

// Rook PST: very flat. Rook value comes from open files (dynamic, not static).
// Royal aisle intersection (center) gets a small +6cp. Home rank is neutral.
#[rustfmt::skip]
const ROOK_GRID: [[i16; 14]; 14] = [
    // rank 0: home rank — neutral
    [0, 0, 0,  0,  0,  1,  2,  2,  1,  0,  0, 0, 0, 0],
    // rank 1: still home territory
    [0, 0, 0,  0,  0,  1,  2,  2,  1,  0,  0, 0, 0, 0],
    // rank 2: beginning to activate
    [0, 0, 0,  1,  1,  2,  3,  3,  2,  1,  1, 0, 0, 0],
    // rank 3: active rook
    [0, 0, 0,  1,  2,  3,  4,  4,  3,  2,  1, 0, 0, 0],
    // rank 4: fully activated
    [0, 0, 0,  2,  2,  3,  4,  4,  3,  2,  2, 0, 0, 0],
    // rank 5: rank aisle — full width
    [1, 1, 2,  3,  3,  3,  5,  5,  3,  3,  3, 2, 1, 1],
    // rank 6: royal rank aisle
    [2, 2, 3,  4,  4,  5,  6,  6,  5,  4,  4, 3, 2, 2],
    // rank 7: royal rank aisle
    [2, 2, 3,  4,  4,  5,  6,  6,  5,  4,  4, 3, 2, 2],
    // rank 8: mirror of rank 5
    [1, 1, 2,  3,  3,  3,  5,  5,  3,  3,  3, 2, 1, 1],
    // rank 9: mirror of rank 4
    [0, 0, 0,  2,  2,  3,  4,  4,  3,  2,  2, 0, 0, 0],
    // rank 10: mirror of rank 3
    [0, 0, 0,  1,  2,  3,  4,  4,  3,  2,  1, 0, 0, 0],
    // rank 11: mirror of rank 2
    [0, 0, 0,  1,  1,  2,  3,  3,  2,  1,  1, 0, 0, 0],
    // rank 12: deep enemy territory
    [0, 0, 0,  0,  0,  1,  2,  2,  1,  0,  0, 0, 0, 0],
    // rank 13: enemy back rank
    [0, 0, 0,  0,  0,  1,  2,  2,  1,  0,  0, 0, 0, 0],
];

// Queen PST: nearly flat, max ±3cp. Queen is so mobile PST is almost irrelevant.
// Gentle nudge to not rush out (back rank -2) and mild center preference (+3).
#[rustfmt::skip]
const QUEEN_GRID: [[i16; 14]; 14] = [
    // rank 0: don't rush out — gets chased by 3 opponents
    [0, 0, 0, -2, -2, -2,  0,  0, -2, -2, -2, 0, 0, 0],
    // rank 1: transitional
    [0, 0, 0, -2,  0,  0,  0,  0,  0,  0, -2, 0, 0, 0],
    // rank 2: beginning to activate
    [0, 0, 0, -1,  0,  1,  1,  1,  1,  0, -1, 0, 0, 0],
    // rank 3
    [0, 0, 0,  0,  0,  1,  2,  2,  1,  0,  0, 0, 0, 0],
    // rank 4
    [-1, 0, 0,  0,  1,  2,  3,  3,  2,  1,  0, 0, 0,-1],
    // rank 5
    [-1, 0, 0,  1,  1,  2,  3,  3,  2,  1,  1, 0, 0,-1],
    // rank 6: center — peak
    [ 0, 0, 1,  1,  2,  2,  3,  3,  2,  2,  1, 1, 0, 0],
    // rank 7: center — peak
    [ 0, 0, 1,  1,  2,  2,  3,  3,  2,  2,  1, 1, 0, 0],
    // rank 8: mirror of rank 5
    [-1, 0, 0,  1,  1,  2,  3,  3,  2,  1,  1, 0, 0,-1],
    // rank 9: mirror of rank 4
    [-1, 0, 0,  0,  1,  2,  3,  3,  2,  1,  0, 0, 0,-1],
    // rank 10: deep infiltration
    [0, 0, 0,  0,  0,  1,  2,  2,  1,  0,  0, 0, 0, 0],
    // rank 11: behind enemy lines
    [0, 0, 0,  0,  0,  1,  1,  1,  1,  0,  0, 0, 0, 0],
    // rank 12: near enemy back rank
    [0, 0, 0,  0,  0,  0,  1,  1,  0,  0,  0, 0, 0, 0],
    // rank 13: enemy back rank — neutral
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
];

// King PST: stay home. Center is death. Halved from original values.
// King safety eval component handles the detailed danger assessment.
// PST provides baseline preference: back rank good, center bad.
#[rustfmt::skip]
const KING_GRID: [[i16; 14]; 14] = [
    // rank 0 (back rank): home — safe
    [0, 0, 0, 10, 15,  8,  5,  5,  8, 15, 10, 0, 0, 0],
    // rank 1: one step forward is risky in 4PC
    [0, 0, 0, -3, -3, -5, -8, -8, -5, -3, -3, 0, 0, 0],
    // rank 2: exposed
    [0, 0, 0, -3, -5, -8,-10,-10, -8, -5, -3, 0, 0, 0],
    // rank 3: entering danger zone
    [0, 0, 0, -8,-10,-13,-15,-15,-13,-10, -8, 0, 0, 0],
    // rank 4: center danger
    [-3,-3,-3,-10,-15,-18,-20,-20,-18,-15,-10,-3,-3,-3],
    // rank 5: center danger — full width
    [-5,-5,-8,-13,-18,-20,-23,-23,-20,-18,-13,-8,-5,-5],
    // rank 6: center — maximum exposure
    [-8,-8,-10,-15,-18,-20,-25,-25,-20,-18,-15,-10,-8,-8],
    // rank 7: center — maximum exposure
    [-8,-8,-10,-15,-18,-20,-25,-25,-20,-18,-15,-10,-8,-8],
    // rank 8: mirror of rank 5
    [-5,-5,-8,-13,-18,-20,-23,-23,-20,-18,-13,-8,-5,-5],
    // rank 9: mirror of rank 4
    [-3,-3,-3,-10,-15,-18,-20,-20,-18,-15,-10,-3,-3,-3],
    // rank 10: mirror of rank 3
    [0, 0, 0, -8,-10,-13,-15,-15,-13,-10, -8, 0, 0, 0],
    // rank 11: mirror of rank 2
    [0, 0, 0, -3, -5, -8,-10,-10, -8, -5, -3, 0, 0, 0],
    // rank 12: mirror of rank 1
    [0, 0, 0, -3, -3, -5, -8, -8, -5, -3, -3, 0, 0, 0],
    // rank 13: mirror of rank 0 (not our home — no bonus)
    [0, 0, 0,  0,  0,  0, -3, -3,  0,  0,  0, 0, 0, 0],
];

/// PST values for Red's perspective, indexed by PieceType::index() then by square.
/// PromotedQueen uses the same table as Queen.
const RED_PST: [[i16; TOTAL_SQUARES]; PIECE_TYPE_COUNT] = [
    flatten_pst(PAWN_GRID),   // 0: Pawn
    flatten_pst(KNIGHT_GRID), // 1: Knight
    flatten_pst(BISHOP_GRID), // 2: Bishop
    flatten_pst(ROOK_GRID),   // 3: Rook
    flatten_pst(QUEEN_GRID),  // 4: Queen
    flatten_pst(KING_GRID),   // 5: King
    flatten_pst(QUEEN_GRID),  // 6: PromotedQueen (same as Queen)
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::square_from;

    #[test]
    fn test_rotation_red_identity() {
        for (sq, &rot) in ROTATION[Player::Red.index()].iter().enumerate() {
            assert_eq!(
                rot,
                sq as u8,
                "Red rotation should be identity for square {sq}"
            );
        }
    }

    #[test]
    fn test_rotation_roundtrip_180() {
        // Applying Yellow rotation (180) twice returns to original.
        for (sq, &rot) in ROTATION[Player::Yellow.index()].iter().enumerate() {
            let first = rot as usize;
            let second = ROTATION[Player::Yellow.index()][first] as usize;
            assert_eq!(
                second, sq,
                "Yellow rotation applied twice should be identity for square {sq}"
            );
        }
    }

    #[test]
    fn test_rotation_blue_green_complement() {
        // Blue (90) followed by Green (270) should yield Yellow (180).
        for sq in 0..TOTAL_SQUARES {
            let blue_of_green =
                ROTATION[Player::Blue.index()][ROTATION[Player::Green.index()][sq] as usize];
            let yellow = ROTATION[Player::Yellow.index()][sq];
            assert_eq!(
                blue_of_green, yellow,
                "Blue(Green(sq)) should equal Yellow(sq) for square {sq}"
            );
        }
    }

    #[test]
    fn test_rotation_known_squares() {
        // Red's h1 -> canonical = 7 (identity)
        assert_eq!(ROTATION[Player::Red.index()][7], 7);
        // Blue's a7 (index 84) -> canonical = 6
        assert_eq!(ROTATION[Player::Blue.index()][84], 6);
        // Yellow's g14 (index 188) -> canonical = 7
        assert_eq!(ROTATION[Player::Yellow.index()][188], 7);
        // Green's n8 (index 111) -> canonical = 6
        assert_eq!(ROTATION[Player::Green.index()][111], 6);
    }

    #[test]
    fn test_pst_knight_center_bonus() {
        let center = square_from(7, 7).unwrap();
        let edge = square_from(3, 0).unwrap();
        let center_val = pst_value(PieceType::Knight, center, Player::Red);
        let edge_val = pst_value(PieceType::Knight, edge, Player::Red);
        assert!(
            center_val > edge_val,
            "Knight center ({center_val}) should be > edge ({edge_val})"
        );
    }

    #[test]
    fn test_pst_pawn_advancement_bonus() {
        let rank2 = square_from(6, 2).unwrap();
        let rank5 = square_from(6, 5).unwrap();
        let val2 = pst_value(PieceType::Pawn, rank2, Player::Red);
        let val5 = pst_value(PieceType::Pawn, rank5, Player::Red);
        assert!(
            val5 > val2,
            "Advanced pawn ({val5}) should be > less advanced ({val2})"
        );
    }

    #[test]
    fn test_pst_king_back_rank_preferred() {
        let back = square_from(7, 0).unwrap();
        let center = square_from(7, 7).unwrap();
        let back_val = pst_value(PieceType::King, back, Player::Red);
        let center_val = pst_value(PieceType::King, center, Player::Red);
        assert!(
            back_val > center_val,
            "King on back rank ({back_val}) should be > center ({center_val})"
        );
    }

    #[test]
    fn test_pst_symmetry_red_yellow() {
        let red_king_sq = square_from(7, 0).unwrap();
        let yellow_king_sq = square_from(6, 13).unwrap();
        let red_val = pst_value(PieceType::King, red_king_sq, Player::Red);
        let yellow_val = pst_value(PieceType::King, yellow_king_sq, Player::Yellow);
        assert_eq!(
            red_val, yellow_val,
            "Red h1 PST ({red_val}) should equal Yellow g14 PST ({yellow_val})"
        );
    }

    #[test]
    fn test_positional_score_starting_position_symmetric() {
        let board = Board::starting_position();
        let red_pos = positional_score(&board, Player::Red);
        let yellow_pos = positional_score(&board, Player::Yellow);
        assert_eq!(
            red_pos, yellow_pos,
            "Red ({red_pos}) and Yellow ({yellow_pos}) should have equal positional scores at start"
        );
        let blue_pos = positional_score(&board, Player::Blue);
        let green_pos = positional_score(&board, Player::Green);
        assert_eq!(
            blue_pos, green_pos,
            "Blue ({blue_pos}) and Green ({green_pos}) should have equal positional scores at start"
        );
    }

    #[test]
    fn test_pst_values_bounded() {
        // PSTs are now gentle: max ±10cp for pieces, ±25cp for king.
        for (pt_idx, pst_table) in RED_PST.iter().enumerate() {
            let bound = if pt_idx == PieceType::King.index() { 25 } else { 10 };
            for (sq, &val) in pst_table.iter().enumerate() {
                assert!(
                    val.abs() <= bound,
                    "PST[{pt_idx}][{sq}] = {val} exceeds ±{bound}cp bound"
                );
            }
        }
    }
}
