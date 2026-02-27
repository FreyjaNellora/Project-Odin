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
// Piece-Square Tables — Red's perspective
// ─────────────────────────────────────────────────
//
// Layout: rank 0 (Red's back rank) at bottom, rank 13 at top.
// Index = rank * 14 + file.
// Invalid corners: files 0-2/ranks 0-2, files 11-13/ranks 0-2,
//                  files 0-2/ranks 11-13, files 11-13/ranks 11-13.
// Those entries are 0 (never accessed in practice).
//
// Values are intentionally simple — this is a bootstrap eval replaced by NNUE.

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

// Pawn PST: reward advancement toward promotion (rank 8 for Red).
// Center files (5-8, 0-indexed) get a small bonus.
#[rustfmt::skip]
const PAWN_GRID: [[i16; 14]; 14] = [
    // rank 0: back rank (no pawns here normally)
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
    // rank 1: starting rank for Red pawns
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
    // rank 2
    [0, 0, 0,  5,  5,  8, 10, 10,  8,  5,  5, 0, 0, 0],
    // rank 3
    [0, 0, 0,  5,  8, 12, 15, 15, 12,  8,  5, 0, 0, 0],
    // rank 4
    [0, 0, 0, 10, 12, 18, 22, 22, 18, 12, 10, 0, 0, 0],
    // rank 5
    [0, 0, 0, 15, 18, 25, 30, 30, 25, 18, 15, 0, 0, 0],
    // rank 6
    [0, 0, 0, 20, 25, 32, 38, 38, 32, 25, 20, 0, 0, 0],
    // rank 7
    [0, 0, 0, 30, 32, 40, 45, 45, 40, 32, 30, 0, 0, 0],
    // rank 8: promotion rank for Red — max bonus (will promote, so rarely reached)
    [0, 0, 0, 50, 50, 50, 50, 50, 50, 50, 50, 0, 0, 0],
    // rank 9-13: beyond promotion, not relevant for Red pawns
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
];

// Knight PST: 4-player symmetric center zone. Back-rank penalty at rank 0 only.
// Peak at center 4×4 zone (+12cp). Ranks 10-13 taper toward neutral — deep
// infiltration is not penalized (knights behind enemy lines fork pieces).
// First development hop (rank 0 → rank 2) ≈ +10cp — competitive, not dominant.
#[rustfmt::skip]
const KNIGHT_GRID: [[i16; 14]; 14] = [
    // rank 0: back-rank penalty — develop
    [0, 0, 0, -8, -5, -3, -3, -3, -3, -5, -8, 0, 0, 0],
    // rank 1: transitional
    [0, 0, 0, -5, -2,  2,  3,  3,  2, -2, -5, 0, 0, 0],
    // rank 2: active — supports central pawns
    [0, 0, 0, -5,  2,  5,  8,  8,  5,  2, -5, 0, 0, 0],
    // rank 3: outpost zone begins
    [0, 0, 0, -3,  3,  8, 12, 12,  8,  3, -3, 0, 0, 0],
    // rank 4: outpost — strong pressure in all directions
    [-3,-2,-5, -3,  3,  8, 12, 12,  8,  3, -3,-5,-2,-3],
    // rank 5: approaching center — full-width zone active
    [-3, 2, 2,  3,  6,  8, 10, 10,  8,  6,  3, 2, 2,-3],
    // rank 6: center — peak zone, knight controls maximum squares
    [-3, 3, 5,  8,  8, 10, 12, 12, 10,  8,  8, 5, 3,-3],
    // rank 7: center — peak zone
    [-3, 3, 5,  8,  8, 10, 12, 12, 10,  8,  8, 5, 3,-3],
    // rank 8: mirror of rank 5
    [-3, 2, 2,  3,  6,  8, 10, 10,  8,  6,  3, 2, 2,-3],
    // rank 9: mirror of rank 4
    [-3,-2,-5, -3,  3,  8, 12, 12,  8,  3, -3,-5,-2,-3],
    // rank 10: deep infiltration — still useful outpost behind enemy lines
    [0, 0, 0, -3,  3,  8, 10, 10,  8,  3, -3, 0, 0, 0],
    // rank 11: deep — limited mobility near edge but not penalized
    [0, 0, 0,  0,  0,  3,  5,  5,  3,  0,  0, 0, 0, 0],
    // rank 12: near enemy back rank — neutral
    [0, 0, 0,  0,  0,  0,  3,  3,  0,  0,  0, 0, 0, 0],
    // rank 13: enemy back rank — neutral (limited squares but not undeveloped)
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
];

