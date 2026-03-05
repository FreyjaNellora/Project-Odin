# PROJECT ODIN -- STATUS

**Last Updated:** 2026-03-05
**Updated By:** Claude Sonnet 4.6 (Stage 19: Optimization & Hardening -- Phases 1-7 complete)

---

## Current State

| Field | Value |
|-------|-------|
| **Current Stage** | Stage 19 (Optimization & Hardening) -- Phases 1-7 complete. Pending post-audit + tag. |
| **Current Build-Order Step** | Stage 19 complete. Next: Stage 20 (Gen-0 NNUE training run on GPU). |
| **Build Compiles** | Yes -- cargo build --release passes with LTO, 0 warnings |
| **Tests Pass** | Yes -- engine: 600 total (573 unit+integration + 27 fuzz, 6 ignored); UI: 63 Vitest. |
| **Blocking Issues** | None. Gen-0 pipeline run (Stage 15) still needed for trained NNUE weights. |

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
| 8 | BRS/Paranoid Hybrid Layer | complete | post-audit done | stage-08-complete / v1.8 | |
| 9 | TT & Move Ordering | complete | post-audit done | stage-09-complete / v1.9 | |
| 10 | MCTS | complete | post-audit done | stage-10-complete / v1.10 | |
| 11 | Hybrid Integration | complete | post-audit done | -- | Pending tag. |
| 12 | Self-Play & Regression Testing | complete | post-audit done | -- | Pending tag. |
| 13 | Time Management | complete | post-audit done | -- | Pending tag. |
| 14 | NNUE Feature Design & Architecture | complete | post-audit done | -- | Pending tag. |
| 15 | NNUE Training Pipeline | complete | post-audit done | -- | Pending Gen-0 run + T13 + tag. |
| 16 | NNUE Integration | complete | post-audit done | -- | Pending tag. |
| 17 | Game Mode Variant Tuning | complete | post-audit done | -- | Pending tag. |
| 18 | Full UI | complete | post-audit done | -- | Pending tag. |
| 19 | Optimization & Hardening | in-progress | -- | -- | Phases 1-7 done. Pending post-audit + tag. |

---

## What the Next Session Should Do First

1. Read STATUS.md + HANDOFF.md
2. Begin Stage 20 entry protocol (AGENT_CONDUCT 1.1)
3. Run Gen-0 NNUE training pipeline on GPU (see Stage 15 spec)

---

## Known Regressions

None. All tests pass (600 engine + 63 UI Vitest).

## Deferred Issues (non-blocking)

- EP rule correctness: ep_sq cleared too eagerly after every make_move -- eligible players denied window. Low impact in practice.
- TT EP flag: compress_move drops EP flag; decompress_move re-derives. Potential stale TT replay in edge cases.
- Pondering: Deferred from Stage 13.
- NPS stretch goals (1M NPS, 10K sims/sec): Require tree parallelism, deferred.

---

## Performance Baselines (Stage 19 additions)

| Metric | Baseline | Final | Improvement |
|--------|---------|-------|-------------|
| forward_pass | 55.9 us | 1.37 us | 40.8x |
| full_init | 9.6 us | 3.78 us | 2.5x |
| incremental_push | 948 ns | 798 ns | 1.2x |
| BRS depth 4 | 3.5 ms | 3.18 ms | 1.1x |
| BRS depth 6 | 62.3 ms | 25.3 ms | 2.46x |
| MCTS 1000 sims | 133.7 ms | 124.9 ms | 1.07x |

---

*Update this file at the end of every session. It takes 2 minutes and saves the next session 30 minutes of orientation.*
