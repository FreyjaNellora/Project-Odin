# Audit Log — Stage 17: Game Mode Variant Tuning

## Pre-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes — 0 errors, 0 warnings
- Tests pass: 557 total (308 unit + 249 integration, 6 ignored), 0 failures
- Previous downstream flags reviewed: W17-W25 from Stage 16 downstream log

### Findings

1. **Chess960 castling make/unmake required atomic handling.** Standard `move_piece(from, to)` panics when king destination overlaps rook start (or vice versa). Fixed by remove-both-then-place pattern in both make_move and unmake_move for FLAG_CASTLE_KING and FLAG_CASTLE_QUEEN.

2. **Board::empty() castling_starts initialization.** Initially set to `[(0,0,0); 4]` (zeros). Square 0 = (0,0) is an invalid corner on the 14x14 board. FEN4-loaded boards start from Board::empty() and trigger `update_castling_rights` → `castling_config` → reads from castling_starts → walks between squares → hits invalid square. Fixed: Board::empty() now uses standard starting values.

3. **compute_priors signature change.** Added `board: &Board` parameter. All 3 internal call sites + 2 test call sites updated. No external callers (function is `pub(crate)`).

4. **castling_empty_squares full rewrite.** Old algorithm computed squares "between king and rook." Chess960 requires union of king-to-destination-path + rook-to-destination-path, minus both starting squares. New implementation handles all edge cases (king at destination, rook passing through king's square, overlapping paths).

### Risks for This Stage

- **R1:** Chess960 castling correctness — edge cases with king/rook at or past destination squares. MITIGATED: atomic remove-both-then-place, extensive seed testing (100 seeds).
- **R2:** Dead piece capture ordering change could affect standard game strength. LOW RISK: only changes ordering for Dead-status pieces (never present in standard games).
- **R3:** New eval terms could cause score inflation. MITIGATED: all terms use small weights (10-30cp), clamped to [-30000, 30000].

---

## Post-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Deliverables Check

- [x] Chess960 back rank generator (chess960.rs)
- [x] Board::chess960_position() with correct 4-player mirroring
- [x] Castling refactored for Chess960 compatibility
- [x] Dead piece capture ordering fix in BRS order_moves
- [x] Dead piece prior fix in MCTS compute_priors
- [x] DKW proximity penalty eval (dkw.rs)
- [x] FFA claim-win urgency eval (ffa_strategy.rs)
- [x] Terrain-aware eval (terrain.rs)
- [x] EvalWeights expanded with 5 new fields
- [x] Chess960 protocol option
- [x] 18 acceptance tests (T1-T18)
- [x] All 536 existing tests pass
- [x] 0 clippy warnings

### Code Quality
#### Uniformity
All new eval modules follow the same pattern: `pub(crate) fn xxx(board, player, statuses, weights) -> i16`. Consistent with existing eval components.

#### Bloat
3 new eval files are small (50-90 lines each). No unnecessary abstractions.

#### Efficiency
- Terrain eval: O(piece_count × 8) board lookups — negligible
- DKW penalty: O(4 × 1) king distance checks — negligible
- FFA strategy: O(4) score comparisons — negligible

#### Dead Code
- `is_valid_chess960()` marked `#[allow(dead_code)]` — used only in tests

#### Broken Code
None found.

#### Temporary Code
None.

### Search/Eval Integrity

- Evaluator trait FROZEN — no signature changes
- Searcher trait FROZEN — no signature changes
- perft(1-4) unchanged: 20/395/7800/152050
- NNUE accumulator push/pop ordering unchanged
- TT probe still after repetition check

### Future Conflict Analysis

- **Stage 18 (Full UI):** Chess960 mode needs UI toggle. `chess960` engine option is already wired.
- **Stage 19 (Optimization):** Terrain eval inner loops are not SIMD-critical (small loop counts).

### Unaccounted Concerns

None.

### Reasoning & Methods

- Chess960 mirroring verified by T2 (all 4 players produce same logical arrangement)
- Castling correctness verified by T5 (king/rook at correct squares) + T15 (full game without panic)
- perft invariant verified by T16
- Standard game regression verified by T17
- DKW game completion verified by T18

---

## Related

- Stage spec: [[stage_17_variants]]
- Downstream log: [[downstream_log_stage_17]]
