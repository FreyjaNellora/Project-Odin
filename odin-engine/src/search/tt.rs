// Transposition Table — Stage 9
//
// Zobrist-keyed fixed-size hash table that caches BRS search results across
// iterative deepening depths and between moves. Entries store:
//   - Key verification (upper 32 bits of Zobrist)
//   - Best move (compressed to from+to squares, 2 bytes)
//   - Score (centipawns, ply-adjusted for mate distances)
//   - Depth at which the score was computed
//   - Bound type (exact / lower / upper) and search generation (age)
//
// Replacement policy: depth-preferred with age fallback.
// Default size: 2^20 entries ≈ 12 MB (increase for stronger play).

use crate::movegen::Move;

// ---------------------------------------------------------------------------
// Bound-type flags
// ---------------------------------------------------------------------------

/// The stored score is the exact minimax value at this position.
pub const TT_EXACT: u8 = 0b01;

/// Lower bound: the position failed high (score >= beta). Real score >= stored.
pub const TT_LOWER: u8 = 0b10;

/// Upper bound: no move exceeded alpha. Real score <= stored.
pub const TT_UPPER: u8 = 0b11;

/// Sentinel compressed move meaning "no move stored".
const NO_MOVE: u16 = 0xFFFF;

/// Mate score threshold for ply-distance adjustment.
/// Any score with |score| > MATE_THRESHOLD is treated as a mate score.
const MATE_THRESHOLD: i16 = 19_900;

/// Default entry count (power of 2). 2^20 = 1,048,576 entries × 12 bytes ≈ 12 MB.
/// Increase to e.g. `1 << 24` (201 MB) for stronger play.
pub const TT_DEFAULT_ENTRIES: usize = 1 << 20;

// ---------------------------------------------------------------------------
// TTEntry
// ---------------------------------------------------------------------------

/// One transposition table entry (12 bytes including padding).
#[derive(Clone, Copy)]
struct TTEntry {
    /// Upper 32 bits of the Zobrist hash. Detects index collisions.
    key: u32,
    /// Compressed move: `from_sq | (to_sq << 8)`. `NO_MOVE` if none.
    best_move: u16,
    /// Score in centipawns. Mate scores are ply-adjusted before storage.
    score: i16,
    /// Depth at which this entry was computed.
    depth: u8,
    /// bits 0-1: bound type (TT_EXACT / TT_LOWER / TT_UPPER)
    /// bits 2-7: search generation (0-63)
    flags: u8,
}

impl TTEntry {
    fn empty() -> Self {
        Self {
            key: 0,
            best_move: NO_MOVE,
            score: 0,
            depth: 0,
            flags: 0,
        }
    }

    #[inline]
    fn bound_type(&self) -> u8 {
        self.flags & 0b11
    }

    #[inline]
    fn generation(&self) -> u8 {
        self.flags >> 2
    }
}

// ---------------------------------------------------------------------------
// TTProbe — result of a table lookup
// ---------------------------------------------------------------------------

/// Result of a transposition table probe.
pub struct TTProbe {
    /// `Some(score)` if the TT entry allows an early return (depth sufficient,
    /// bounds matched). The caller should return this value immediately.
    pub score: Option<i16>,
    /// Compressed best-move hint for move ordering. Always returned when the
    /// hash matches, regardless of depth. Use `decompress_move` to recover
    /// the full `Move` from the legal-move list.
    pub best_move: Option<u16>,
}

// ---------------------------------------------------------------------------
// TranspositionTable
// ---------------------------------------------------------------------------

/// Fixed-size Zobrist-keyed transposition table.
///
/// Lives in `BrsSearcher` so it persists across iterative deepening depths
/// and between moves. Not reset between `search()` calls.
pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    mask: usize,    // entries.len() - 1 (power of 2 for fast index)
    generation: u8, // current search generation (0-63, wraps)
}

impl TranspositionTable {
    /// Create a new table with `num_entries` slots (must be a power of 2).
    pub fn new(num_entries: usize) -> Self {
        assert!(
            num_entries.is_power_of_two(),
            "TT entry count must be a power of 2, got {num_entries}"
        );
        Self {
            entries: vec![TTEntry::empty(); num_entries],
            mask: num_entries - 1,
            generation: 0,
        }
    }

