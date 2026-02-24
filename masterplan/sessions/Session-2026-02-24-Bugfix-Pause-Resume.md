---
type: session
tags:
  - type/session
  - stage/ui
date: 2026-02-24
---

# Session: 2026-02-24 — Pause/Resume Race Condition Fix

## Context

User was testing Stage 8 (BRS/Paranoid Hybrid) in the UI's full-auto mode. They paused the game mid-play, read the board for a moment, then resumed. Blue received two consecutive turns (moves g2g4 and k2j4), followed by engine errors (`illegal or unrecognized move: k2j4`, `no position set`). The game state was corrupted and could not recover.

## What Was Done

### Bug Investigation

1. Analyzed the user's game log and communication log — confirmed Blue moved twice at move 7
2. Read `odin-engine/src/protocol/mod.rs` — confirmed engine-side logic is correct (single-threaded, sequential command processing, `nextturn` emitted from post-move state)
3. Read `odin-ui/src/hooks/useGameState.ts` — identified the race condition between `togglePause` resume path and `bestmove` handler's `maybeChainEngineMove` path
4. Traced the exact sequence: both paths schedule `sendGoFromRef()` via `setTimeout`, and neither checked `awaitingBestmoveRef` before sending

### Fix Applied

Two guards in `useGameState.ts`:
- **`sendGoFromRef` (line 199):** `if (awaitingBestmoveRef.current) return;` — prevents any caller from sending duplicate `go` commands
- **`togglePause` (line 425):** `if (!awaitingBestmoveRef.current)` — skips scheduling the timeout entirely if a search is in flight; just sets `autoPlayRef = true` and lets the bestmove handler chain naturally

### Verification

- 54 Vitest tests pass
- 361 engine tests pass (233 unit + 128 integration, 3 ignored)
- User ran a 21-move game with pause/resume — no duplicate turns observed

## Key Decisions

- **No `stop` command on pause:** The engine search continues to completion when paused. This is acceptable — the search is fast (depth 6 at most) and the bestmove is still useful. The guard prevents the race condition without needing to cancel searches.
- **Defense-in-depth:** Both `sendGoFromRef` and `togglePause` have guards. The `sendGoFromRef` guard is the primary protection; the `togglePause` guard avoids scheduling an unnecessary timeout.

## Files Modified

- `odin-ui/src/hooks/useGameState.ts` — two guard additions (lines 199, 425)

## Vault Notes Created

- [[Issue-UI-Pause-Resume-Race-Condition]] — full bug diagnosis and fix
- [[Session-2026-02-24-Bugfix-Pause-Resume]] — this file
