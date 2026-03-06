---
tags:
  - stage/20
  - area/nnue-training
---

# Session 2026-03-05: Stage 20 Entry + Datagen Launch

## Summary

Started Stage 20 (Gen-0 NNUE Training Run). Completed full stage entry protocol, wrote formal Stage 20 spec in [[MASTERPLAN]], created pre-audit in [[audit_log_stage_20]], and launched self-play data generation.

## What Happened

1. Read [[STATUS]], [[HANDOFF]], and all upstream audit/downstream logs for Stages 14-16 ([[audit_log_stage_14]], [[audit_log_stage_15]], [[audit_log_stage_16]], [[downstream_log_stage_14]], [[downstream_log_stage_15]], [[downstream_log_stage_16]]).
2. Verified clean build: 594 tests passed, 0 failed.
3. Wrote Stage 20 spec in [[MASTERPLAN]] Section 4 (line 1391). Tier 5 (Learn), 5 acceptance criteria, 8 build-order steps.
4. Created [[audit_log_stage_20]] with full pre-audit documenting upstream findings and risks.
5. Updated [[STATUS]] -- Stage 19 complete, Stage 20 in-progress.
6. Launched datagen: 1000 games at depth 4, FFA mode, sample every 4 plies. Output: observer/training_data_gen0.jsonl. Estimated ~33 hours.

## Key Decisions

- **Depth 4 for datagen** (not depth 8): Gives one full rotation around the board (4 players). Depth 8 was ~10x slower with diminishing returns for Gen-0 bootstrap data.
- **Kaggle free tier** for GPU training: User set up account this session. Free T4/P100 GPU sufficient for Gen-0.
- **Formal Stage 20 spec** added to MASTERPLAN rather than treating as Stage 15 completion.

## Files Modified

- masterplan/MASTERPLAN.md -- Stage 20 spec added
- masterplan/STATUS.md -- Updated current stage
- masterplan/audit_log_stage_20.md -- NEW
- observer/datagen_gen0_config.json -- NEW

## Next Steps

Datagen running in background. Next session: convert JSONL -> binary, upload to Kaggle, train, export .onnue, run T13.
