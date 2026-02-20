---
type: moc
tags:
  - type/moc
  - tier/simple-search
last_updated: 2026-02-19
---

# Tier 2: Simple Search (Stages 6-7)

Evaluation and basic search. Engine becomes playable (weakly) after Stage 7.

## Stage Specs (in [[MASTERPLAN]] Section 4)

| Stage | Spec | Audit Log | Downstream Log |
|---|---|---|---|
| 6 -- Bootstrap Eval + Evaluator Trait | [[stage_06_bootstrap_eval]] | [[audit_log_stage_06]] | [[downstream_log_stage_06]] |
| 7 -- Plain BRS + Searcher Trait | [[stage_07_plain_brs]] | [[audit_log_stage_07]] | [[downstream_log_stage_07]] |

## Key Decisions

- [[DECISIONS]] ADR-003: Dual-head NNUE with Evaluator trait (eval_scalar + eval_4vec)
- [[DECISIONS]] ADR-009: Searcher trait defined early, implemented incrementally

## Invariants Established

| Invariant | Stage |
|---|---|
| Evaluator trait is the eval boundary | 6 |
| Eval produces sane values | 6 |
| Searcher trait is the search boundary | 7 |
| Engine finds forced mates | 7 |
