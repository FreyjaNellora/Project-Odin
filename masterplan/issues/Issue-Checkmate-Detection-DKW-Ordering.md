# Issue: Checkmate Not Detected — DKW Ordering + Protocol Early Return

**Date:** 2026-02-21
**Stage:** 7 (post-completion regression)
**Status:** RESOLVED

## Symptom

After Green played n7i2 (queen to i2), Red's king at h1 was in checkmate. The engine emitted `nextturn Red` instead of `eliminated Red checkmate` + `nextturn Blue`. The human player was stuck: the game would not advance, and any attempted move was rejected as illegal.

## Root Cause

Two bugs in sequence:

### Bug A — Wrong ordering in `apply_move` (root cause)

`odin-engine/src/gamestate/mod.rs`, `apply_move`:

`check_elimination_chain` ran **before** `process_dkw_moves`. If a DKW piece (belonging to a previously resigned player) was the only piece Red could legally capture, `check_elimination_chain` saw that capture and returned `HasMoves`. Then `process_dkw_moves` ran and the DKW piece wandered to a different square. Red now had no legal moves — but the checkmate test had already passed.

### Bug B — Protocol early return (safety net)

`odin-engine/src/protocol/mod.rs`, `handle_go`:

```rust
if gs.legal_moves().is_empty() {
    self.send(&format_error("no legal moves available"));
    return;
}
```

When `go` was called for a player with no legal moves, the protocol emitted an error and returned. `apply_move` was never called, `check_elimination_chain` was never reached, and the player was never eliminated.

## Fix

### Fix A — Swap step order in `apply_move`

Moved `process_dkw_moves` to run **before** `check_elimination_chain`. DKW pieces now settle into their post-move positions before checkmate detection evaluates the board.

New order in `apply_move`:
1–5. (unchanged)
6. `process_dkw_moves` — DKW pieces move first
7. `check_elimination_chain` — checkmate detection sees the final board
8. `check_game_over`

### Fix B — Handle no-legal-moves in `handle_go`

Added `GameState::handle_no_legal_moves()` public method that runs `check_elimination_chain` + `process_dkw_moves` + `check_game_over` without requiring a move.

In `handle_go`, replaced the error-and-return with:
- Call `handle_no_legal_moves()`
- Emit `info string eliminated <color> checkmate/stalemate` for each eliminated player
- Emit `info string gameover` or `info string nextturn <next>`
- Recurse into `handle_go` so the engine produces a `bestmove` for the next alive player

## Verification (session 1)

`cargo test --lib` — 199/199 passing after fix. Includes two new `gamestate` tests (`test_handle_no_legal_moves_checkmate`, `test_handle_no_legal_moves_stalemate`) and one new `protocol` test (`test_go_mated_player_emits_eliminated_and_advances`).

---

## Addendum: Bug C — UI Parser Drops Reason-Suffixed Elimination Events (2026-02-21)

**Status:** RESOLVED (session 2)

### Symptom

After the session-1 fixes, the user still reported "Red checkmated but engine stops instead of advancing to Blue." The engine was correctly detecting and emitting the checkmate event, but the UI was silently ignoring it.

### Root Cause

`odin-ui/src/lib/protocol-parser.ts`, line 46:

```typescript
const color = trimmed.slice('info string eliminated '.length).trim();
if (isValidPlayerColor(color)) { ... }
```

The `handle_no_legal_moves` path in the protocol emits `info string eliminated Red checkmate` (with reason word). After slicing the prefix, the parser obtained `"Red checkmate"` as the color string. `isValidPlayerColor("Red checkmate")` returns false, so the entire event was silently dropped. `eliminatedPlayersRef` was never updated in the UI, so Red appeared to remain active in the UI's turn tracker.

### Fix

Extract only the first whitespace-delimited token:

```typescript
const rest = trimmed.slice('info string eliminated '.length).trim();
const color = rest.split(/\s+/)[0];
if (isValidPlayerColor(color)) { ... }
```

### Secondary Fix (same session)

The Bug B fix (session 1) added `info string nextturn Blue` to the normal (non-checkmate) `handle_go` output path. Three Stage 7 integration tests in `odin-engine/tests/stage_07_brs.rs` assumed only search info lines were emitted and failed. Updated tests to filter out `info string` lines when counting/validating depth-based search lines.

### Verification (session 2)

- Engine: 199 lib + 305 integration tests, all passing
- UI (Vitest): 54 tests passing (9 new tests for `eliminated`, `nextturn`, `gameover` parsing)