// Bishop PST: 4-player symmetric center blob for diagonal coverage.
// Back-rank penalty (rank 0) drives development — blocked by own pawns.
// Center 4×4 zone peaks at 32cp — long diagonals cover entire 14×14 board.
// Ranks 10-13 taper toward neutral — deep infiltration is not penalized.
// A bishop behind enemy lines still controls long diagonals back to center.
#[rustfmt::skip]
const BISHOP_GRID: [[i16; 14]; 14] = [
    // rank 0: back rank — blocked by pawns, strong penalty to develop
    [0, 0, 0,-10,-12,-15,-15,-15,-15,-12,-10, 0, 0, 0],
    // rank 1: first development step — fianchetto / diagonal activation
    [0, 0, 0, -3,  5, 12, 15, 15, 12,  5, -3, 0, 0, 0],
    // rank 2: developing — diagonals opening up
    [0, 0, 0, -5,  5, 18, 22, 22, 18,  5, -5, 0, 0, 0],
    // rank 3: outpost — controls long diagonal
    [0, 0, 0, -3,  8, 20, 25, 25, 20,  8, -3, 0, 0, 0],
    // rank 4: deep development — full diagonal range emerging
    [-3, 0, 0,  0, 10, 22, 28, 28, 22, 10,  0, 0, 0,-3],
    // rank 5: approaching center — wide diagonal coverage
    [-3, 5, 5,  8, 15, 25, 30, 30, 25, 15,  8, 5, 5,-3],
    // rank 6: center — peak diagonal range, bishop is long-range sniper
    [-3, 8,12, 18, 22, 28, 32, 32, 28, 22, 18,12, 8,-3],
    // rank 7: center — peak
    [-3, 8,12, 18, 22, 28, 32, 32, 28, 22, 18,12, 8,-3],
    // rank 8: mirror of rank 5
    [-3, 5, 5,  8, 15, 25, 30, 30, 25, 15,  8, 5, 5,-3],
    // rank 9: mirror of rank 4
    [-3, 0, 0,  0, 10, 22, 28, 28, 22, 10,  0, 0, 0,-3],
    // rank 10: deep infiltration — still controls long diagonals
    [0, 0, 0,  0,  8, 18, 22, 22, 18,  8,  0, 0, 0, 0],
    // rank 11: behind enemy lines — diagonal reach back to center
    [0, 0, 0,  0,  5, 12, 15, 15, 12,  5,  0, 0, 0, 0],
    // rank 12: near enemy back rank — still useful on diagonals
    [0, 0, 0,  0,  3,  8, 10, 10,  8,  3,  0, 0, 0, 0],
    // rank 13: enemy back rank — neutral (not penalized for infiltration)
    [0, 0, 0,  0,  0,  3,  5,  5,  3,  0,  0, 0, 0, 0],
];

