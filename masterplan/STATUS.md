# PROJECT ODIN — STATUS

**Last Updated:** 2026-02-25
**Updated By:** Claude Sonnet 4.6 (Post-Stage 8 bugfixes: repetition detection, piece notation, player label fix)

---

## Current State

| Field | Value |
|-------|-------|
| **Current Stage** | Stage 8 implementation complete — awaiting user testing before tagging |
| **Current Build-Order Step** | All 10 steps done (0, 0b, 1-9) |
| **Build Compiles** | Yes — `cargo build --release` passes, 0 warnings |
| **Tests Pass** | Yes — engine: 233 unit + 128 integration = 361 total (3 ignored); UI: 54 Vitest |
| **Blocking Issues** | None |

---

## Stage Completion Tracker

| Stage | Name | Status | Audited | Git Tag | Notes |
|-------|------|--------|---------|---------|-------|
| 0 | Project Skeleton | complete | post-audit done | stage-00-complete / v1.0 | |
| 1 | Board Representation | complete | post-audit done | stage-01-complete / v1.1 | |
| 2 | Move Generation + Attack Query API | complete | post-audit done | stage-02-complete / v1.2 | |
| 3 | Game State & Rules | complete | post-audit done | stage-03-complete / v1.3 | |
| 4 | Odin Protocol | complete | post-audit done | stage-04-complete / v1.4 | |
| 5 | Basic UI Shell | complete | post-audit done | stage-05-complete / v1.5 | |
| 6 | Bootstrap Eval + Evaluator Trait | complete | post-audit done | stage-06-complete / v1.6 | |
| 7 | Plain BRS + Searcher Trait | complete | post-audit done | stage-07-complete / v1.7 | |
| 8 | BRS/Paranoid Hybrid Layer | complete (pending user verification) | post-audit done | — | All steps done; awaiting user testing before tag |
| 9 | TT & Move Ordering | not-started | — | — | |
| 10 | MCTS | not-started | — | — | |
| 11 | Hybrid Integration | not-started | — | — | |
| 12 | Self-Play & Regression Testing | not-started | — | — | |
| 13 | Time Management | not-started | — | — | |
| 14 | NNUE Feature Design & Architecture | not-started | — | — | |
| 15 | NNUE Training Pipeline | not-started | — | — | |
| 16 | NNUE Integration | not-started | — | — | |
| 17 | Game Mode Variant Tuning | not-started | — | — | |
| 18 | Full UI | not-started | — | — | |
| 19 | Optimization & Hardening | not-started | — | — | |

**Status values:** `not-started` | `in-progress` | `complete` | `blocked`
**Audited values:** `—` (not applicable) | `pre-audit done` | `post-audit done` | `audit-failed`

---

## Documentation Status

| Document | Status | Notes |
|----------|--------|-------|
| MASTERPLAN.md | current | v3.1 (minor refinements applied per recent commit). |
| AGENT_CONDUCT.md | current | v1.1 — Section 1.16 added (Deferred-Debt Escalation), Section 3 replaced (tracing). |
| 4PC_RULES_REFERENCE.md | current | Complete game rules. |
| DECISIONS.md | current | 15 ADRs. ADR-007/008 superseded by ADR-015 (Huginn → tracing). ADR-014 (UI Vision), ADR-015 (Retire Huginn). |
| HANDOFF.md | current | Stage 8 complete, pending user verification. |
| STATUS.md (this file) | current | |
| README.md | current | Project overview at repo root. |
| audit_log_stage_00.md through audit_log_stage_08.md | current | All complete. |
| downstream_log_stage_00.md through downstream_log_stage_08.md | current | All complete. |

---

## What the Next Session Should Do First

1. Read STATUS.md + HANDOFF.md (this file + HANDOFF.md)
2. **User testing results:** User will run their own tests on Stage 8 before proceeding
3. If user approves: tag `stage-08-complete` / `v1.8`, begin Stage 9 (TT & Move Ordering)
4. If issues found: fix and re-test

---

## Known Regressions

None. All existing tests pass (361 engine + 54 UI Vitest).

---

## Non-Stage Changes

**2026-02-25 — In-Search Repetition + UI Bugfixes** ([[Session-2026-02-25-UI-Bugfixes]]):

In-search repetition detection added to BRS (`game_history` snapshot + `rep_stack` path-local, push/pop in max_node/min_node). Depth default raised to 7. Piece-prefix notation added to game log. Critical bug fixed: game log player labels were shifted by one due to React 18 batching — `currentPlayerRef.current` was read inside a deferred functional updater, seeing the *next* player's value. Fixed by snapshotting both `currentPlayerRef.current` and `boardRef.current` as locals before calling `setMoveHistory`. Commits: `f50fc57`, `b98c087`.

