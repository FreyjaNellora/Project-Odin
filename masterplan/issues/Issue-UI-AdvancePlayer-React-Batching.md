---
type: issue
date_opened: 2026-02-20
last_updated: 2026-02-20
date_resolved: 2026-02-20
stage: 5
severity: blocking
status: resolved
tags:
  - stage/05
  - severity/blocking
  - area/ui
---

# Issue: advancePlayer Returns Wrong Player Due to React 18 Batching

## Description

`advancePlayer` in useGameState.ts computed the next player inside a `setCurrentPlayer` updater function. With React 18 automatic batching, when multiple setState calls are pending in the same handler (e.g., the readyok handler calls `setMoveList`, `setBoard`, `setLastMoveFrom`, `setLastMoveTo` via `applyMoveToBoard`), React may skip eager state computation for subsequent setState calls. The updater function runs during render instead of immediately, so the `nextPlayer` variable stays at its default value `'Red'`.

Every downstream auto-play decision — `shouldEnginePlay(nextPlayer)`, `maybeChainEngineMove(nextPlayer)` — used the wrong player, causing semi-auto and full-auto modes to completely fail.

## Affected Components

- [[Component-BasicUI]] — useGameState.ts `advancePlayer` function
- All play mode logic that depends on knowing the current player after a turn advance

## Workaround

None needed after fix. The core issue was relying on React's state updater for synchronous computation.

## Resolution

Compute next player directly from `currentPlayerRef.current` (the ref) instead of depending on React's updater timing:

```typescript
const advancePlayer = useCallback((): Player => {
  const idx = PLAYERS.indexOf(currentPlayerRef.current);
  const nextPlayer = PLAYERS[(idx + 1) % 4];
  currentPlayerRef.current = nextPlayer;
  setCurrentPlayer(nextPlayer);
  return nextPlayer;
}, []);
```

Pattern documented in [[Pattern-React-Ref-Async-State]].

## Related

- [[audit_log_stage_05]] — Post-Audit Addendum
- [[Session-2026-02-20-Stage05-Bugfix]]
- [[Component-BasicUI]]
- [[Pattern-React-Ref-Async-State]]
