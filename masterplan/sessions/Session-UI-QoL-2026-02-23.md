---
type: session
tags:
  - type/session
  - scope/ui
date: 2026-02-23
stage: non-stage (UI QoL)
---

# Session: UI QoL Additions (2026-02-23)

**Scope:** Out-of-band UI improvements — not tied to any numbered stage.
**Agent:** Claude Opus 4.6

## What Was Done

### 1. Square Coordinate Labels (Feature 1)
- Added `showCoords` prop to `BoardSquare` — renders algebraic notation (e.g. "e4") in bottom-left corner of each square
- Subtle coloring: dark text on light squares (`rgba(0,0,0,0.35)`), light text on dark squares (`rgba(255,255,255,0.35)`)
- Toggle checkbox ("Coords") added to left panel below game controls
- Files: `BoardSquare.tsx`, `BoardDisplay.tsx`, `App.tsx`, `App.css`

### 2. Prominent NPS Display (Feature 2)
- NPS moved from inline `info-summary` span to large 20px bold display at top of new `AnalysisPanel` component
- Formatted with `toLocaleString()` (e.g. "102,456 NPS")
- Files: `AnalysisPanel.tsx`, `AnalysisPanel.css`

### 3. Enriched Game Log (Feature 3)
- New `GameLog` component showing move history as: `{moveNum}. {Player}: {move} ({eval}cp, d{depth}, {nodes} nodes)`
- Each entry has colored left border matching player (Red=#cc0000, Blue=#0066cc, Yellow=#ccaa00, Green=#00aa44)
- Added `MoveEntry` type and `moveHistory` state to `useGameState.ts`
- Info snapshot captured from `latestInfoRef` at bestmove time and stored alongside each move
- User moves recorded with `info: null`
- Files: `GameLog.tsx`, `GameLog.css`, `useGameState.ts`

### 4. Engine Internals Panel (Feature 4)
- New `EngineInternals` component — collapsible panel showing:
  - Search phase (BRS/MCTS badge)
  - BRS surviving candidates
  - MCTS simulation count
  - Selective depth
  - Per-player values (v1-v4) in a 4-column grid with player colors
- All data from existing `latestInfo` — no new engine protocol needed
- Files: `EngineInternals.tsx`, `EngineInternals.css`

### 5. Communication Log (Feature 5)
- Split DebugConsole into two concerns:
  - `AnalysisPanel` — parsed info display (depth, score, nodes, NPS, PV)
  - `CommunicationLog` — raw protocol log + command input
- CommunicationLog is collapsible, keeps existing color coding (errors=red, bestmove=green, info=gray)
- Original `DebugConsole` component preserved (not deleted) but no longer wired into App
- Files: `CommunicationLog.tsx`, `CommunicationLog.css`, `App.tsx`

### 6. Board Zoom (Feature 6)
- Mouse wheel zoom on the board SVG using CSS `transform: scale()`
- Clamped between 0.5x and 2.0x
- Transform origin tracks mouse position for zoom-toward-cursor behavior
- Container has `overflow: hidden` to clip zoomed content
- **Known issue:** Zoom is somewhat buggy — the board frame boundary occasionally shifts. Marked as polish-later for [[stage_19_polish]] or [[stage_18_full_ui]].
- Files: `BoardDisplay.tsx`, `App.css`

### 7. Layout Reorganization
- Right panel changed from single DebugConsole to vertical stack: Analysis (top, always visible) → Game Log → Engine Internals (collapsible) → Communication Log (collapsible)
- Right panel now scrollable with `overflow-y: auto`
- Center panel given `flex: 2` for larger board area
- Board container sized to `calc(100vh - 70px)` with `overflow: hidden`
- Files: `App.tsx`, `App.css`

## Known Issues / Follow-Up Items

1. **Board zoom is buggy** — frame boundary occasionally shifts on scroll. Low priority — save for polish phase. See [[stage_19_polish]].
2. **Duplicate info across panels** — some data (depth, nodes, NPS) appears in both AnalysisPanel and EngineInternals. Needs dedup pass — AnalysisPanel should own search summary, EngineInternals should own only engine-specific fields (phase, BRS surviving, MCTS sims, per-player values).
3. **No per-player scoring log** — no UI for tracking point changes from captures/eliminations per move. Would need engine to emit explicit scoring events or UI to diff `values` between moves. Future feature for [[stage_18_full_ui]].
4. **Huginn integration opportunity** — engine-side Huginn observation points could provide richer debug data for the Communication Log or a dedicated Huginn panel. Not implemented this session.

## Test Results

- Vitest: 54 tests passing (no regressions)
- TypeScript: clean `tsc --noEmit`

## Files Changed

- `odin-ui/src/components/BoardDisplay.tsx` — zoom, showCoords prop
- `odin-ui/src/components/BoardSquare.tsx` — coordinate labels, showCoords prop
- `odin-ui/src/components/AnalysisPanel.tsx` — **NEW** analysis summary with prominent NPS
- `odin-ui/src/components/GameLog.tsx` — **NEW** enriched move history
- `odin-ui/src/components/EngineInternals.tsx` — **NEW** collapsible engine data
- `odin-ui/src/components/CommunicationLog.tsx` — **NEW** raw protocol log + command input
- `odin-ui/src/hooks/useGameState.ts` — MoveEntry type, moveHistory state, info snapshot capture
- `odin-ui/src/App.tsx` — new layout wiring, coords toggle
- `odin-ui/src/App.css` — right panel stacking, larger board container, overflow hidden
- `odin-ui/src/styles/AnalysisPanel.css` — **NEW**
- `odin-ui/src/styles/GameLog.css` — **NEW**
- `odin-ui/src/styles/EngineInternals.css` — **NEW**
- `odin-ui/src/styles/CommunicationLog.css` — **NEW**

## Related

- [[Component-GameLog]]
- [[Component-EngineInternals]]
- [[Component-CommunicationLog]]
- [[Component-BasicUI]]
