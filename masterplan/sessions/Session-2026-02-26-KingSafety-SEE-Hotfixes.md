# Session — 2026-02-26 — King Safety + SEE Hotfixes

**Date:** 2026-02-26
**Stage:** Post-Stage-9 (non-stage gameplay quality fixes)
**Commit:** `a37b237`

## Context

Stage 9 (TT + Move Ordering) was already complete from the previous session. User launched the app (`cargo tauri dev`) and observed two gameplay quality bugs in a live game.

## Bugs Fixed

### Bug 1 — King Walk (Blue Ka7b6)

**Symptom:** Blue's king walked forward without penalty. After pawn pushes from the opening, the engine consistently preferred `Ka7b6`.

**Root cause:** `KING_GRID` rank 1 (one step forward from back rank) had values `[0,0,0,10,10,5,0,0,5,10,10,0,0,0]` — mildly positive! Combined with equal king safety at a7 vs b6 after pawn pushes (both had 1 shield pawn, same attacker pressure), the total difference was only +5cp — not enough for the engine to avoid the walk.

**Fix (`pst.rs`):**
- Rank 1 changed to `[0,0,0,-5,-5,-10,-15,-15,-10,-5,-5,0,0,0]`
- King one step forward is now clearly penalized (-5 to -15cp by file, worst at center)
- All values stay within ±50cp PST bounds

**Fix (`king_safety.rs`):**
- `PAWN_SHIELD_BONUS`: 35 → 50cp (max 150cp for 3-pawn shield vs 105cp before)
- `OPEN_KING_FILE_PENALTY`: 25 → 40cp
- Makes each pawn shield square count more, amplifying the cost of vacating the shield

### Bug 2 — Hanging Pawn (Blue d9→e9, Yellow Bh12e9)

**Symptom:** Blue pushed an undefended pawn to e9; Yellow's bishop captured it for free with check on the next move. The engine didn't anticipate this.

**Root cause (two layers):**
1. `see(bishop×pawn, 0)` computed `captured_val - attacker_val = 100 - 500 = -400 < 0` → classified as losing capture regardless of whether the pawn was actually defended.
2. Losing captures go last in `order_moves()`. Progressive narrowing at depth 7+ (limit=3) pruned the move before it was ever explored at min nodes.

**Fix (`brs.rs`):**
- `see()` now checks if any opponent attacks `to_sq` via `is_square_attacked_by` before applying the exchange formula.
- If `!is_recapturable`: the capture is free → return `captured_val >= threshold` directly.
- If `is_recapturable`: apply the original `captured_val - attacker_val >= threshold` logic.
- Signature changed: `see(board: &Board, mv: Move, player: Player, threshold: i16) -> bool`
- `order_moves()` updated: takes `board: &Board` and `player: Player` (replacing `player_idx: usize`)

## Test Results

All 387 engine tests pass (246 unit + 141 integration, 3 ignored). No regressions.

## Files Changed

| File | Change |
|------|--------|
| `odin-engine/src/eval/pst.rs` | KING_GRID rank 1: positive → negative |
| `odin-engine/src/eval/king_safety.rs` | PAWN_SHIELD_BONUS 35→50, OPEN_FILE_PENALTY 25→40; test updated |
| `odin-engine/src/search/brs.rs` | `see()` defense check; `order_moves()` takes `board`+`player` |

## Notes

- `see()` is a private free function — changing its signature is an internal impl detail (public `BrsSearcher` trait unchanged).
- The simplified SEE (single exchange, now with defense check) is still a W6 known limitation. Full recursive 4PC SEE deferred to Stage 19.
- KING_GRID PST bounds test (`test_pst_values_bounded`) passes with the new rank 1 values (max absolute value is 15 < 50 bound).
