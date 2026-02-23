---
type: issue
date_opened: 2026-02-22
last_updated: 2026-02-22
date_resolved: 2026-02-22
stage: 7
severity: warning
status: resolved
tags: [ui, promotion, bugfix]
---

# Pawn Promotion: Wrong Ranks + No Selection UI

## Description

Two related bugs preventing pawn promotion from working in the UI:

**Bug 1 (Critical): Wrong promotion rank/file constants.** The `handleSquareClick` function in `useGameState.ts` checked board edges (ranks 0/13, files 0/13) to detect promotion, but the engine's actual promotion coordinates are rank 8 (Red), rank 5 (Yellow), file 8 (Blue), file 5 (Green). In 4-player chess, pawns promote at the midline, not the opposite edge. Because the check never matched, the promotion suffix was never appended to the move string. The engine received e.g. `"d7d8"` instead of `"d7d8q"` and rejected it as illegal (the only legal moves at that point are `d7d8q`, `d7d8n`, `d7d8r`, `d7d8b`).

**Bug 2 (UX): No promotion piece selection UI.** Even if Bug 1 were fixed alone, the code would silently auto-promote to queen. No dialog existed for the user to choose between queen, rook, bishop, or knight.

Engine-side promotions (bestmove output) were unaffected — the engine correctly includes the suffix and the UI parser/display handles it.

## Affected Components

- [[Component-BasicUI]] — `useGameState.ts` promotion detection, `App.tsx` component wiring
- Engine promotion coordinates defined in `odin-engine/src/movegen/generate.rs:33-41` (PAWN_CONFIG)

## Workaround

None prior to fix. Promotion moves were rejected by the engine.

## Resolution

Fixed (user-tested 2026-02-22):

1. **Fixed promotion rank/file constants** in `useGameState.ts:282-286` — Red=8, Yellow=5, Blue=file 8, Green=file 5 (matching engine PAWN_CONFIG).
2. **Added `pendingPromotion` state** to `useGameState` — when a pawn reaches a promotion rank, the move is deferred until the user selects a piece.
3. **Created `PromotionDialog` component** — overlay showing 4 piece options (PromotedQueen/R/B/N) in the player's color. Escape or click-outside cancels.
4. **Wired into `App.tsx`** — dialog renders over the board when `pendingPromotion` is set.
5. **Used `w` (PromotedQueen) suffix, not `q` (Queen)** — the engine generates `PieceType::PromotedQueen` for pawn promotions, which has FEN char `W` (lowercase `w`). Initial attempt with `q` was rejected by the engine as illegal.

Files modified:
- `odin-ui/src/hooks/useGameState.ts` — promotion detection fix, state, callbacks
- `odin-ui/src/components/PromotionDialog.tsx` — new component
- `odin-ui/src/components/PromotionDialog.css` — new styles
- `odin-ui/src/App.tsx` — wiring
- `odin-ui/src/App.css` — `position: relative` on center-panel for overlay positioning

## Related

- [[stage_07_plain_brs]]
- [[downstream_log_stage_07]]
