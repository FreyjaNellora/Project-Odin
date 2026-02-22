# PROJECT ODIN — STATUS

**Last Updated:** 2026-02-21
**Updated By:** Claude Sonnet 4.6 (Stage 7 bugfix session — second pass)

---

## Current State

| Field | Value |
|-------|-------|
| **Current Stage** | Stage 7 complete + bugfixes; ready for Stage 8 |
| **Current Build-Order Step** | Stage 7 complete (all steps + post-completion regressions fixed) |
| **Build Compiles** | Yes — `cargo build`, `cargo build --features huginn` both pass |
| **Tests Pass** | Yes — engine: 199 lib + 305 integration tests; UI: 54 Vitest |
| **Blocking Issues** | None |

---

## Stage Completion Tracker

| Stage | Name | Status | Audited | Git Tag | Notes |
|-------|------|--------|---------|---------|-------|
| 0 | Skeleton + Huginn Core | complete | post-audit done | stage-00-complete / v1.0 | |
| 1 | Board Representation | complete | post-audit done | stage-01-complete / v1.1 | |
| 2 | Move Generation + Attack Query API | complete | post-audit done | stage-02-complete / v1.2 | |
| 3 | Game State & Rules | complete | post-audit done | stage-03-complete / v1.3 | |
| 4 | Odin Protocol | complete | post-audit done | stage-04-complete / v1.4 | |
| 5 | Basic UI Shell | complete | post-audit done | stage-05-complete / v1.5 | |
| 6 | Bootstrap Eval + Evaluator Trait | complete | post-audit done | stage-06-complete / v1.6 | Tagged this session |
| 7 | Plain BRS + Searcher Trait | complete | post-audit done | stage-07-complete / v1.7 | Engine playable; post-completion regressions resolved |
| 8 | BRS/Paranoid Hybrid Layer | not-started | — | — | |
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
| AGENT_CONDUCT.md | current | v1.0 complete. |
| 4PC_RULES_REFERENCE.md | current | Complete game rules. |
| DECISIONS.md | current | 12 ADRs (ADR-012 added Stage 7: BRS turn order). |
| HANDOFF.md | current | Stage 7 + bugfix session state captured. |
| STATUS.md (this file) | current | |
| README.md | current | Project overview at repo root. |
| audit_log_stage_00.md through audit_log_stage_07.md | current | All complete. |
| downstream_log_stage_00.md through downstream_log_stage_07.md | current | All complete. |

---

## What the Next Session Should Do First

1. Read STATUS.md + HANDOFF.md (this file + HANDOFF.md)
2. Follow Stage Entry Protocol (AGENT_CONDUCT 1.1) for Stage 8
3. Stage 8: BRS/Paranoid Hybrid Layer
   - Depends on Stage 7 (→ 6 → 3 → 2 → 1 → 0)
   - Key task: improve eval's FFA strategic accuracy (lead penalty tuning) — see [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]]
   - Key task: verify and remove `[unverified]` from tactical_suite.txt mate positions
   - See [[downstream_log_stage_07]] for must-know items before modifying search or eval

---

## Known Regressions

None. All Stage 7 post-completion regressions resolved:
- Semi-auto human player guard (session 1)
- Checkmate detection DKW ordering (session 1)
- UI parser dropping `eliminated Red checkmate` events (session 2 — this session)
- Stage 7 integration tests updated for `info string nextturn` protocol addition (session 2)

---

## Performance Baselines

| Metric | Value | Stage | Notes |
|---|---|---|---|
| `cargo build` (dev) | 0.70s | 0 | Empty project baseline |
| `cargo build --features huginn` (dev) | 0.97s | 0 | |
| `cargo build --release` | 1.30s | 0 | Binary: 129,024 bytes |
| Test count (no huginn) | 2 | 0 | |
| Test count (with huginn) | 11 | 0 | |
| `cargo build` (dev, incremental) | ~0.18s | 1 | Board module added |
| `cargo build --release` | ~0.33s | 1 | Binary: 129,024 bytes (unchanged — main.rs is empty) |
| Test count (no huginn) | 64 | 1 | 44 unit + 2 stage-00 + 18 stage-01 |
| Test count (with huginn) | 73 | 1 | 53 unit + 2 stage-00 + 18 stage-01 |
| Test count (no huginn) | 125 | 2 | 87 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 |
| perft(1) | 20 | 2 | Permanent invariant |
| perft(2) | 395 | 2 | Permanent invariant |
| perft(3) | 7,800 | 2 | Permanent invariant |
| perft(4) | 152,050 / ~0.56s | 2 | Permanent invariant (debug build) |
| 1000 random games @ 100 ply | ~15s | 2 | Debug build |
| Test count (no huginn) | 164 | 3 | 108 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 |
| 1000 random games via GameState | ~104s | 3 | Normal mode, debug build (permanent invariant) |
| 1000 random games via GameState (terrain) | ~104s | 3 | Terrain mode, debug build (permanent invariant) |
| Test count (no huginn) | 229 | 4 | 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 |
| Vitest test count | 45 | 5 | 29 board-constants + 16 protocol-parser |
| Vitest test count | 54 | 7 (bugfix) | 29 board-constants + 25 protocol-parser (9 new) |
| Tauri backend compile (fresh) | ~11s | 5 | Debug profile |
| Test count (no huginn) | 275 | 6 | 191 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 + 11 stage-06 |
| eval_scalar per call | <10us | 6 | Release build, starting position |
| Starting material per player | 4300cp | 6 | 8P + 2N + 2B + 2R + Q + K |
| Test count (no huginn) | 302 | 7 | 196 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 + 11 stage-06 + 22 stage-07 |
| BRS depth 6 (debug, starting pos) | 1,547ms / 10,916 nodes | 7 | ~7k NPS debug |
| BRS depth 6 (release, starting pos) | 109ms / 10,916 nodes | 7 | ~100k NPS release |
| BRS depth 8 (release, starting pos) | 371ms / 31,896 nodes | 7 | Move converges at depth 6 (j1i3); stable 6-8 |
| BRS depth 4 (CI cap) | 80ms debug / 4ms release | 7 | Stable move e1f3 |

---

*Update this file at the end of every session. It takes 2 minutes and saves the next session 30 minutes of orientation.*
