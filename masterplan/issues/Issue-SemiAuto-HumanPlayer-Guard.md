# Issue: Semi-Auto Engine Takes Over Human's Turn

**Date:** 2026-02-21
**Stage:** 7 (post-completion regression)
**Status:** RESOLVED

## Symptom

In semi-auto play mode with no human player selected, the engine played all four players' turns indefinitely. The "Play as" color selector buttons were disabled once a game started, so the user could not correct the situation mid-game.

## Root Cause

`shouldEnginePlay` in `odin-ui/src/hooks/useGameState.ts`:

```typescript
if (playModeRef.current === 'semi-auto' && humanPlayerRef.current !== player) return true;
```

When `humanPlayerRef.current = null` (no player selected), `null !== player` evaluates to `true` for every player — including what should be the human's turn. The engine auto-played all four colors.

Secondary: `disabled={gameInProgress}` on the player selector buttons prevented the user from correcting the selection mid-game.

## Fix

Three changes:

1. **`odin-ui/src/hooks/useGameState.ts` — `shouldEnginePlay`**: Added `humanPlayerRef.current !== null` guard. Without a player selected, semi-auto behaves like manual.

2. **`odin-ui/src/hooks/useGameState.ts` — `setHumanPlayer`**: Added `autoPlayRef.current = false` so changing the human player mid-game immediately stops any in-flight engine chain.

3. **`odin-ui/src/components/GameControls.tsx`**: Removed `disabled={gameInProgress}` from player selector buttons. Players can now be (re)selected at any time.

## Verification

`npm test` — 45/45 passing after fix.
