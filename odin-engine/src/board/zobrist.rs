// Zobrist hash key generation for four-player chess.
//
// Keys: random u64 per (square, piece_type, owner),
// plus castling rights (256 entries for 8-bit key),
// en passant (14 files), and side to move (4 players).
// Fixed seed for reproducibility.

use super::square::{BOARD_SIZE, TOTAL_SQUARES};
use super::types::{PIECE_TYPE_COUNT, PLAYER_COUNT};

/// Total piece-square keys: 196 squares x 7 types x 4 owners.
const PIECE_SQUARE_KEYS: usize = TOTAL_SQUARES * PIECE_TYPE_COUNT * PLAYER_COUNT;

/// Castling rights are 8 bits (2 per player), so 256 possible states.
const CASTLING_KEYS: usize = 256;

/// En passant keys: one per file (14 files).
const EN_PASSANT_KEYS: usize = BOARD_SIZE;

/// Side to move keys: one per player (4).
const SIDE_TO_MOVE_KEYS: usize = PLAYER_COUNT;

/// Pre-computed Zobrist hash keys for four-player chess.
///
/// Generated from a fixed seed (xorshift64) for reproducibility.
/// The same keys must be used across the entire engine.
pub struct ZobristKeys {
    piece_square: [u64; PIECE_SQUARE_KEYS],
    castling: [u64; CASTLING_KEYS],
    en_passant: [u64; EN_PASSANT_KEYS],
    side_to_move: [u64; SIDE_TO_MOVE_KEYS],
}

impl Default for ZobristKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl ZobristKeys {
    /// Generate all Zobrist keys from a fixed seed.
    pub fn new() -> Self {
        let mut rng = XorShift64::new(0x3243F6A8885A308D); // Fixed seed (sqrt(2) hex)

        let mut piece_square = [0u64; PIECE_SQUARE_KEYS];
        for key in piece_square.iter_mut() {
            *key = rng.next();
        }

        let mut castling = [0u64; CASTLING_KEYS];
        // Index 0 = no castling rights, so leave it as 0 (XOR with 0 = no change)
        for key in castling.iter_mut().skip(1) {
            *key = rng.next();
        }

        let mut en_passant = [0u64; EN_PASSANT_KEYS];
        for key in en_passant.iter_mut() {
            *key = rng.next();
        }

        let mut side_to_move = [0u64; SIDE_TO_MOVE_KEYS];
        for key in side_to_move.iter_mut() {
            *key = rng.next();
        }

        Self {
            piece_square,
            castling,
            en_passant,
            side_to_move,
        }
    }

    /// Key for a piece on a given square.
    /// Index: square * (7 * 4) + piece_type * 4 + player.
    #[inline]
    pub fn piece_key(&self, square: u8, piece_type_idx: usize, player_idx: usize) -> u64 {
        let idx = (square as usize) * (PIECE_TYPE_COUNT * PLAYER_COUNT)
            + piece_type_idx * PLAYER_COUNT
            + player_idx;
        self.piece_square[idx]
    }

    /// Key for the full 8-bit castling rights value.
    #[inline]
    pub fn castling_key(&self, rights: u8) -> u64 {
        self.castling[rights as usize]
    }

    /// Key for en passant on a given file (0-13).
    #[inline]
    pub fn en_passant_key(&self, file: u8) -> u64 {
        self.en_passant[file as usize]
    }

    /// Key for the side to move (player index 0-3).
    #[inline]
    pub fn side_to_move_key(&self, player_idx: usize) -> u64 {
        self.side_to_move[player_idx]
    }
}

/// Simple xorshift64 PRNG for Zobrist key generation. Not cryptographic —
/// only needs to produce well-distributed u64 values from a fixed seed.
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zobrist_keys_are_deterministic() {
        let keys1 = ZobristKeys::new();
        let keys2 = ZobristKeys::new();
        // Same seed produces same keys
        assert_eq!(keys1.piece_key(42, 0, 0), keys2.piece_key(42, 0, 0));
        assert_eq!(keys1.castling_key(255), keys2.castling_key(255));
        assert_eq!(keys1.en_passant_key(7), keys2.en_passant_key(7));
        assert_eq!(keys1.side_to_move_key(0), keys2.side_to_move_key(0));
    }

    #[test]
    fn test_zobrist_keys_are_nonzero() {
        let keys = ZobristKeys::new();
        // Piece keys should be nonzero
        assert_ne!(keys.piece_key(42, 0, 0), 0);
        assert_ne!(keys.piece_key(100, 3, 2), 0);
        // Side to move keys should be nonzero
        for i in 0..4 {
            assert_ne!(keys.side_to_move_key(i), 0);
        }
    }

    #[test]
    fn test_zobrist_keys_are_distinct() {
        let keys = ZobristKeys::new();
        // Different squares/pieces should produce different keys
        let k1 = keys.piece_key(42, 0, 0);
        let k2 = keys.piece_key(42, 0, 1);
        let k3 = keys.piece_key(43, 0, 0);
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_castling_key_zero_for_no_rights() {
        let keys = ZobristKeys::new();
        assert_eq!(keys.castling_key(0), 0);
    }

    #[test]
    fn test_xorshift_no_zero_output() {
        let mut rng = XorShift64::new(0x3243F6A8885A308D);
        for _ in 0..1000 {
            assert_ne!(rng.next(), 0);
        }
    }

    #[test]
    fn test_side_to_move_keys_all_different() {
        let keys = ZobristKeys::new();
        let stm: Vec<u64> = (0..4).map(|i| keys.side_to_move_key(i)).collect();
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert_ne!(stm[i], stm[j], "side_to_move keys {i} and {j} collide");
            }
        }
    }
}
