# Issue: UI Stale Engine Output

**Status:** RESOLVED (2026-03-01)
**Severity:** High (corrupts game state)
**Session:** [[Session-2026-03-01-UI-Stale-Output-Fix]]

## Symptom

After clicking New Game (or restarting the engine), the game log shows wrong player labels, missing search info, and/or illegal move errors like `info string Error: illegal or unrecognized move: g13g11`. The old engine's bestmove from the previous game gets processed as the new game's first move.

## Root Cause

Two separate paths:

1. **Engine restart:** The old engine's reader thread (`thread::spawn` in `engine.rs`) continues emitting `engine-output` Tauri events after the process is killed. The `BufReader` drains buffered stdout. Both old and new reader threads emit to the same event channel with no way to distinguish them.

2. **New Game (same process):** `newGame()` resets UI state but the engine is still computing from the previous `go`. The stale bestmove arrives after state reset and gets processed as Move 1 of the new game.

## Fix

### Path 1: Rust-level generation tagging

- `EngineManager.generation: u64` — bumped on every `spawn()`
- `EngineOutputPayload { line, gen }` — every event tagged with the generation
- `useEngine.ts` — discards events where `gen !== engineGenRef.current`

### Path 2: JS-level ignore flag

- `ignoreNextBestmoveRef` — set when `newGame()` detects in-flight search
- `newGame()` sends `stop` to cancel the search
- `info`/`nextturn`/`bestmove` handlers check flag before processing
- Flag consumed when stale bestmove arrives

## Failed Approach

A JS-only generation counter (`gameGenRef`/`awaitingGenRef`) was attempted but failed: the stale bestmove arrived after `sendGoFromRef` set `awaitingGenRef = gameGenRef`, so the guard `awaitingGenRef !== gameGenRef` passed and the stale move was processed.

## Files Modified

- `odin-ui/src-tauri/src/engine.rs`
- `odin-ui/src-tauri/src/lib.rs`
- `odin-ui/src/hooks/useEngine.ts`
- `odin-ui/src/hooks/useGameState.ts`
