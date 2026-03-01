// Chess960 (Fischer Random) position generator for 4-player chess.
//
// Generates a valid back rank arrangement satisfying Chess960 constraints:
//   - Bishops on opposite-colored squares
//   - King between the two rooks
//   - 8 pieces placed: R, N, B, Q, K, B, N, R (some permutation)
//
// The same logical arrangement is rotated/mirrored for all 4 players.

use crate::board::PieceType;
use crate::util::SplitMix64;

/// Generate a valid Chess960 back rank arrangement from a seed.
///
/// Returns `[PieceType; 8]` representing the pieces on files 0-7 (mapped to
/// each player's back rank files 3-10). Satisfies Chess960 constraints:
///   - Bishops on opposite-colored squares (even/odd index parity)
///   - King between the two rooks
///
/// The algorithm places pieces in order: bishops (forced to opposite colors),
/// queen, knights, then R-K-R fills the remaining 3 slots left-to-right,
/// which guarantees king-between-rooks.
pub fn generate_back_rank(seed: u64) -> [PieceType; 8] {
    let mut rng = SplitMix64::new(seed);
    let mut rank = [None; 8];

    // 1. Place first bishop on a random even-index square (0, 2, 4, 6)
    let even_squares = [0usize, 2, 4, 6];
    let idx = (rng.next_u64() % 4) as usize;
    rank[even_squares[idx]] = Some(PieceType::Bishop);

    // 2. Place second bishop on a random odd-index square (1, 3, 5, 7)
    let odd_squares = [1usize, 3, 5, 7];
    let idx = (rng.next_u64() % 4) as usize;
    rank[odd_squares[idx]] = Some(PieceType::Bishop);

    // 3. Place queen on a random empty square (6 remaining)
    let empty: Vec<usize> = (0..8).filter(|&i| rank[i].is_none()).collect();
    let idx = (rng.next_u64() % empty.len() as u64) as usize;
    rank[empty[idx]] = Some(PieceType::Queen);

    // 4. Place first knight on a random empty square (5 remaining)
    let empty: Vec<usize> = (0..8).filter(|&i| rank[i].is_none()).collect();
    let idx = (rng.next_u64() % empty.len() as u64) as usize;
    rank[empty[idx]] = Some(PieceType::Knight);

    // 5. Place second knight on a random empty square (4 remaining)
    let empty: Vec<usize> = (0..8).filter(|&i| rank[i].is_none()).collect();
    let idx = (rng.next_u64() % empty.len() as u64) as usize;
    rank[empty[idx]] = Some(PieceType::Knight);

    // 6. Place R-K-R in the 3 remaining squares (left-to-right).
    //    This guarantees king is between the two rooks.
    let empty: Vec<usize> = (0..8).filter(|&i| rank[i].is_none()).collect();
    debug_assert_eq!(empty.len(), 3);
    rank[empty[0]] = Some(PieceType::Rook);
    rank[empty[1]] = Some(PieceType::King);
    rank[empty[2]] = Some(PieceType::Rook);

    // Convert Option<PieceType> to PieceType
    rank.map(|slot| slot.unwrap())
}

/// Validate that a back rank arrangement satisfies Chess960 constraints.
/// Used by unit tests and integration tests (stage_17).
#[allow(dead_code)]
pub fn is_valid_chess960(rank: &[PieceType; 8]) -> bool {
    // Find bishops
    let bishops: Vec<usize> = rank
        .iter()
        .enumerate()
        .filter(|(_, &pt)| pt == PieceType::Bishop)
        .map(|(i, _)| i)
        .collect();
    if bishops.len() != 2 {
        return false;
    }
    // Bishops must be on opposite-colored squares
    if bishops[0] % 2 == bishops[1] % 2 {
        return false;
    }

    // Find king and rooks
    let king_pos = rank.iter().position(|&pt| pt == PieceType::King);
    let rooks: Vec<usize> = rank
        .iter()
        .enumerate()
        .filter(|(_, &pt)| pt == PieceType::Rook)
        .map(|(i, _)| i)
        .collect();
    if rooks.len() != 2 || king_pos.is_none() {
        return false;
    }
    let king = king_pos.unwrap();
    // King must be between the two rooks
    if !(rooks[0] < king && king < rooks[1]) {
        return false;
    }

    // Piece counts: 2R, 2N, 2B, 1Q, 1K
    let mut counts = [0u8; 7]; // indexed by PieceType
    for &pt in rank {
        counts[pt.index()] += 1;
    }
    counts[PieceType::Rook.index()] == 2
        && counts[PieceType::Knight.index()] == 2
        && counts[PieceType::Bishop.index()] == 2
        && counts[PieceType::Queen.index()] == 1
        && counts[PieceType::King.index()] == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_back_rank_is_valid() {
        for seed in 0..100 {
            let rank = generate_back_rank(seed);
            assert!(
                is_valid_chess960(&rank),
                "seed {seed} produced invalid arrangement: {rank:?}"
            );
        }
    }

    #[test]
    fn test_deterministic_seed() {
        let a = generate_back_rank(42);
        let b = generate_back_rank(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_seeds_produce_variety() {
        let mut arrangements = std::collections::HashSet::new();
        for seed in 0..50 {
            arrangements.insert(generate_back_rank(seed));
        }
        // With 50 seeds out of 960 possible arrangements, expect multiple distinct ones
        assert!(
            arrangements.len() > 5,
            "only {} distinct arrangements from 50 seeds",
            arrangements.len()
        );
    }
}
