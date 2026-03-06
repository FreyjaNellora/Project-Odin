# HANDOFF -- Stage 20 In Progress

**Date:** 2026-03-05
**Stage:** Stage 20 -- Gen-0 NNUE Training Run (IN PROGRESS)
**Next:** Continue Stage 20 after datagen completes

---

## What Was Done This Session

1. **Stage entry protocol complete** -- Read all upstream audit/downstream logs for Stages 14-16. Build and test verified clean (594 passed, 0 failed, 6 ignored).

2. **Stage 20 spec written** -- Added formal Stage 20 specification to MASTERPLAN.md (line 1391). Tier 5 (Learn), depends on Stages 15 and 19. Five acceptance criteria defined.

3. **Pre-audit complete** -- Created audit_log_stage_20.md with full pre-audit section covering upstream findings, risks, and build state.

4. **STATUS.md updated** -- Stage 19 marked complete, Stage 20 in-progress.

5. **Datagen launched** -- 1000 games at depth 4 (full board rotation), FFA mode, sample interval 4. Running via `node match.mjs datagen_gen0_config.json` in observer/. Output: `observer/training_data_gen0.jsonl`. Estimated ~33 hours to complete.

---

## What Was NOT Completed

- JSONL -> binary conversion (waiting for datagen)
- Kaggle upload and GPU training
- T13 integration test
- Post-audit

---

## Files Modified

- `masterplan/MASTERPLAN.md` -- Added Stage 20 spec (line 1391)
- `masterplan/STATUS.md` -- Stage 19 complete, Stage 20 in-progress
- `masterplan/audit_log_stage_20.md` -- NEW, pre-audit complete
- `observer/datagen_gen0_config.json` -- NEW, depth 4 config for Gen-0

---

## What the Next Session Should Do First

1. Check if datagen is done: look for `observer/training_data_gen0.jsonl` (should have ~35K lines)
2. If done, convert: `./target/release/odin-engine.exe --datagen --input observer/training_data_gen0.jsonl --output observer/training_data_gen0.bin`
3. Upload `.bin` to Kaggle as a dataset
4. Open `odin-nnue/kaggle_train.ipynb` on Kaggle, enable GPU, configure `BIN_PATH` to point at uploaded dataset, run all cells
5. Download `weights_gen0.onnue` from Kaggle output
6. Run T13: `cargo test -- test_load_exported_weights --ignored`
7. Run 10+ self-play games with NNUE weights to verify AC4

---

## Deferred Issues (non-blocking, carried)

- EP rule correctness: ep_sq cleared too eagerly
- TT EP flag: compress_move drops EP flag
- W21: Null v1-v4 positions excluded from training data
- Pondering: Deferred from Stage 13
- NPS stretch goals: Require tree parallelism
