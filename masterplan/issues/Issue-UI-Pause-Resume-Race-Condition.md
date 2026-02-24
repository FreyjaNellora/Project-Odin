---
type: issue
tags:
  - type/issue
  - stage/ui
  - status/resolved
created: 2026-02-24
resolved: 2026-02-24
---

# Issue: UI Pause/Resume Race Condition (Duplicate `go` Commands)

## Symptom

When the user pauses the game during auto-play and then resumes, one player can receive two consecutive turns. The game log shows the same color moving twice (e.g., `7. Blue: g2g4` followed by `7. Blue: k2j4`), and subsequent engine errors appear: `Error: illegal or unrecognized move: k2j4` and `Error: no position set, send 'position' first`.

## Root Cause

Two code paths could fire `sendGoFromRef()` simultaneously after resume:

1. **Resume handler** (`togglePause`): On unpause, schedules `sendGoFromRef()` via `setTimeout`.
2. **Bestmove handler** (`handleEngineMessage` → `bestmove` case): When the in-flight search's `bestmove` arrives after resume, `maybeChainEngineMove()` schedules another `sendGoFromRef()`.

Neither path checked whether a search was already in progress (`awaitingBestmoveRef`). Both timeouts fired, both sent `position + go`, and the engine processed two searches for the same player position. The second `bestmove` was erroneously added to the move list, corrupting the game state.

### Trigger Conditions

- Auto-play (full-auto or semi-auto) is active
- User pauses while the engine is mid-search (search is blocking, no `stop` command sent on pause)
- User resumes — the in-flight search completes and its `bestmove` arrives around the same time as the resume timeout fires
- The user does NOT need to toggle quickly; the engine's IPC bestmove delivery and the resume click event simply need to land close together in the JS event loop

## Fix

Two guards added to `odin-ui/src/hooks/useGameState.ts`:

### 1. `sendGoFromRef` — early bail if search in flight (line 199)

```typescript
const sendGoFromRef = useCallback(() => {
  if (awaitingBestmoveRef.current) return;  // <-- NEW
  // ... rest unchanged
}, [sendCommand]);
```

### 2. `togglePause` — skip scheduling if search in flight (line 425)

```typescript
if (shouldEnginePlay(player)) {
  autoPlayRef.current = true;
  if (!awaitingBestmoveRef.current) {  // <-- NEW
    setTimeout(() => {
      if (autoPlayRef.current) {
        sendGoFromRef();
      }
    }, engineDelayRef.current);
  }
}
```

The second guard is defense-in-depth. If a search is in flight when the user resumes, `autoPlayRef` is set to `true` and the natural bestmove handler chains the next move via `maybeChainEngineMove`.

## Verification

- 54 Vitest tests pass (no regressions)
- 361 engine tests pass
- Manual testing: ran 21-move game with multiple pause/resume cycles, no duplicate turns observed

## Related

- [[Pattern-React-Ref-Async-State]] — the ref-mirroring pattern that makes this guard work in async callbacks
- [[Issue-UI-AdvancePlayer-React-Batching]] — previous async timing bug in the same hook
- [[Component-BasicUI]] — UI architecture
