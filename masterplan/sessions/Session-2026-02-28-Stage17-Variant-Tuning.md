# Session: Stage 17 — Game Mode Variant Tuning

**Date:** 2026-02-28
**Agent:** Claude Opus 4.6
**Duration:** Single session (continued from context compaction)
**Stage:** 17 — Game Mode Variant Tuning
**Status:** IMPLEMENTATION COMPLETE

## Summary

Implemented Stage 17: Chess960 position generation, DKW-aware evaluation, FFA scoring strategy, terrain-aware evaluation, and dead piece capture ordering fixes. 21 new tests (3 unit + 18 integration), all 557 engine tests pass, 0 clippy warnings.

## Key Decisions

1. **DKW chance nodes in MCTS skipped (W26)** — Random DKW king moves have negligible strategic impact. Expectimax would cost 3-5x per simulation for minimal gain.
2. **FFA self-stalemate skipped (W27)** — Too complex for marginal gain.
3. **Chess960 FEN deferred (W28)** — Only `position startpos` supported for now.
4. **Dead piece value = 1 (not 0)** — Gives dead captures minimal but non-zero priority.
5. **Board::empty() uses standard castling_starts** — Prevents FEN4 compatibility panic.

## Bugs Found

1. **Board::empty() zeros** — castling_starts initialized to (0,0,0) = invalid squares. Fixed: use standard values.
2. **Chess960 castling overlap** — `move_piece` panics when king dest = rook start. Fixed: atomic remove-both-then-place.

## Files Changed

15 files modified/created. See HANDOFF.md for complete list.

## Test Results

- Before: 536 (305 unit + 231 integration, 6 ignored)
- After: 557 (308 unit + 249 integration, 6 ignored)
- Delta: +21 (+3 unit in chess960.rs, +18 integration in stage_17_variant_tuning.rs)
- 0 failures, 0 clippy warnings

## Related

- [[audit_log_stage_17]]
- [[downstream_log_stage_17]]
