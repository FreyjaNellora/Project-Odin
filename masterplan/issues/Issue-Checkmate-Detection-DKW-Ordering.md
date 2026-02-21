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

## Verification

`cargo test --lib` — 199/199 passing after fix. Includes two new `gamestate` tests (`test_handle_no_legal_moves_checkmate`, `test_handle_no_legal_moves_stalemate`) and one new `protocol` test (`test_go_mated_player_emits_eliminated_and_advances`).
