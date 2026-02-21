# PROJECT ODIN — STATUS

**Last Updated:** 2026-02-20
**Updated By:** Claude Opus 4.6 (Stage 5 bugfix & play modes session)

---

## Current State

| Field | Value |
|-------|-------|
| **Current Stage** | Stage 5 complete, ready for Stage 6 |
| **Current Build-Order Step** | Stage 5 complete (all steps) |
| **Build Compiles** | Yes (engine: `cargo build`, `cargo build --features huginn`; UI: `cargo build` in src-tauri, `tsc --noEmit`) |
| **Tests Pass** | Yes (engine: 229 total; UI: 45 Vitest) |
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
| 5 | Basic UI Shell | complete | post-audit done | — | Tag pending. Bugfixes + play modes added post-audit (see addendum). |
| 6 | Bootstrap Eval + Evaluator Trait | not-started | — | — | |
| 7 | Plain BRS + Searcher Trait | not-started | — | — | |
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
| MASTERPLAN.md | current | v3.0 complete. 20 stages (0-19) in 6 tiers. |
| AGENT_CONDUCT.md | current | v1.0 complete. |
| 4PC_RULES_REFERENCE.md | current | Complete game rules. |
| DECISIONS.md | current | 11 ADRs from planning sessions. |
| HANDOFF.md | current | Stage 5 bugfix session state captured. |
| STATUS.md (this file) | current | |
| README.md | current | Project overview at repo root. |
| audit_log_stage_00.md | current | Pre-audit + post-audit complete. |
| downstream_log_stage_00.md | current | All sections filled. |
| audit_log_stage_01.md | current | Pre-audit + post-audit complete. |
| downstream_log_stage_01.md | current | All sections filled. |
| audit_log_stage_02.md | current | Pre-audit + post-audit complete. |
| downstream_log_stage_02.md | current | All sections filled. |
| audit_log_stage_03.md | current | Pre-audit + post-audit complete. |
| downstream_log_stage_03.md | current | All sections filled. |
| audit_log_stage_04.md | current | Pre-audit + post-audit complete. |
| downstream_log_stage_04.md | current | All sections filled. |
| audit_log_stage_05.md | current | Pre-audit + post-audit + bugfix addendum complete. |
| downstream_log_stage_05.md | current | All sections filled. Play mode API contracts added. |

---

## What the Next Session Should Do First

1. Create `stage-05-complete` / `v1.5` git tag
2. Begin Stage 6: Bootstrap Eval + Evaluator Trait
3. Follow Stage Entry Protocol (AGENT_CONDUCT 1.1)
4. Note: Stage 6 is independent of Stage 5 in the dependency chain (both depend on Stage 3)

---

## Known Regressions

None.

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
| Tauri backend compile (fresh) | ~11s | 5 | Debug profile |

---

*Update this file at the end of every session. It takes 2 minutes and saves the next session 30 minutes of orientation.*
