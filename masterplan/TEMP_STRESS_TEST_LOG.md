# TEMPORARY — Stress Test & Fuzz Investigation Log

> **This file is temporary.** Delete after user gives final approval on Phase 5-7 completion.
> Referenced from: `masterplan/HANDOFF.md`

---

## Batch 1, Sitting 1 (Attempt 1) — 2026-03-01

**Config:** 500 games, depth 2, 100 MCTS sims, FFA Standard, 200 ply cap
**Binary:** release build (pre-EP-fix)
**Result:** FAILED — only 1 game completed, Game 2 hung indefinitely

### Issue: Engine silent death → match.mjs hang
- **Symptom:** Game 2 never completed. Only 1 engine process visible in task manager (should be 2).
- **Root cause:** `Engine` class in `observer/lib/engine.mjs` had no stderr capture and no dead-process detection. When an engine process died, `readLine()` hung forever on a closed stdout pipe.
- **Fix:** Added `#dead` flag, `#stderrBuf`, process `exit` event handler, null return from `readLine()`, and null check in `match.mjs` game loop. See `observer/lib/engine.mjs` and `observer/match.mjs`.

---

## Batch 1, Sitting 1 (Attempt 2) — 2026-03-01

**Config:** Same as attempt 1, with engine crash detection fix
**Binary:** release build (pre-EP-fix, with observer fixes)
**Result:** 21 games completed before manual stop for investigation. **2 crashes found.**

### Bug: `remove_piece: square is empty` (EP after elimination)

**Crash rate:** 2/21 games (~10%)
- Game 10: crash at ply 55, Blue eliminated at ply 48
- Game 20: crash at ply 35, Blue eliminated at ply 32

**Backtrace (debug binary):**
```
remove_piece (board_struct.rs:425)
  → make_move (moves.rs, EP branch)
  → generate_legal (generate.rs)
  → determine_status_at_turn (gamestate/mod.rs)
  → check_elimination_chain (gamestate/mod.rs)
  → apply_move (mcts.rs, expand_node)
  → expand_node (mcts.rs)
  → run_simulation (mcts.rs)
  → MctsSearcher::search (mcts.rs)
  → HybridController::search (hybrid.rs)
```

**Root cause:** En passant `make_move` used `player.prev()` to find the pushing player. In 4-player chess, after a player elimination, `.prev()` returns the eliminated player (who has no pawn on the expected square), not the actual pusher who may be 2 or 3 turns back.

**Fix (odin-engine/src/movegen/moves.rs):**
1. New `find_ep_captured_pawn_sq()` function: scans all 4 directions from EP target to find the actual pawn on the board
2. `make_move` EP branch: uses `find_ep_captured_pawn_sq()` instead of `player.prev()`
3. `unmake_move` EP branch: uses `undo.captured_piece.owner` (the real pusher) instead of `player.prev()`

**Verification:**
- 20/20 replay runs of crashing Game 20 position: zero crashes
- All 573 tests pass (316 unit + 257 integration, 6 ignored)
- +6 new tests from fix verification

---

## Batch 1, Sitting 1 (Attempt 3) — 2026-03-02

**Config:** 500 games, depth 2, 100 MCTS sims, FFA Standard, 200 ply cap
**Binary:** release build WITH EP fix
**Status:** PENDING — needs release build then launch

---

## Summary of Fixes Applied

| Issue | File(s) | Status |
|-------|---------|--------|
| Engine silent death → hang | `observer/lib/engine.mjs`, `observer/match.mjs` | FIXED |
| EP crash after elimination | `odin-engine/src/movegen/moves.rs` | FIXED, verified |

## Test Count Progression

| Point | Tests |
|-------|-------|
| Start of Phase 5 | 567 (316u + 251i, 6 ignored) |
| After EP fix | 573 (316u + 257i, 6 ignored) |
