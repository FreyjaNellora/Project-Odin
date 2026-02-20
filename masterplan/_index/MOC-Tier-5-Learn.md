---
type: moc
tags:
  - type/moc
  - tier/learn
last_updated: 2026-02-19
---

# Tier 5: Learn (Stages 14-16)

NNUE neural network evaluation: design, training, and integration.

## Stage Specs (in [[MASTERPLAN]] Section 4)

| Stage | Spec | Audit Log | Downstream Log |
|---|---|---|---|
| 14 -- NNUE Feature Design & Architecture | [[stage_14_nnue_design]] | [[audit_log_stage_14]] | [[downstream_log_stage_14]] |
| 15 -- NNUE Training Pipeline | [[stage_15_nnue_training]] | [[audit_log_stage_15]] | [[downstream_log_stage_15]] |
| 16 -- NNUE Integration | [[stage_16_nnue_integration]] | [[audit_log_stage_16]] | [[downstream_log_stage_16]] |

## Key Decisions

- [[DECISIONS]] ADR-003: Dual-head NNUE with Evaluator trait
- [[DECISIONS]] ADR-004: HalfKP-4 feature set (4,480 features per perspective)

## Parallel Work

Stage 14 depends only on Stage 6 (Evaluator trait). Can start parallel with Stages 10-11.
