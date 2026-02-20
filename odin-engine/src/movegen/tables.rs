// Pre-computed attack tables for the 14x14 board with 36 invalid corners.
//
// For each valid square, store:
//   - Ray tables for sliding pieces (8 directions, list of squares until edge/corner)
//   - Knight destination tables
//   - King adjacency tables
//   - Pawn attack tables (per player)
//
// All tables respect board boundaries and invalid corner squares.

use crate::board::{file_of, is_valid_square, rank_of, square_from, Square, BOARD_SIZE, TOTAL_SQUARES};

/// Direction deltas as (file_delta, rank_delta).
/// Order: N, NE, E, SE, S, SW, W, NW.
const DIRECTION_DELTAS: [(i8, i8); 8] = [
    (0, 1),   // North (+rank)
    (1, 1),   // Northeast
    (1, 0),   // East (+file)
    (1, -1),  // Southeast
    (0, -1),  // South (-rank)
    (-1, -1), // Southwest
    (-1, 0),  // West (-file)
    (-1, 1),  // Northwest
];

/// Number of ray directions for sliding pieces.
pub const NUM_DIRECTIONS: usize = 8;

/// Knight move deltas (file, rank).
const KNIGHT_DELTAS: [(i8, i8); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

/// Pre-computed attack tables for all 196 squares.
pub struct AttackTables {
    /// Ray tables: for each square and direction, a list of squares along that ray.
    /// rays[sq][dir] = Vec of squares from sq outward in direction dir, stopping at
    /// board edge or invalid corner.
    rays: Vec<[Vec<Square>; NUM_DIRECTIONS]>,

    /// Knight destinations: for each square, valid destination squares.
    knight_moves: Vec<Vec<Square>>,

    /// King destinations: for each square, valid adjacent squares.
    king_moves: Vec<Vec<Square>>,

    /// Pawn attack squares: for each player and square, the capture target squares.
    /// pawn_attacks[player_idx][sq] = Vec of squares this pawn can attack (diagonal captures).
    pawn_attacks: [Vec<Vec<Square>>; 4],
}

impl AttackTables {
    /// Generate all attack tables.
    pub fn new() -> Self {
        let mut rays = Vec::with_capacity(TOTAL_SQUARES);
        let mut knight_moves = Vec::with_capacity(TOTAL_SQUARES);
        let mut king_moves = Vec::with_capacity(TOTAL_SQUARES);

        for sq in 0..TOTAL_SQUARES as u8 {
            if is_valid_square(sq) {
                rays.push(Self::compute_rays(sq));
                knight_moves.push(Self::compute_knight_moves(sq));
                king_moves.push(Self::compute_king_moves(sq));
            } else {
                // Placeholder for invalid squares — never accessed
                rays.push(Default::default());
                knight_moves.push(Vec::new());
                king_moves.push(Vec::new());
            }
        }

        let pawn_attacks = [
            Self::compute_pawn_attacks(0), // Red: +rank, captures (+/-1 file, +1 rank)
            Self::compute_pawn_attacks(1), // Blue: +file, captures (+1 file, +/-1 rank)
            Self::compute_pawn_attacks(2), // Yellow: -rank, captures (+/-1 file, -1 rank)
            Self::compute_pawn_attacks(3), // Green: -file, captures (-1 file, +/-1 rank)
        ];

        Self {
            rays,
            knight_moves,
            king_moves,
            pawn_attacks,
        }
    }

    /// Get the ray from a square in a given direction.
    /// Returns a slice of squares along the ray (not including the origin).
    #[inline]
    pub fn ray(&self, sq: Square, direction: usize) -> &[Square] {
        &self.rays[sq as usize][direction]
    }

    /// Get all knight destination squares from a square.
    #[inline]
    pub fn knight_destinations(&self, sq: Square) -> &[Square] {
        &self.knight_moves[sq as usize]
    }

    /// Get all king destination squares from a square.
    #[inline]
    pub fn king_destinations(&self, sq: Square) -> &[Square] {
        &self.king_moves[sq as usize]
    }

    /// Get pawn attack squares for a given player from a given square.
    #[inline]
    pub fn pawn_attack_squares(&self, player_idx: usize, sq: Square) -> &[Square] {
        &self.pawn_attacks[player_idx][sq as usize]
    }

    fn compute_rays(sq: Square) -> [Vec<Square>; NUM_DIRECTIONS] {
        let file = file_of(sq) as i8;
        let rank = rank_of(sq) as i8;

        let mut result: [Vec<Square>; NUM_DIRECTIONS] = Default::default();

        for (dir_idx, &(df, dr)) in DIRECTION_DELTAS.iter().enumerate() {
            let mut ray = Vec::new();
            let mut f = file + df;
            let mut r = rank + dr;

            while f >= 0 && f < BOARD_SIZE as i8 && r >= 0 && r < BOARD_SIZE as i8 {
                let target = square_from(f as u8, r as u8).unwrap();
                if !is_valid_square(target) {
                    break; // Stop ray at invalid corner squares
                }
                ray.push(target);
                f += df;
                r += dr;
            }

            result[dir_idx] = ray;
        }

        result
    }

    fn compute_knight_moves(sq: Square) -> Vec<Square> {
        let file = file_of(sq) as i8;
        let rank = rank_of(sq) as i8;

        let mut moves = Vec::new();
        for &(df, dr) in &KNIGHT_DELTAS {
            let f = file + df;
            let r = rank + dr;
            if f >= 0 && f < BOARD_SIZE as i8 && r >= 0 && r < BOARD_SIZE as i8 {
                let target = square_from(f as u8, r as u8).unwrap();
                if is_valid_square(target) {
                    moves.push(target);
                }
            }
        }
        moves
    }

    fn compute_king_moves(sq: Square) -> Vec<Square> {
        let file = file_of(sq) as i8;
        let rank = rank_of(sq) as i8;

        let mut moves = Vec::new();
        for &(df, dr) in &DIRECTION_DELTAS {
            let f = file + df;
            let r = rank + dr;
            if f >= 0 && f < BOARD_SIZE as i8 && r >= 0 && r < BOARD_SIZE as i8 {
                let target = square_from(f as u8, r as u8).unwrap();
                if is_valid_square(target) {
                    moves.push(target);
                }
            }
        }
        moves
    }

    /// Compute pawn attack tables for a given player.
    /// Returns a Vec indexed by square, each containing the attack target squares.
    fn compute_pawn_attacks(player_idx: usize) -> Vec<Vec<Square>> {
        // Pawn capture deltas per player:
        // Red (+rank): captures at (+1 file, +1 rank) and (-1 file, +1 rank)
        // Blue (+file): captures at (+1 file, +1 rank) and (+1 file, -1 rank)
        // Yellow (-rank): captures at (+1 file, -1 rank) and (-1 file, -1 rank)
        // Green (-file): captures at (-1 file, +1 rank) and (-1 file, -1 rank)
        let capture_deltas: [(i8, i8); 2] = match player_idx {
            0 => [(-1, 1), (1, 1)],   // Red
            1 => [(1, -1), (1, 1)],    // Blue
            2 => [(-1, -1), (1, -1)],  // Yellow
            3 => [(-1, -1), (-1, 1)],  // Green
            _ => unreachable!(),
        };

        let mut attacks = Vec::with_capacity(TOTAL_SQUARES);
        for sq in 0..TOTAL_SQUARES as u8 {
            if is_valid_square(sq) {
                let file = file_of(sq) as i8;
                let rank = rank_of(sq) as i8;
                let mut sq_attacks = Vec::new();
                for &(df, dr) in &capture_deltas {
                    let f = file + df;
                    let r = rank + dr;
                    if f >= 0 && f < BOARD_SIZE as i8 && r >= 0 && r < BOARD_SIZE as i8 {
                        let target = square_from(f as u8, r as u8).unwrap();
                        if is_valid_square(target) {
                            sq_attacks.push(target);
                        }
                    }
                }
                attacks.push(sq_attacks);
            } else {
                attacks.push(Vec::new());
            }
        }
        attacks
    }
}

/// Direction indices for quick reference.
pub const DIR_NORTH: usize = 0;
pub const DIR_NORTHEAST: usize = 1;
pub const DIR_EAST: usize = 2;
pub const DIR_SOUTHEAST: usize = 3;
pub const DIR_SOUTH: usize = 4;
pub const DIR_SOUTHWEST: usize = 5;
pub const DIR_WEST: usize = 6;
pub const DIR_NORTHWEST: usize = 7;

/// Whether a direction is diagonal.
#[inline]
pub fn is_diagonal(dir: usize) -> bool {
    dir % 2 == 1
}

/// Whether a direction is orthogonal (straight).
#[inline]
pub fn is_orthogonal(dir: usize) -> bool {
    dir % 2 == 0
}

/// Lazily-initialized global attack tables.
pub fn global_attack_tables() -> &'static AttackTables {
    use std::sync::OnceLock;
    static TABLES: OnceLock<AttackTables> = OnceLock::new();
    TABLES.get_or_init(AttackTables::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_square_has_8_king_moves() {
        let tables = AttackTables::new();
        // e5 (file 4, rank 4) — center, all 8 neighbors valid
        let sq = square_from(4, 4).unwrap();
        assert_eq!(tables.king_destinations(sq).len(), 8);
    }

    #[test]
    fn test_corner_adjacent_has_fewer_king_moves() {
        let tables = AttackTables::new();
        // d1 (file 3, rank 0) — on south edge, adjacent to bottom-left corner
        // Neighbors: c1 (invalid!), d2 (valid), e1 (valid), e2 (valid), c2 (invalid!)
        let sq = square_from(3, 0).unwrap();
        let moves = tables.king_destinations(sq);
        assert!(moves.len() < 8, "should have fewer than 8 moves near corner");
        // Specifically: d2, e1, e2 = 3 valid neighbors
        assert_eq!(moves.len(), 3);
    }

    #[test]
    fn test_knight_moves_center() {
        let tables = AttackTables::new();
        // g7 (file 6, rank 6) — center, all 8 knight moves valid
        let sq = square_from(6, 6).unwrap();
        assert_eq!(tables.knight_destinations(sq).len(), 8);
    }

    #[test]
    fn test_knight_moves_near_corner() {
        let tables = AttackTables::new();
        // d1 (file 3, rank 0) — some knight moves land on corners
        let sq = square_from(3, 0).unwrap();
        let moves = tables.knight_destinations(sq);
        // From d1: possible moves (f,r): (5,1), (4,2), (2,1)invalid, (1,1)invalid, (5,-1)OOB, (4,-2)OOB, (2,-1)OOB, (1,-1)OOB
        // Valid: (5,1)=f2, (4,2)=e3
        for &m in moves {
            assert!(is_valid_square(m), "knight destination must be valid");
        }
    }

    #[test]
    fn test_ray_stops_at_corner() {
        let tables = AttackTables::new();
        // d4 (file 3, rank 3) — southwest ray should stop before hitting corner
        let sq = square_from(3, 3).unwrap();
        let sw_ray = tables.ray(sq, DIR_SOUTHWEST);
        // SW from d4: c3 (file 2, rank 2) is INVALID (corner)
        assert!(
            sw_ray.is_empty(),
            "SW ray from d4 should stop at corner boundary"
        );
    }

    #[test]
    fn test_ray_along_south_edge() {
        let tables = AttackTables::new();
        // d1 (file 3, rank 0) — east ray along rank 0
        let sq = square_from(3, 0).unwrap();
        let east_ray = tables.ray(sq, DIR_EAST);
        // East from d1: e1, f1, g1, h1, i1, j1, k1 (7 squares, stops before l1 which is corner)
        assert_eq!(east_ray.len(), 7, "east ray from d1 should have 7 squares");
        assert_eq!(east_ray[0], square_from(4, 0).unwrap()); // e1
        assert_eq!(east_ray[6], square_from(10, 0).unwrap()); // k1
    }

    #[test]
    fn test_pawn_attacks_red() {
        let tables = AttackTables::new();
        // Red pawn on e4 (file 4, rank 3): attacks d5 and f5
        let sq = square_from(4, 3).unwrap();
        let attacks = tables.pawn_attack_squares(0, sq);
        assert_eq!(attacks.len(), 2);
        assert!(attacks.contains(&square_from(3, 4).unwrap())); // d5
        assert!(attacks.contains(&square_from(5, 4).unwrap())); // f5
    }

    #[test]
    fn test_pawn_attacks_blue() {
        let tables = AttackTables::new();
        // Blue pawn on b5 (file 1, rank 4): attacks c4 and c6
        let sq = square_from(1, 4).unwrap();
        let attacks = tables.pawn_attack_squares(1, sq);
        assert_eq!(attacks.len(), 2);
        assert!(attacks.contains(&square_from(2, 3).unwrap())); // c4
        assert!(attacks.contains(&square_from(2, 5).unwrap())); // c6
    }

    #[test]
    fn test_pawn_attacks_yellow() {
        let tables = AttackTables::new();
        // Yellow pawn on e11 (file 4, rank 10): attacks d10 and f10
        let sq = square_from(4, 10).unwrap();
        let attacks = tables.pawn_attack_squares(2, sq);
        assert_eq!(attacks.len(), 2);
        assert!(attacks.contains(&square_from(3, 9).unwrap())); // d10
        assert!(attacks.contains(&square_from(5, 9).unwrap())); // f10
    }

    #[test]
    fn test_pawn_attacks_green() {
        let tables = AttackTables::new();
        // Green pawn on m5 (file 12, rank 4): attacks l4 and l6
        let sq = square_from(12, 4).unwrap();
        let attacks = tables.pawn_attack_squares(3, sq);
        assert_eq!(attacks.len(), 2);
        assert!(attacks.contains(&square_from(11, 3).unwrap())); // l4
        assert!(attacks.contains(&square_from(11, 5).unwrap())); // l6
    }

    #[test]
    fn test_ray_north_from_bottom_edge() {
        let tables = AttackTables::new();
        // d1 (file 3, rank 0) — north ray should go all the way to d14
        let sq = square_from(3, 0).unwrap();
        let north_ray = tables.ray(sq, DIR_NORTH);
        assert_eq!(north_ray.len(), 13); // ranks 1-13
    }

    #[test]
    fn test_all_valid_squares_have_king_moves() {
        let tables = AttackTables::new();
        for sq in 0..TOTAL_SQUARES as u8 {
            if is_valid_square(sq) {
                assert!(
                    !tables.king_destinations(sq).is_empty(),
                    "valid square {} should have at least one king move",
                    sq
                );
            }
        }
    }
}
