---
type: component
tags:
  - type/component
  - scope/ui
created: 2026-02-23
---

# Component: GameLog

Enriched move history panel showing each move with search info.

## Location

`odin-ui/src/components/GameLog.tsx` + `odin-ui/src/styles/GameLog.css`

## Purpose

Displays a scrollable move history where each entry includes the move number, player, move notation, and search info (eval, depth, nodes) from the engine's last info line before bestmove.

## Data Source

- `moveHistory: MoveEntry[]` from `useGameState.ts`
- Each `MoveEntry` contains `{ move, player, info }` where `info` is an `InfoData` snapshot captured at bestmove time
- User-made moves have `info: null` (no engine search was performed)

## Display Format

```
{moveNum}. {Player}: {move} ({eval}cp, d{depth}, {nodes} nodes)
```

Each entry has a 3px left border colored by player:
- Red: `#cc0000`
- Blue: `#0066cc`
- Yellow: `#ccaa00`
- Green: `#00aa44`

Move numbering: increments every 4 moves (one full round of all players).

## Auto-Scroll

Uses a `scrollRef` to auto-scroll to the bottom when new moves arrive.

## Related

- [[Component-BasicUI]] — parent UI shell
- [[Session-UI-QoL-2026-02-23]] — creation session
- `useGameState.ts` — `MoveEntry` type and `moveHistory` state
