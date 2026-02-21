---
type: pattern
stage_introduced: 5
tags:
  - stage/05
  - area/ui
last_updated: 2026-02-20
---

# Pattern: React Ref-Based Async State

## Problem

In React 18 with automatic batching, when multiple `setState` calls occur in the same event handler or callback, React may defer running state updater functions until the next render. This means:

```typescript
// BROKEN: nextPlayer may not be computed when you need it
let nextPlayer: Player = 'Red'; // default
setCurrentPlayer((prev) => {
  nextPlayer = PLAYERS[(PLAYERS.indexOf(prev) + 1) % 4];
  return nextPlayer;
});
// nextPlayer is still 'Red' here if the updater was deferred!
```

This is especially dangerous in auto-play chains where setTimeout callbacks need the current value immediately to decide what to do next.

## Solution

Maintain a `useRef` mirror of any React state that needs to be read synchronously. Update the ref **before** calling setState:

```typescript
const currentPlayerRef = useRef<Player>('Red');

const advancePlayer = useCallback((): Player => {
  const idx = PLAYERS.indexOf(currentPlayerRef.current);
  const nextPlayer = PLAYERS[(idx + 1) % 4];
  currentPlayerRef.current = nextPlayer;  // Ref updated immediately
  setCurrentPlayer(nextPlayer);            // React state for re-render
  return nextPlayer;                       // Return value is reliable
}, []);
```

## When to Use

- Any value that is read synchronously after being written in the same handler
- Values used in setTimeout/setInterval chains where closures capture stale state
- Values used in conditional logic that determines what action to take next (e.g., "should the engine play this turn?")

## Applied In

- `useGameState.ts`: `currentPlayerRef`, `playModeRef`, `humanPlayerRef`, `engineDelayRef`
- `advancePlayer()`, `shouldEnginePlay()`, `maybeChainEngineMove()`, `sendGoFromRef()`

## Related

- [[Component-BasicUI]]
- [[Issue-UI-AdvancePlayer-React-Batching]]
