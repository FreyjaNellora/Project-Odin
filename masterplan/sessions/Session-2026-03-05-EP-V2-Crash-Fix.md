# Session: 2026-03-05 -- EP V2 Crash Fix

**Stage:** 19 Phase 5
**Date:** 2026-03-05
**Agent:** Claude Sonnet 4.6

## Summary

Investigated and fixed the EP V2 crash (`remove_piece: square is empty` at `board_struct.rs:425`). Root cause confirmed, fix verified, documentation updated.

## What Was Done

### Investigation

Ran four parallel suspect investigations:
- Suspect A (convert_to_dead changes pawn type): RULED OUT -- `set_piece_status` only changes `piece.status`, not type/owner. `piece_at` returns the piece regardless.
- Suspect B (EP generated for wrong player): Red herring -- any player with an attacking pawn can EP-capture. Generation is correct; the crash is elsewhere.
- Suspect C (TT stale EP): NOT ON THIS PATH -- crash backtrace is `expand_node -> apply_move`, no BRS/TT on stack.
- Suspect D (self-EP in check_elimination_chain): **CONFIRMED ROOT CAUSE.**

Added diagnostic panic to `find_ep_captured_pawn_sq` to capture exact board state. Output:
```
find_ep_captured_pawn_sq: no pawn found! ep_target=(2,4) capturing=Blue
scanned=[Red->(2,5)[empty], Yellow->(2,3)[empty], Green->(1,4)[empty]]
```

### Root Cause

Self-EP scenario:
1. Blue double-pushes -> ep_sq set at (2,4)
2. Red immediately in checkmate -> eliminated
3. `check_elimination_chain` advances back to Blue
4. Blue's pseudo-legal moves include EP at (2,4) (Blue pawn at (1,3) attacks it)
5. `find_ep_captured_pawn_sq` scans all players except Blue -> finds no pawn
6. Old fallback `en_passant_captured_sq(ep_target, Blue.prev())` -> wrong square -> PANIC

### Fix

**`odin-engine/src/movegen/moves.rs`:** `find_ep_captured_pawn_sq` returns `Option<Square>` (was `Square`), made `pub`. Fallback removed. `make_move` EP branch uses `.expect()`.

**`odin-engine/src/movegen/generate.rs`:** EP generation in `generate_pseudo_legal` gated on `find_ep_captured_pawn_sq(board, ep_sq, player).is_some()`. Invalid self-capture move never generated.

### Verification

`observer/reproduce_crash.mjs` 500 attempts: ZERO crashes (vs. crash at attempt 2 pre-fix).
`cargo build --release`: success, 0 warnings.

## EP Rule Clarification (from user)

- Any player with an attacking pawn can EP-capture a double-pushed pawn
- Each eligible player gets one chance -- their own immediate next turn
- Opposite-side players (Red/Yellow, Blue/Green) can't reach EP position before promotion -- treat as N/A
- The current engine clears ep_sq after every make_move (separate correctness bug, deferred)

## Deferred Issues

- EP rule correctness: ep_sq cleared too eagerly -- denies eligible players after ineligible player moves
- TT EP flag: compress_move drops EP flag, could replay stale EP in BRS layer

## Files Modified

- `odin-engine/src/movegen/moves.rs`
- `odin-engine/src/movegen/generate.rs`
- `observer/reproduce_crash.mjs` (restored to release binary)
- `masterplan/issues/Issue-EP-Remove-Piece-Crash-V2.md`
- `masterplan/HANDOFF.md`
- `masterplan/STATUS.md`

## Related

- [[Issue-EP-Remove-Piece-Crash-V2]]
- [[Session-2026-03-04-EP-V2-Investigation]] (prior session)
