---
type: session
date: 2026-02-20
stage: 5
tags:
  - stage/05
  - tier/foundation
---

# Session: 2026-02-20 -- Stage 05 Bugfix & Play Modes

## Goal

Fix bugs found during live `tauri dev` testing and add play mode features (Manual, Semi-Auto, Full Auto) with speed control and pause.

## What Happened

1. Launched `tauri dev` for the first time — app loaded successfully.
2. Discovered Blue/Green pawns vanished when moved. Root cause: en passant false positive for non-Red/Yellow orientations. Fixed by requiring both file AND rank to change for diagonal detection.
3. Discovered castling display broken for Blue/Green. Root cause: castling detection only checked file distance, but Blue/Green castle by rank. Fixed with orientation-aware detection.
4. Discovered board clipped at bottom (Red's side cut off). Fixed by removing fixed SVG dimensions and adding CSS responsive sizing.
5. User requested play modes. Implemented three modes:
   - **Manual:** Click-to-move only. Engine Move button for engine moves.
   - **Semi-Auto:** User picks a color, engine auto-plays the other three.
   - **Full Auto:** Engine plays all four colors.
6. Added speed control slider (100-2000ms delay) and pause/resume button.
7. Discovered semi-auto mode completely broken. Root cause: `advancePlayer` used React state updater which doesn't execute synchronously under React 18 batching. Fixed by computing from ref directly.
8. User reported player switching mid-game caused engine to play user's turns. Fixed by locking player selector when game is in progress (moveList non-empty).
9. Traced semi-auto logic through all 4 player colors (Red, Blue, Yellow, Green) to verify correctness.
10. User confirmed all modes working: "awesome work!"

## Components Touched

- [[Component-BasicUI]] — major changes to useGameState.ts, GameControls.tsx, BoardDisplay.tsx, App.css, App.tsx, GameControls.css

## Files Modified

- `odin-ui/src/hooks/useGameState.ts` — En passant fix, castling fix, play modes, advancePlayer ref fix, gameInProgress
- `odin-ui/src/components/BoardDisplay.tsx` — Removed fixed SVG dimensions
- `odin-ui/src/App.css` — Responsive board sizing
- `odin-ui/src/components/GameControls.tsx` — Mode selector, player picker, speed slider, pause button
- `odin-ui/src/styles/GameControls.css` — Styles for new controls
- `odin-ui/src/App.tsx` — Wired new props

## Discoveries

1. **En passant detection in 4PC must check both axes.** Blue/Green pawns move forward by changing file, so `fileOf(from) !== fileOf(to)` is true for every forward move. Only a true diagonal (both file AND rank change) indicates en passant.
2. **React 18 automatic batching breaks synchronous state reads.** When multiple setState calls are batched in the same handler, updater functions may not execute immediately. Refs must be used for any value needed synchronously after a setState call.
3. **4-player castling is orientation-dependent.** Red/Yellow castle horizontally (file). Blue/Green castle vertically (rank). Both the detection and the rook placement must branch on player orientation.

## Issues Created/Resolved

- Created and resolved: [[Issue-UI-EP-False-Positive]] — en passant false positive for Blue/Green
- Created and resolved: [[Issue-UI-Castling-Blue-Green]] — castling display broken for Blue/Green
- Created and resolved: [[Issue-UI-AdvancePlayer-React-Batching]] — advancePlayer React 18 batching bug
- Existing issues unchanged: [[Issue-Perft-Values-Unverified]], [[Issue-DKW-Halfmove-Clock]], [[Issue-DKW-Invisible-Moves-UI]]

## Patterns Documented

- [[Pattern-React-Ref-Async-State]] — Use refs alongside React state for values needed synchronously in async chains