**2026-02-24 — UI Pause/Resume Bugfix** ([[Session-2026-02-24-Bugfix-Pause-Resume]]):

Fixed race condition in `useGameState.ts` where pausing and resuming auto-play could send duplicate `position + go` commands to the engine, causing one player to move twice in a row. Two guards added: `sendGoFromRef` checks `awaitingBestmoveRef` before sending, `togglePause` skips scheduling if a search is already in flight. See [[Issue-UI-Pause-Resume-Race-Condition]].

**2026-02-23 — UI QoL Session** ([[Session-UI-QoL-2026-02-23]]):

Added 4 new UI components and enhanced 2 existing ones outside the numbered stage pipeline:
- `AnalysisPanel` — prominent NPS display + search summary (replaces DebugConsole info section)
- `GameLog` — enriched move history with per-move eval/depth/nodes and player-colored borders
- `EngineInternals` — collapsible panel: search phase, BRS surviving, MCTS sims, per-player values
- `CommunicationLog` — raw protocol log + command input (split from DebugConsole)
- `BoardSquare` — optional coordinate labels on each square
- `BoardDisplay` — mouse wheel zoom (CSS transform, known buggy — polish later)
- Right panel reorganized from single DebugConsole to stacked sections

Follow-up items noted but not blocking:
- Board zoom frame boundary shift (cosmetic, polish phase)
- Info duplication between AnalysisPanel and EngineInternals
- No per-player scoring log (capture/elimination point tracking)

---

## Performance Baselines

| Metric | Value | Stage | Notes |
|---|---|---|---|
| `cargo build` (dev) | 0.70s | 0 | Empty project baseline |
| `cargo build --release` | 1.30s | 0 | Binary: 129,024 bytes |
| Test count | 2 | 0 | |
| `cargo build` (dev, incremental) | ~0.18s | 1 | Board module added |
| `cargo build --release` | ~0.33s | 1 | Binary: 129,024 bytes (unchanged — main.rs is empty) |
| Test count | 64 | 1 | 44 unit + 2 stage-00 + 18 stage-01 |
| Test count | 125 | 2 | 87 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 |
| perft(1) | 20 | 2 | Permanent invariant |
| perft(2) | 395 | 2 | Permanent invariant |
| perft(3) | 7,800 | 2 | Permanent invariant |
| perft(4) | 152,050 / ~0.56s | 2 | Permanent invariant (debug build) |
| 1000 random games @ 100 ply | ~15s | 2 | Debug build |
| Test count | 164 | 3 | 108 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 |
| 1000 random games via GameState | ~104s | 3 | Normal mode, debug build (permanent invariant) |
| 1000 random games via GameState (terrain) | ~104s | 3 | Terrain mode, debug build (permanent invariant) |
| Test count | 229 | 4 | 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 |
| Vitest test count | 45 | 5 | 29 board-constants + 16 protocol-parser |
| Vitest test count | 54 | 7 (bugfix) | 29 board-constants + 25 protocol-parser (9 new) |
| Tauri backend compile (fresh) | ~11s | 5 | Debug profile |
| Test count | 275 | 6 | 191 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 + 11 stage-06 |
| eval_scalar per call | <10us | 6 | Release build, starting position |
| Starting material per player | 4300cp | 6 | 8P + 2N + 2B + 2R + Q + K |
| Test count | 302 | 7 | 196 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 + 11 stage-06 + 22 stage-07 |
| Test count | 316 | 8 (Step 0) | 210 unit + 106 integration (11 new unit tests for GameMode/EvalProfile) |
| Test count | 361 | 8 (complete) | 233 unit + 128 integration (3 ignored). Board scanner, hybrid scoring, eval fix, smoke-play. |
| BRS depth 6 (debug, starting pos) | 1,547ms / 10,916 nodes | 7 | ~7k NPS debug |
| BRS depth 6 (release, starting pos) | 109ms / 10,916 nodes | 7 | ~100k NPS release |
| BRS depth 8 (release, starting pos) | 371ms / 31,896 nodes | 7 | Move converges at depth 6 (j1i3); stable 6-8 |
| BRS depth 4 (CI cap) | 80ms debug / 4ms release | 7 | Stable move e1f3 |
| Hybrid BRS depth 6 (release) | < 10,916 nodes (~49% reduction) | 8 | Progressive narrowing active |
| Hybrid BRS depth 8 (release) | < 31,896 nodes (~46% reduction) | 8 | Progressive narrowing active |
| Board scanner | < 1ms per call | 8 | Release build |

---

*Update this file at the end of every session. It takes 2 minutes and saves the next session 30 minutes of orientation.*