// Rook PST: 4 royal aisles (files g, h + ranks 7, 8) valued equally.
// Center 4 squares (g7, g8, h7, h8) peak at 18cp — intersection of all aisles.
// Aisle squares taper outward symmetrically in both axes.
// No penalty for staying home — king needs back-rank defenders early.
#[rustfmt::skip]
const ROOK_GRID: [[i16; 14]; 14] = [
    // rank 0: home rank — neutral. Center files worth slightly more (open-file prep).
    [0, 0, 0,  0,  0,  3,  5,  5,  3,  0,  0, 0, 0, 0],
    // rank 1: still home territory, small center file preference
    [0, 0, 0,  0,  0,  3,  5,  5,  3,  0,  0, 0, 0, 0],
    // rank 2: beginning to activate — aisle files start showing value
    [0, 0, 0,  2,  3,  5,  8,  8,  5,  3,  2, 0, 0, 0],
    // rank 3: active rook — controls open file into enemy half
    [0, 0, 0,  3,  5,  8, 12, 12,  8,  5,  3, 0, 0, 0],
    // rank 4: fully activated, rank aisle influence emerging
    [0, 0, 0,  5,  5,  8, 12, 12,  8,  5,  5, 0, 0, 0],
    // rank 5: rank aisle opens — full-width bonus (files a-n all contribute)
    [3, 3, 5,  8,  8,  9, 15, 15,  9,  8,  8, 5, 3, 3],
    // rank 6 (rank 7): royal rank aisle — rook aims at Blue Q/Green K
    [5, 5, 8, 12, 12, 15, 18, 18, 15, 12, 12, 8, 5, 5],
    // rank 7 (rank 8): royal rank aisle — rook aims at Blue K/Green Q
    [5, 5, 8, 12, 12, 15, 18, 18, 15, 12, 12, 8, 5, 5],
    // rank 8: mirror of rank 5
    [3, 3, 5,  8,  8,  9, 15, 15,  9,  8,  8, 5, 3, 3],
    // rank 9: mirror of rank 4
    [0, 0, 0,  5,  5,  8, 12, 12,  8,  5,  5, 0, 0, 0],
    // rank 10: mirror of rank 3
    [0, 0, 0,  3,  5,  8, 12, 12,  8,  5,  3, 0, 0, 0],
    // rank 11: mirror of rank 2
    [0, 0, 0,  2,  3,  5,  8,  8,  5,  3,  2, 0, 0, 0],
    // rank 12: deep enemy territory — still useful on aisle files
    [0, 0, 0,  0,  0,  3,  5,  5,  3,  0,  0, 0, 0, 0],
    // rank 13: enemy back rank
    [0, 0, 0,  0,  0,  3,  5,  5,  3,  0,  0, 0, 0, 0],
];

// Queen PST: 4-player symmetric center blob — don't rush out early.
// Back-rank penalty (rank 0) discourages premature queen development.
// Center 4 squares peak at 8cp — queen is so mobile PST stays modest.
// Ranks 10-13 taper toward neutral — queen infiltrating enemy territory
// is aggressive and should not be penalized.
#[rustfmt::skip]
const QUEEN_GRID: [[i16; 14]; 14] = [
    // rank 0: don't move queen out immediately — gets chased by 3 opponents
    [0, 0, 0, -5, -5, -5,  0,  0, -5, -5, -5, 0, 0, 0],
    // rank 1: still early — slight penalty for edges
    [0, 0, 0, -5,  0,  0,  0,  0,  0,  0, -5, 0, 0, 0],
    // rank 2: beginning to activate
    [0, 0, 0, -5,  0,  3,  5,  5,  3,  0, -5, 0, 0, 0],
    // rank 3: moderate center preference
    [0, 0, 0,  0,  0,  5,  8,  8,  5,  0,  0, 0, 0, 0],
    // rank 4: center zone widening
    [-5, 0, 0,  0,  3,  5,  8,  8,  5,  3,  0, 0, 0,-5],
    // rank 5: full-width center influence
    [-5, 0, 0,  3,  5,  5,  8,  8,  5,  5,  3, 0, 0,-5],
    // rank 6: center — peak, queen controls maximum lines
    [ 0, 0, 3,  5,  5,  8,  8,  8,  8,  5,  5, 3, 0, 0],
    // rank 7: center — peak
    [ 0, 0, 3,  5,  5,  8,  8,  8,  8,  5,  5, 3, 0, 0],
    // rank 8: mirror of rank 5
    [-5, 0, 0,  3,  5,  5,  8,  8,  5,  5,  3, 0, 0,-5],
    // rank 9: mirror of rank 4
    [-5, 0, 0,  0,  3,  5,  8,  8,  5,  3,  0, 0, 0,-5],
    // rank 10: deep infiltration — queen raiding enemy territory
    [0, 0, 0,  0,  0,  5,  8,  8,  5,  0,  0, 0, 0, 0],
    // rank 11: behind enemy lines — still strong on all axes
    [0, 0, 0,  0,  0,  3,  5,  5,  3,  0,  0, 0, 0, 0],
    // rank 12: near enemy back rank — neutral
    [0, 0, 0,  0,  0,  0,  3,  3,  0,  0,  0, 0, 0, 0],
    // rank 13: enemy back rank — neutral (not penalized for infiltration)
    [0, 0, 0,  0,  0,  0,  0,  0,  0,  0,  0, 0, 0, 0],
];