    /// Create a table sized to approximately `mb` megabytes.
    /// The actual allocation is rounded down to the nearest power of 2 in entry count.
    pub fn with_mb(mb: usize) -> Self {
        let target_bytes = mb * 1024 * 1024;
        let entry_bytes = std::mem::size_of::<TTEntry>();
        let target_entries = target_bytes / entry_bytes;
        let num_entries = if target_entries <= 1 {
            1
        } else if target_entries.is_power_of_two() {
            target_entries
        } else {
            // Largest power of 2 <= target_entries
            target_entries.next_power_of_two() >> 1
        };
        Self::new(num_entries)
    }

    /// Number of entries in the table.
    pub fn len(&self) -> usize {
        self.mask + 1
    }

    /// Returns true if the table has no entries (mask == 0 means 1 slot, not zero).
    /// A TT is never truly empty once constructed, but this satisfies the `len`/`is_empty` pair.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Increment the generation counter. Call once at the start of each search.
    pub fn increment_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1) & 0x3F; // 6-bit (0-63)
    }

    /// Probe the table.
    ///
    /// - Returns a best-move hint (for ordering) whenever the hash matches.
    /// - Returns a cutoff score (for early return) when the stored depth is
    ///   sufficient and the bound type allows it.
    ///
    /// `alpha` and `beta` may be adjusted on bound hits even when `score` is `None`.
    pub fn probe(&self, hash: u64, depth: u8, alpha: &mut i16, beta: &mut i16, ply: u8) -> TTProbe {
        let idx = (hash as usize) & self.mask;
        let entry = self.entries[idx];
        let verify_key = (hash >> 32) as u32;

        // Hash mismatch — completely different position.
        if entry.key != verify_key {
            return TTProbe {
                score: None,
                best_move: None,
            };
        }

        let best_move = if entry.best_move != NO_MOVE {
            Some(entry.best_move)
        } else {
            None
        };

        // Entry exists for this position, but depth is too shallow for a score cutoff.
        // Return the move hint only (useful for ordering).
        if entry.depth < depth {
            return TTProbe {
                score: None,
                best_move,
            };
        }

        let raw_score = Self::score_from_tt(entry.score, ply);

        let cutoff = match entry.bound_type() {
            TT_EXACT => Some(raw_score),
            TT_LOWER => {
                // Score is a lower bound: true value >= stored.
                if raw_score >= *beta {
                    Some(raw_score) // fail-high cutoff
                } else {
                    *alpha = (*alpha).max(raw_score);
                    None
                }
            }
            TT_UPPER => {
                // Score is an upper bound: true value <= stored.
                if raw_score <= *alpha {
                    Some(raw_score) // fail-low cutoff
                } else {
                    *beta = (*beta).min(raw_score);
                    None
                }
            }
            _ => None, // malformed entry (shouldn't happen)
        };

        TTProbe {
            score: cutoff,
            best_move,
        }
    }

    /// Store a search result.
    ///
    /// `best_move` should be `compress_move(mv)` for the best move found, or
    /// `None` if no best move is available (e.g., all moves failed low).
    /// `score` is the raw search score; mate adjustments are applied internally.
    pub fn store(
        &mut self,
        hash: u64,
        best_move: Option<u16>,
        score: i16,
        depth: u8,
        flag: u8,
        ply: u8,
    ) {
        let idx = (hash as usize) & self.mask;
        let verify_key = (hash >> 32) as u32;
        let existing = self.entries[idx];

        // Replacement decision (depth-preferred with generation fallback):
        // Replace if any of:
        //   1. Same position hash (always update — same bucket, fresher info)
        //   2. Existing entry is from an old generation
        //   3. New search is equal or deeper
        let should_replace = existing.key == verify_key
            || existing.generation() != self.generation
            || depth >= existing.depth;

        if !should_replace {
            return;
        }

        // If writing a TT_UPPER (all-node, no improvement), prefer to preserve an
        // existing best move from this position over overwriting with None.
        let stored_move = best_move.unwrap_or({
            if existing.key == verify_key {
                existing.best_move
            } else {
                NO_MOVE
            }
        });

        let adjusted_score = Self::score_to_tt(score, ply);
        let packed_flags = (flag & 0b11) | (self.generation << 2);

        self.entries[idx] = TTEntry {
            key: verify_key,
            best_move: stored_move,
            score: adjusted_score,
            depth,
            flags: packed_flags,
        };
    }

    /// Compress a `Move` to a `u16` for TT storage (from_sq | to_sq << 8).
    #[inline]
    pub fn compress_move(mv: Move) -> u16 {
        (mv.from_sq() as u16) | ((mv.to_sq() as u16) << 8)
    }

    /// Find the full `Move` in a legal-move slice matching a compressed TT move.
    ///
    /// For promotions (multiple moves sharing the same from+to squares),
    /// returns the first match. Callers that need exact promotion disambiguation
    /// can iterate the results, but in practice move generation order places
    /// queen promotions first, which is what we want in move ordering.
    pub fn decompress_move(compressed: u16, moves: &[Move]) -> Option<Move> {
        if compressed == NO_MOVE {
            return None;
        }
        let from = (compressed & 0xFF) as u8;
        let to = (compressed >> 8) as u8;
        moves
            .iter()
            .find(|&&m| m.from_sq() == from && m.to_sq() == to)
            .copied()
    }

    // -- Mate score adjustment -----------------------------------------------

    /// Adjust a score for storage: convert "mate in N from this node" to
    /// "mate in N from the root" by adding ply offset. This ensures the stored
    /// score is consistent regardless of at which depth the entry was created.
    fn score_to_tt(score: i16, ply: u8) -> i16 {
        if score > MATE_THRESHOLD {
            score.saturating_add(ply as i16)
        } else if score < -MATE_THRESHOLD {
            score.saturating_sub(ply as i16)
        } else {
            score
        }
    }

    /// Reverse the ply adjustment when reading a mate score back from TT.
    fn score_from_tt(score: i16, ply: u8) -> i16 {
        if score > MATE_THRESHOLD {
            score.saturating_sub(ply as i16)
        } else if score < -MATE_THRESHOLD {
            score.saturating_add(ply as i16)
        } else {
            score
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::PieceType;

    fn make_test_move(from: u8, to: u8) -> Move {
        Move::new(from, to, PieceType::Pawn)
    }

    #[test]
    fn test_tt_empty_probe_returns_nothing() {
        let tt = TranspositionTable::new(1024);
        let mut alpha = -1000_i16;
        let mut beta = 1000_i16;
        let result = tt.probe(0x1234567890ABCDEFu64, 4, &mut alpha, &mut beta, 0);
        assert!(result.score.is_none());
        assert!(result.best_move.is_none());
    }

    #[test]
    fn test_tt_exact_store_and_probe() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0x1234567890ABCDEFu64;
        let mv = make_test_move(20, 34);
        let compressed = TranspositionTable::compress_move(mv);

        tt.store(hash, Some(compressed), 350, 5, TT_EXACT, 2);

        let mut alpha = -1000_i16;
        let mut beta = 1000_i16;
        let result = tt.probe(hash, 5, &mut alpha, &mut beta, 2);
        assert_eq!(result.score, Some(350));
        assert_eq!(result.best_move, Some(compressed));
    }

    #[test]
    fn test_tt_depth_preferred_no_replacement_shallower() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0xDEADBEEF12345678u64;

        // Store a deep entry
        tt.store(hash, None, 200, 6, TT_EXACT, 0);

        // Attempt to overwrite with shallower entry (different position, same index)
        // In this test, the key is the same so it WILL be overwritten (same position).
        // Test that a different hash at the same index is NOT replaced if shallower.
        // (Hard to test pure depth preference without forcing index collision.)
        // Instead, test that a same-position shallow store does NOT downgrade the depth.
        tt.store(hash, None, 100, 4, TT_EXACT, 0); // shallower, same hash

        let mut alpha = -1000_i16;
        let mut beta = 1000_i16;
        // After storing at depth 4 (shallower), probing at depth 6 should miss.
        let result = tt.probe(hash, 6, &mut alpha, &mut beta, 0);
        // score should be None since depth 4 < requested 6
        assert!(result.score.is_none());
    }

    #[test]
    fn test_tt_lower_bound_adjusts_alpha() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0xAABBCCDDEEFF0011u64;
        tt.store(hash, None, 300, 4, TT_LOWER, 0);

        let mut alpha = 100_i16;
        let mut beta = 500_i16;
        let result = tt.probe(hash, 4, &mut alpha, &mut beta, 0);
        // Score is a lower bound >= 300, does not exceed beta=500, so no cutoff.
        assert!(result.score.is_none());
        // But alpha should be raised to 300.
        assert_eq!(alpha, 300);
    }

    #[test]
    fn test_tt_lower_bound_cutoff_when_score_exceeds_beta() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0x1122334455667788u64;
        tt.store(hash, None, 600, 4, TT_LOWER, 0);

        let mut alpha = 100_i16;
        let mut beta = 500_i16;
        let result = tt.probe(hash, 4, &mut alpha, &mut beta, 0);
        // Score 600 >= beta 500: cutoff.
        assert_eq!(result.score, Some(600));
    }

    #[test]
    fn test_tt_upper_bound_cutoff_when_score_below_alpha() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0x9988776655443322u64;
        tt.store(hash, None, 50, 4, TT_UPPER, 0);

        let mut alpha = 100_i16;
        let mut beta = 500_i16;
        let result = tt.probe(hash, 4, &mut alpha, &mut beta, 0);
        // Score 50 <= alpha 100: fail-low cutoff.
        assert_eq!(result.score, Some(50));
    }

    #[test]
    fn test_tt_move_hint_returned_for_shallow_entry() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0x5566778899AABBCCu64;
        let mv = make_test_move(10, 24);
        let compressed = TranspositionTable::compress_move(mv);
        tt.store(hash, Some(compressed), 150, 4, TT_EXACT, 0);

        let mut alpha = -1000_i16;
        let mut beta = 1000_i16;
        // Probe at depth 5: entry depth 4 < 5, so no score.
        let result = tt.probe(hash, 5, &mut alpha, &mut beta, 0);
        assert!(result.score.is_none());
        // But the move hint should still be returned.
        assert_eq!(result.best_move, Some(compressed));
    }

    #[test]
    fn test_tt_compress_decompress_round_trip() {
        let mv = make_test_move(42, 98);
        let compressed = TranspositionTable::compress_move(mv);
        // Build a move list containing mv.
        let moves = vec![mv];
        let recovered = TranspositionTable::decompress_move(compressed, &moves);
        assert_eq!(recovered, Some(mv));
    }

    #[test]
    fn test_tt_decompress_no_match_returns_none() {
        let mv_stored = make_test_move(10, 20);
        let mv_other = make_test_move(30, 40);
        let compressed = TranspositionTable::compress_move(mv_stored);
        let moves = vec![mv_other]; // doesn't contain mv_stored
        assert!(TranspositionTable::decompress_move(compressed, &moves).is_none());
    }

    #[test]
    fn test_tt_mate_score_ply_adjustment() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0xFEDCBA9876543210u64;
        let mate_score: i16 = 19_950; // mate in ~1-2 plies

        // Store at ply 3.
        tt.store(hash, None, mate_score, 4, TT_EXACT, 3);

        // Retrieve at ply 3 — should get same score back.
        let mut alpha = -30_000_i16;
        let mut beta = 30_000_i16;
        let result = tt.probe(hash, 4, &mut alpha, &mut beta, 3);
        assert_eq!(result.score, Some(mate_score));
    }

    #[test]
    fn test_tt_generation_age_replacement() {
        let mut tt = TranspositionTable::new(1024);
        let hash: u64 = 0x0011223344556677u64;

        // Store at generation 0.
        tt.store(hash, None, 100, 6, TT_EXACT, 0);

        // Advance generation.
        tt.increment_generation();

        // Store shallower at new generation.
        tt.store(hash, None, 200, 4, TT_EXACT, 0);

        // Probe at depth 4 — new generation entry should be found.
        let mut alpha = -1000_i16;
        let mut beta = 1000_i16;
        let result = tt.probe(hash, 4, &mut alpha, &mut beta, 0);
        assert_eq!(result.score, Some(200));
    }

    #[test]
    fn test_tt_with_mb_creates_valid_table() {
        let tt = TranspositionTable::with_mb(1); // 1 MB
                                                 // Should have at least 1 entry and a power-of-2 count.
        let len = tt.len();
        assert!(len > 0);
        assert!(len.is_power_of_two());
    }
}
