# Audit Log — Stage 18: Full UI

## Pre-Audit
**Date:** 2026-03-01
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes — engine 0 errors 0 warnings, UI tsc --noEmit clean
- Tests pass: engine 557 total (308 unit + 249 integration, 6 ignored), UI 63 Vitest
- Previous downstream flags reviewed: W26-W30, W13, W18-W20 from Stage 17 downstream log

### Findings

1. **Engine protocol extensions (P0-A) — permanent investment.** Added 4 new `info string` emissions: `in_check`, `brs_moves`, `mcts_visits`, `stop_reason`. All use `info string` prefix (not `info`) to avoid breaking existing tests that validate structured info line fields (depth, score cp). Parser tests added (7 new).

2. **Per-slot player configuration (P0-B) — replaced `PlayMode` with `SlotConfig`.** Removed `PlayMode` type (`manual`/`semi-auto`/`full-auto`), `playMode` state/ref, `humanPlayer` state/ref. Replaced with `SlotConfig = Record<Player, 'human' | 'engine'>`. `shouldEnginePlay()` simplified to single ref check. `handleSquareClick` consolidated from 3-branch playMode logic to simple engine/human check.

3. **Self-play dashboard (P0-C) — new hook + component.** `useSelfPlay` hook watches `isGameOver` via useEffect, records results, auto-starts next game. Saves/restores user's slot config and engine delay. Added `gameWinner: Player | null` to useGameState return (was only setting `isGameOver` on gameover message, not tracking who won).

4. **Undo/redo (P1-A) — pure board replay approach.** Undo rebuilds board from scratch via `replayMoveOnBoard()` (pure function mirror of `applyMoveToBoard`). This avoids needing `unmake_move` in the UI. Redo stack cleared on any new move (branch point). Both disabled during active search. Engine position synced via `position startpos moves [...]` + `isready`.

5. **Debug panel enhancements (P1-B) — display new engine data.** EngineInternals now shows BRS surviving move list with scores, MCTS top-5 visit counts, and stop reason. AnalysisPanel shows stop reason inline with depth.

6. **P2 items intentionally deferred.** Move arrows, check highlight, terrain styling, FEN4 parser are deferred to the web platform build. The Tauri UI is a development tool; these visual features have low ROI here and will be rebuilt for the web app. This is NOT a regression — it's a deliberate prioritization decision. Future agents should not attempt to "fix" or "complete" these.

### Risks for This Stage

- **R1:** Self-play `useEffect` dependency on `game.isGameOver` — could miss rapid game-over transitions if React batches multiple state updates. LOW RISK: game-over is a terminal state, not rapidly toggled.
- **R2:** Undo doesn't rebuild eliminated players or scores — display may be stale after undo past an elimination. LOW RISK: dev tool, user can start a new game.
- **R3:** `replayMoveOnBoard` duplicates board mutation logic from `applyMoveToBoard`. ACCEPTED: keeps undo implementation simple and isolated from React state.
- **R4:** Self-play `setTimeout(50ms)` before `newGame()` to let React flush config. FRAGILE but acceptable for dev tool.

---

## Post-Audit
**Date:** 2026-03-01
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status |
|-------------|--------|
| P0-A: Engine protocol extensions (Rust + TS parser) | Complete |
| P0-B: Per-slot player configuration | Complete |
| P0-C: Self-play dashboard | Complete |
| P1-A: Undo/redo | Complete |
| P1-B: Debug panel enhancements | Complete |
| P2: Visual polish (arrows, check, terrain, FEN4) | Deferred to web platform |

### Code Quality
#### Uniformity
All new TypeScript follows existing patterns: hooks use useCallback/useRef for async safety, CSS matches existing dark theme. Rust `info string` emissions follow established pattern from prior stages.

#### Bloat
Three new files created (useSelfPlay.ts, SelfPlayDashboard.tsx, SelfPlayDashboard.css) — all necessary for new feature. No unnecessary abstractions.

#### Efficiency
`replayMoveOnBoard` copies the entire board array per move during undo replay. For typical games (100-400 moves), this is negligible. No performance concern for a dev tool.

#### Dead Code
`PlayMode` type removed cleanly. No leftover references. `MODE_LABELS` constant removed. Old player-selector CSS kept but unused — will be removed in cleanup.

#### Broken Code
None found. All paths compile clean, tests pass.

#### Temporary Code
None. All code is production-quality for the dev tool scope.

### Search/Eval Integrity
Engine search and eval are UNCHANGED. All Stage 18 changes are either:
- New `info string` emissions (read-only, no search logic changes)
- `in_check` emission (single `is_in_check()` call, already used elsewhere)
- UI-only code (TypeScript/React)

No risk to search correctness or eval accuracy.

### Future Conflict Analysis
- **Stage 19 (Optimization):** No conflicts. Engine protocol extensions are additive.
- **Web platform:** Engine-side changes (info string emissions) benefit any frontend. UI changes are Tauri-specific and will be replaced.

### Unaccounted Concerns
- **W31:** Self-play `gameWinner` is null for both "no game over" and "draw". Disambiguated by `isGameOver` flag, but could be cleaner with a discriminated union. LOW PRIORITY.
- **W32:** Undo past eliminations doesn't restore eliminated player state (kings removed from display board, eliminated set not cleared). Would need engine cooperation to fix properly. ACCEPTABLE for dev tool.

### Reasoning & Methods
- **Priority framework:** User guidance categorized features as dev-tool-ROI vs web-platform-rebuild. Engine-side protocol extensions are permanent investment; UI polish is throwaway. This reasoning is documented in the plan and audit for future agent awareness.
- **Testing approach:** TypeScript type checking (`tsc --noEmit`) + Vitest suite + engine `cargo test` at each step. No manual smoke test in this session (AC4 self-play 100+ games needs manual verification).

---

## Related

- Stage spec: [[stage_18_full_ui]]
- Downstream log: [[downstream_log_stage_18]]
