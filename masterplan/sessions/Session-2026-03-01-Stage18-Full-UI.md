# Session — 2026-03-01 — Stage 18: Full UI

## Summary

Implemented Stage 18 (Full UI) for the Tauri dev tool. Prioritized engine-side protocol extensions (permanent investment) and high-ROI dev tool features (self-play dashboard, per-slot config, undo/redo). Visual polish (move arrows, check highlight, terrain styling, FEN4 parser) deferred to web platform build per user guidance.

## Priority Framework

User established that the Tauri UI is a **dev tool** — the real product is a future web platform. This drives all prioritization:

| Priority | Items | Rationale |
|----------|-------|-----------|
| P0 | Engine protocol extensions | Permanent investment, benefits web app too |
| P0 | Per-slot player config | Core testing functionality |
| P0 | Self-play dashboard | Best ROI for training evaluation |
| P1 | Undo/redo | Useful for debugging positions |
| P1 | Debug panel enhancements | Show new engine data |
| P2 (deferred) | Move arrows, check highlight, terrain, FEN4 | Rebuild for web platform |

## Changes Made

### P0-A: Engine Protocol Extensions (Rust + TypeScript)

**Engine (Rust):**
- `hybrid.rs` — Added `brs_moves` emission (surviving moves with scores after BRS phase 1), `stop_reason` emission at 5 return paths (forced, time_pressure, brs_confidence, time, depth, nodes, complete)
- `mcts.rs` — Added `mcts_visits` emission (top-5 root children by visit count before bestmove)
- `protocol/mod.rs` — Added `in_check` emission (after position set, checks if next-to-move is in check)

**UI (TypeScript):**
- `protocol.ts` — Extended `InfoData` with `brsMoves`, `mctsVisits`, `stopReason`. Added `in_check` message type.
- `protocol-parser.ts` — 4 new `info string` parsers
- `protocol-parser.test.ts` — 7 new tests (63 total, was 56)

### P0-B: Per-Slot Player Configuration

- `useGameState.ts` — Replaced `PlayMode` + `humanPlayer` with `SlotConfig = Record<Player, 'human' | 'engine'>`. Simplified `shouldEnginePlay()` and `handleSquareClick()`.
- `GameControls.tsx` — Replaced play mode selector with 4-row slot toggle panel + presets (Play as Red / Watch / Hot Seat)
- `App.tsx` — Updated prop passing

### P0-C: Self-Play Dashboard

- `useSelfPlay.ts` — New hook: runs batches of all-engine games, collects win rates, avg game length, avg duration
- `SelfPlayDashboard.tsx` — New component: config (games, speed), progress bar, win rate bars, averages
- `SelfPlayDashboard.css` — Styling
- `useGameState.ts` — Added `gameWinner: Player | null` state for self-play to track results

### P1-A: Undo/Redo

- `useGameState.ts` — Added `redoMovesRef`/`redoHistoryRef`, `undo()`/`redo()` callbacks, `canUndo`/`canRedo` derived state, `replayMoveOnBoard()` pure helper. Redo stack cleared on new moves.
- `GameControls.tsx` — Undo/Redo buttons
- `GameControls.css` — Button styling

### P1-B: Debug Panel Enhancements

- `EngineInternals.tsx` — BRS surviving moves with scores, MCTS top-5 visit counts, stop reason
- `AnalysisPanel.tsx` — Stop reason inline with depth display
- `EngineInternals.css` — Move list chip styling

## Test Results

- Engine: 557 tests (308 unit + 249 integration, 6 ignored) — all passing
- UI: 63 Vitest tests (29 board-constants + 34 protocol-parser) — all passing
- TypeScript: compiles clean (`tsc --noEmit`)

## Files Modified/Created

### Engine (Rust)
- `odin-engine/src/search/hybrid.rs` — MODIFIED
- `odin-engine/src/search/mcts.rs` — MODIFIED
- `odin-engine/src/protocol/mod.rs` — MODIFIED
- `odin-engine/tests/stage_10_mcts.rs` — MODIFIED (test fix for info string lines)

### UI (TypeScript/React)
- `odin-ui/src/types/protocol.ts` — MODIFIED
- `odin-ui/src/lib/protocol-parser.ts` — MODIFIED
- `odin-ui/src/lib/protocol-parser.test.ts` — MODIFIED
- `odin-ui/src/hooks/useGameState.ts` — MODIFIED (major: slot config, undo/redo, gameWinner)
- `odin-ui/src/hooks/useSelfPlay.ts` — CREATED
- `odin-ui/src/components/GameControls.tsx` — MODIFIED
- `odin-ui/src/components/SelfPlayDashboard.tsx` — CREATED
- `odin-ui/src/components/EngineInternals.tsx` — MODIFIED
- `odin-ui/src/components/AnalysisPanel.tsx` — MODIFIED
- `odin-ui/src/App.tsx` — MODIFIED
- `odin-ui/src/styles/GameControls.css` — MODIFIED
- `odin-ui/src/styles/SelfPlayDashboard.css` — CREATED
- `odin-ui/src/styles/EngineInternals.css` — MODIFIED

### Documentation
- `masterplan/audit_log_stage_18.md` — WRITTEN
- `masterplan/downstream_log_stage_18.md` — WRITTEN
- `masterplan/sessions/Session-2026-03-01-Stage18-Full-UI.md` — CREATED

## Known Issues

- W31: `gameWinner` null ambiguity (no game over vs draw)
- W32: Undo past eliminations doesn't restore eliminated state
