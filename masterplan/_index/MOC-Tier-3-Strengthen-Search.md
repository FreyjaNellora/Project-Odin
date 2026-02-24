---
type: moc
tags:
  - type/moc
  - tier/strengthen-search
last_updated: 2026-02-19
---

# Tier 3: Strengthen Search (Stages 8-11)

Hybrid search, transposition table, MCTS, and the controller that ties them together.

## Stage Specs (in [[MASTERPLAN]] Section 4)

| Stage | Spec | Audit Log | Downstream Log |
|---|---|---|---|
| 8 -- BRS/Paranoid Hybrid | [[stage_08_brs_hybrid]] / [[stage_08_build_order]] | [[audit_log_stage_08]] | [[downstream_log_stage_08]] |
| 9 -- TT & Move Ordering | [[stage_09_tt_ordering]] | [[audit_log_stage_09]] | [[downstream_log_stage_09]] |
| 10 -- MCTS Strategic Search | [[stage_10_mcts]] | [[audit_log_stage_10]] | [[downstream_log_stage_10]] |
| 11 -- Hybrid Integration | [[stage_11_hybrid_integration]] | [[audit_log_stage_11]] | [[downstream_log_stage_11]] |

## Key Decisions

- [[DECISIONS]] ADR-002: BRS/Paranoid hybrid, not pure MaxN or pure Paranoid

## Parallel Work

Stages 8, 9, and 10 can all run in parallel (no cross-dependencies). Stage 11 requires all three.

## Invariants Established

| Invariant | Stage |
|---|---|
| TT produces no correctness regressions | 9 |
