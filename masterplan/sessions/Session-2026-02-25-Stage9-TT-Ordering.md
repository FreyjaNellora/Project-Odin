# Session — Stage 9: TT & Move Ordering

**Date:** 2026-02-25 (third session of the day)
**Model:** Claude Sonnet 4.6
**Stage:** Stage 9 — Transposition Table & Move Ordering
**Duration:** Single session, full implementation

## Summary

Stage 9 implemented in full. A transposition table (Zobrist-keyed, depth-preferred replacement) and a complete move ordering pipeline (TT hint → winning captures → killers → counter-move → history-sorted quiets → losing captures) were added to the BRS search engine. The result: 58% node reduction at depth 6 vs the Stage 7 baseline, exceeding the >50% acceptance criterion.

## What Was Built

### `search/tt.rs` (new)
- `TTEntry`: 12-byte entries (key u32, move u16, score i16, depth u8, flags u8)
- `TranspositionTable`: Vec-backed, power-of-2 mask indexing, 6-bit generation counter
- Probe: returns `TTProbe { score: Option<i16>, best_move: Option<u16> }`
- Bound types: TT_EXACT, TT_LOWER (failed high), TT_UPPER (failed low)
- Mate score ply adjustment: `score_to_tt` / `score_from_tt` with MATE_THRESHOLD = 19,900
- Replacement policy: same-position always updates; generation mismatch always replaces; depth-preferred otherwise
- 12 unit tests

### `search/brs.rs` additions
- `BrsSearcher.tt`: TT persists across searches (not reset between `search()` calls)
- `BrsContext`: killers, history, countermoves (flat Vec), last_opp_move
- `alphabeta()`: hash computed once, TT probe after rep-check (critical ordering), TT store at bottom (skipped when stopped=true), `orig_alpha` saved for correct flag computation
- `max_node()`: uses `tt_move.or(pv_move)` hint
- Beta cutoff tracking: killers updated, history += depth², counter-move recorded
- `min_node()`: sets `last_opp_move[ply+1]` before recursing
- `see(mv, threshold)`: simplified single-exchange SEE
- `order_moves()`: full 7-bucket pipeline

### `tests/stage_09_tt_ordering.rs` (new, 13 tests)

## Key Decisions

1. **TT probe placed AFTER repetition check**: Critical invariant. TT must not bypass draw detection. Test `test_tt_does_not_bypass_repetition_detection` guards this.

2. **Aborted searches skip TT store**: Partial results from time-budget-exceeded searches would poison the TT with scores that don't reflect the full position analysis.

3. **Eliminated-player skip nodes NOT stored in TT**: Structural transitions (not real search positions). Acceptable cost — these are cheap and not common transpositions.

4. **Counter-move as flat `Vec<Option<Move>>`**: Avoids stack-overflow risk from large 2D array initialization (`Box<[[Option<Move>; 196]; 196]>` would require unsafe or iterator-based init). Flat Vec is clean and safe.

5. **Simplified SEE**: `captured_val - attacker_val >= threshold` only. Full recursive 4PC SEE deferred to Stage 19. This covers the most impactful classification (pawn takes queen = win, queen takes defended pawn = loss).

6. **TT idempotence is score-stable, not move-stable**: Two identical depth-4 searches may return different legal moves (equal-score alternatives), but MUST return the same score. Test reflects this reality.

## Performance

| Depth | Stage 7 Baseline | Stage 9 Result | Reduction |
|-------|------------------|----------------|-----------|
| 6     | 10,916 nodes     | 4,595 nodes    | **58%**   |
| 8     | 31,896 nodes     | 13,009 nodes   | **59%**   |

Depth 8 in 120ms (release). Acceptance criterion >50% at depth 6: met.

## Bugs Fixed During Implementation

1. **`GameState::from_fen4` does not exist**: Integration test test #7 originally tried to construct a position from a FEN4 string via this method. Replaced with starting position + positive score check.

2. **perft(1) = 20, not 40**: The test initially expected 40 (confusing "4 players × 10 moves" with the actual 20 Red legal moves at the starting position). Fixed to 20.

3. **TT non-determinism in "idempotent" test**: Two depth-4 searches returned different equal-score moves (e1f3 vs d2d3). This is correct TT behavior — different move ordering from TT hits changes which equal-score line is explored first. Test updated to check score equality, not move equality.

4. **Monotone node count with warm TT**: Node count at depth 3 was LESS than depth 2 because TT carryover from the depth 2 search reduced depth 3 dramatically. Fixed test to use fresh searchers per depth.

## Related

- Audit log: [[audit_log_stage_09]]
- Downstream log: [[downstream_log_stage_09]]
- Stage spec: [[stage_09_tt_ordering]]
- Previous session: [[Session-2026-02-25-PostElim-Crash-Fix]]
