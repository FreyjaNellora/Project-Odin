# Session 2026-03-06: Stage 20 Gen-0 Training Complete

**Date:** 2026-03-05 to 2026-03-06 (multi-day session)
**Stage:** 20 -- Gen-0 NNUE Training Run
**Outcome:** Stage complete. Pipeline proven end-to-end.

---

## What Happened

### Day 1 (2026-03-05)
- Stage entry protocol: read upstream audit/downstream logs for Stages 14-16
- Wrote Stage 20 spec in [[MASTERPLAN]] (line 1391)
- Created [[audit_log_stage_20]] with pre-audit
- User set up Kaggle account (free tier)
- Launched datagen: 1000 games, depth 4, FFA, sample_interval 4
- Datagen ran overnight (~30 hours)

### Day 2 (2026-03-06)
- Datagen completed: 40,243 samples from 1000 games
- Converted JSONL -> binary (22.4 MB, 0 skipped)
- Installed Kaggle CLI (KGAT token didn't work for upload, used browser)
- Uploaded dataset manually to Kaggle
- Ran kaggle_train.ipynb on GPU T4 x2 (had to fix dataset path)
- Downloaded best_model.pt + weights_gen0.onnue
- T13 test failed: i32 overflow in forward_pass output heads
- Fixed: widened BRS + MCTS head accumulation to i64
- T13 failed again: strict inequality on clamped BRS score
- Fixed: relaxed assertion to allow boundary values
- T13 passes. Full suite: 594 passed, 0 regressions.
- Post-audit complete.

---

## Bug Found

**i32 overflow in NNUE output heads** -- `hidden[h] * weights.brs_weights[h] as i32` overflows when hidden layer values are large (which happens with real trained weights, not random weights used during Stage 14 testing). Fixed by widening to i64. Same issue in MCTS head.

This bug was latent since Stage 14 but only triggered by real trained weights. Random weights during Stage 14 testing produced small values that stayed in i32 range.

---

## Key Decisions

- **Depth 4 for datagen** instead of depth 8. Depth 8 was too slow (~15-30 min/game). Depth 4 gives a full board rotation at ~1.8 min/game.
- **200-ply cap** causes ~50% draws. Acceptable for gen0 -- MCTS targets still provide training signal.
- **User wants bootstrap eval removed** after NNUE weights are viable. To be recorded in [[DECISIONS]] when executed.
- **.gitattributes added** for LF normalization across Windows/Linux tooling.

---

## Links

- [[audit_log_stage_20]]
- [[HANDOFF]]
- [[STATUS]]
- [[Issue-UI-React-Hooks-Queue-Error]]