// King PST: 4-player symmetric — stay on back rank, center is death.
// Rank 0 bonus rewards home position. Penalty escalates equally in all
// directions toward center. After rotation, every player's king wants
// to stay home and is equally deterred from approaching any opponent.
#[rustfmt::skip]
const KING_GRID: [[i16; 14]; 14] = [
    // rank 0 (back rank): strong bonus for home position, especially castled corners
    [0, 0, 0, 20, 30, 15, 10, 10, 15, 30, 20, 0, 0, 0],
    // rank 1: one step forward is dangerous in 4PC
    [0, 0, 0, -5, -5,-10,-15,-15,-10, -5, -5, 0, 0, 0],
    // rank 2: exposed — approaching center danger zone
    [0, 0, 0, -5,-10,-15,-20,-20,-15,-10, -5, 0, 0, 0],
    // rank 3: entering no-man's land
    [0, 0, 0,-15,-20,-25,-30,-30,-25,-20,-15, 0, 0, 0],
    // rank 4: center danger — attacked from all sides
    [-5,-5,-5,-20,-30,-35,-40,-40,-35,-30,-20,-5,-5,-5],
    // rank 5: center danger — full-width penalty
    [-10,-10,-15,-25,-35,-40,-45,-45,-40,-35,-25,-15,-10,-10],
    // rank 6: center — maximum exposure, 3 opponents can attack
    [-15,-15,-20,-30,-35,-40,-50,-50,-40,-35,-30,-20,-15,-15],
    // rank 7: center — maximum exposure
    [-15,-15,-20,-30,-35,-40,-50,-50,-40,-35,-30,-20,-15,-15],
    // rank 8: mirror of rank 5
    [-10,-10,-15,-25,-35,-40,-45,-45,-40,-35,-25,-15,-10,-10],
    // rank 9: mirror of rank 4
    [-5,-5,-5,-20,-30,-35,-40,-40,-35,-30,-20,-5,-5,-5],
    // rank 10: mirror of rank 3
    [0, 0, 0,-15,-20,-25,-30,-30,-25,-20,-15, 0, 0, 0],
    // rank 11: mirror of rank 2
    [0, 0, 0, -5,-10,-15,-20,-20,-15,-10, -5, 0, 0, 0],
    // rank 12: mirror of rank 1
    [0, 0, 0, -5, -5,-10,-15,-15,-10, -5, -5, 0, 0, 0],
    // rank 13: mirror of rank 0 (but no bonus — not our home)
    [0, 0, 0,  0,  0,  0, -5, -5,  0,  0,  0, 0, 0, 0],
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
        for sq in 0..TOTAL_SQUARES {
            assert_eq!(
                ROTATION[Player::Red.index()][sq],
                sq as u8,
                "Red rotation should be identity for square {sq}"
            );
        }
    }

    #[test]
    fn test_rotation_roundtrip_180() {
        // Applying Yellow rotation (180) twice returns to original.
        for sq in 0..TOTAL_SQUARES {
            let first = ROTATION[Player::Yellow.index()][sq] as usize;
            let second = ROTATION[Player::Yellow.index()][first] as usize;
            assert_eq!(
                second, sq,
                "Yellow rotation applied twice should be identity for square {sq}"
            );
        }
    }

    #[test]
    fn test_rotation_blue_green_complement() {
        // Blue (90) followed by Green (270) should be identity (270+90=360=0).
        // Actually, we need the reverse: Green's rotation table reverses Blue's.
        // Blue maps (file, rank) -> (rank, file).
        // Green maps (file, rank) -> (13-rank, 13-file).
        // So Blue followed by Green:
        //   Step 1: sq -> canonical_blue = file*14+rank
        //   Step 2: canonical_blue -> canonical_green = ...
        // These are not inverses. But Blue(Green(sq)) should equal Yellow(sq).
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
        // Red's king starts at h1 = (file=7, rank=0) = index 7.
        // Blue's king starts at a7 = (file=0, rank=6) = index 84.
        // Yellow's king starts at g14 = (file=6, rank=13) = index 188.
        // Green's king starts at n8 = (file=13, rank=7) = index 111.

        // Red's h1 -> canonical = 7 (identity)
        assert_eq!(ROTATION[Player::Red.index()][7], 7);

        // Blue's a7 (index 84) -> canonical = (rank=6, file=0) -> file*14+rank = 0*14+6 = 6
        // This is g1 in Red's frame — similar to Red's king position (h1=7).
        assert_eq!(ROTATION[Player::Blue.index()][84], 6);

        // Yellow's g14 (index 188) -> canonical = (13-6, 13-13) = (7, 0) -> 0*14+7 = 7
        // This maps to h1 — same as Red's king.
        assert_eq!(ROTATION[Player::Yellow.index()][188], 7);

        // Green's n8 (index 111) -> canonical = (13-7, 13-13) = (6, 0) -> 0*14+6 = 6
        // This maps to g1 — same as Blue's king (Blue/Green have K/Q swapped vs Red/Yellow).
        assert_eq!(ROTATION[Player::Green.index()][111], 6);
    }

    #[test]
    fn test_pst_knight_center_bonus() {
        // Center square (file=7, rank=7) should have high knight value.
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
        // Pawn further advanced should have higher PST value (from Red's perspective).
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
        // King on back rank should have higher value than in center.
        let back = square_from(7, 0).unwrap(); // h1, Red's back rank
        let center = square_from(7, 7).unwrap(); // h8, board center
        let back_val = pst_value(PieceType::King, back, Player::Red);
        let center_val = pst_value(PieceType::King, center, Player::Red);
        assert!(
            back_val > center_val,
            "King on back rank ({back_val}) should be > center ({center_val})"
        );
    }

    #[test]
    fn test_pst_symmetry_red_yellow() {
        // Red's king at h1 should get the same PST value as Yellow's king at g14
        // after rotation (both map to the canonical "king on back rank" position).
        let red_king_sq = square_from(7, 0).unwrap(); // h1
        let yellow_king_sq = square_from(6, 13).unwrap(); // g14

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
        // Red and Yellow should have identical positional scores (same rotational position).
        let red_pos = positional_score(&board, Player::Red);
        let yellow_pos = positional_score(&board, Player::Yellow);
        assert_eq!(
            red_pos, yellow_pos,
            "Red ({red_pos}) and Yellow ({yellow_pos}) should have equal positional scores at start"
        );

        // Blue and Green should have identical positional scores.
        let blue_pos = positional_score(&board, Player::Blue);
        let green_pos = positional_score(&board, Player::Green);
        assert_eq!(
            blue_pos, green_pos,
            "Blue ({blue_pos}) and Green ({green_pos}) should have equal positional scores at start"
        );
    }

    #[test]
    fn test_pst_values_bounded() {
        // No PST value should be extreme enough to dominate material.
        for pt_idx in 0..PIECE_TYPE_COUNT {
            for sq in 0..TOTAL_SQUARES {
                let val = RED_PST[pt_idx][sq];
                assert!(
                    val.abs() <= 50,
                    "PST[{pt_idx}][{sq}] = {val} exceeds ±50cp bound"
                );
            }
        }
    }
}
