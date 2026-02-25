---
type: issue
date_opened: 2026-02-25
last_updated: 2026-02-25
date_resolved:
stage: "Post-Stage 8 (debugging)"
severity: warning
status: pending-verification
tags:
  - area/ui
  - stage/08
---

# Issue: Game Log Player Labels Shifted by One (React 18 Batching)

## Description

Every move in the game log was attributed to the **next** player rather than the player who actually made the move. For a full-auto game, this meant all four labels cycled one position clockwise:

- Moves shown as "Blue" were actually **Red**'s moves
- Moves shown as "Yellow" were actually **Blue**'s
- Moves shown as "Green" were actually **Yellow**'s
- Moves shown as "Red" were actually **Green**'s

This caused significant confusion during testing — e.g. "why is Red exposing its king?" was actually asking about Green's moves.

## Root Cause

In `useGameState.ts`, both the `bestmove` and `readyok` handlers passed `currentPlayerRef.current` and `boardRef.current` **inside a React functional updater** passed to `setMoveHistory`:

```typescript
setMoveHistory((prev) => [
  ...prev,
  { move: formatMoveForDisplay(engineMove, boardRef.current), player: currentPlayerRef.current, info: infoSnapshot },
]);
applyMoveToBoard(engineMove);
// ... currentPlayerRef.current updated here to the NEXT player
```

**React 18 automatic batching** defers all functional updaters until the next render flush. By the time the `setMoveHistory` updater executes, `currentPlayerRef.current` has already been mutated to the next player (via `pendingNextTurnRef` assignment in the `bestmove` case, or `advancePlayer()` in the `readyok` case). The updater therefore reads the wrong value.

The same pattern applied to `boardRef.current` — it was read lazily inside the updater rather than at call time.

This is the same class of bug as [[Issue-UI-AdvancePlayer-React-Batching]] (Stage 5), which fixed `advancePlayer` returning the wrong player. The lesson was not generalized when `setMoveHistory` was added in the UI QoL session.

## Affected Components

- `odin-ui/src/hooks/useGameState.ts` — `handleEngineMessage` function, `bestmove` and `readyok` cases

## Workaround

None — the labels were always wrong in full-auto play.

## Resolution

Fixed in commit `b98c087`. Snapshot both refs as local variables **before** calling `setMoveHistory` in both handlers:

```typescript
const movingPlayer = currentPlayerRef.current;
const movingBoard = boardRef.current.slice();
setMoveHistory((prev) => [
  ...prev,
  { move: formatMoveForDisplay(engineMove, movingBoard), player: movingPlayer, info: infoSnapshot },
]);
applyMoveToBoard(engineMove);
// refs updated after — no longer affects the already-captured locals
```

Status: `pending-verification` — awaiting user confirmation through fresh game playthroughs.

## Related

- [[Issue-UI-AdvancePlayer-React-Batching]] — same root cause class (React 18 batching + mutable ref read inside deferred updater)
- [[Session-2026-02-25-UI-Bugfixes]] — session where this was discovered and fixed
