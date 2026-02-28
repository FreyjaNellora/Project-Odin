# HANDOFF — Last Session Summary

**Date:** 2026-02-28
**Stage:** Stage 15 (NNUE Training Pipeline) — IMPLEMENTATION COMPLETE. Pending human review, Gen-0 pipeline run, T13 verification, and tag.
**Next:** Human reviews, runs Gen-0 pipeline, verifies T13, tags `stage-15-complete` / `v1.15`, then begin Stage 16 (NNUE Integration).

## What Was Done This Session

### Stage 15: NNUE Training Pipeline

1. **`observer/match.mjs` (MODIFIED)** — Added datagen mode: v1-v4 capture from engine info, `runDatagen()` self-play loop, `samplePositions()` with skip logic (first 4 plies, eliminated, null v1-v4), `computeGameResult()` backfill, JSONL output.

2. **`observer/datagen_config.json` (CREATED)** — Datagen config: 1000 games, depth 6, sample_interval 4, FFA mode, aggressive eval.

3. **`odin-engine/Cargo.toml` (MODIFIED)** — Added `serde` (derive feature) + `serde_json` dependencies.

4. **`odin-engine/src/main.rs` (MODIFIED)** — `--datagen` CLI flag dispatch before protocol loop.

5. **`odin-engine/src/lib.rs` (MODIFIED)** — Added `pub mod datagen;`.

6. **`odin-engine/src/datagen.rs` (CREATED)** — JSONL reader, `replay_moves()`, `extract_sample()` (556-byte binary format), `run()` CLI entry point. Skips null v1-v4 and eliminated players.

7. **`odin-nnue/model.py` (CREATED)** — OdinNNUE: 4× FT(4480→256) + SCReLU → hidden(1024→32) → dual heads (BRS + MCTS).

8. **`odin-nnue/dataset.py` (CREATED)** — Binary .bin dataset loader (556-byte samples).

9. **`odin-nnue/train.py` (CREATED)** — Multi-task training: λ_BRS=1.0 MSE + λ_MCTS=0.5 sigmoid-MSE (70% search / 30% result) + λ_result=0.25 sigmoid-MSE. Adam, StepLR, early stopping. Windows auto-detect.

10. **`odin-nnue/export.py` (CREATED)** — PyTorch → `.onnue`: FNV-1a arch hash, CRC32, quantization (FT→int16, hidden→int8, biases→int32), weight transposition (PyTorch [out,in] → .onnue [in,out]).

11. **`odin-nnue/requirements.txt` (CREATED)** — `torch>=2.0`, `numpy`.

12. **`odin-nnue/test_pipeline.py` (CREATED)** — 8 Python tests (T6-T12).

13. **`odin-engine/tests/stage_15_datagen.rs` (CREATED)** — 7 Rust tests (T1-T5, T13).

14. **Documentation** — audit_log_stage_15.md, downstream_log_stage_15.md, STATUS.md, HANDOFF.md, session note.

---

## What's Next — Priority-Ordered

### 1. Human Review + Gen-0 Pipeline Run + Tag Stage 15

Review the changes. Run the Gen-0 pipeline:

```bash
cd observer && node match.mjs datagen_config.json
cd ../odin-engine && cargo run --release -- --datagen --input ../observer/training_data_gen0.jsonl --output ../odin-nnue/training_data_gen0.bin
cd ../odin-nnue && pip install -r requirements.txt && python train.py
python export.py best_model.pt weights_gen0.onnue
cd ../odin-engine && cargo test -- test_load_exported_weights --ignored
```

T13 must pass before tagging `stage-15-complete` / `v1.15`.

### 2. Begin Stage 16 (NNUE Integration)

Per MASTERPLAN. Wire `NnueEvaluator` into BRS search with incremental `AccumulatorStack` updates.

---

## Known Issues

- **W17 (carried):** `NnueEvaluator` does full refresh every eval call. Stage 16 must wire `AccumulatorStack::push/pop` into BRS make/unmake.
- **W18 (carried):** King moves mark `needs_refresh` even without king bucketing. Profile in Stage 19.
- **W19 (carried):** EP/castling fall back to full refresh. Profile in Stage 19.
- **W20 (new):** `serde` + `serde_json` in engine. Scoped to datagen CLI path only — not in eval/search hot path.
- **W21 (new):** T13 must be run manually after Gen-0 pipeline completes.
- **W22 (new):** Null v1-v4 positions excluded from training data.
- **W15 (carried):** `PositionType::Endgame` triggers at `piece_count() <= 16`.
- **W16 (carried):** `limits_to_budget()` takes `current_player: Option<Player>`.
- **W13 (carried):** MCTS score 9999 (max) — unchanged.
- **Pondering not implemented:** Deferred from Stage 13.

## Files Created/Modified This Session

- `observer/match.mjs` — MODIFIED (datagen mode)
- `observer/datagen_config.json` — CREATED
- `odin-engine/Cargo.toml` — MODIFIED (serde deps)
- `odin-engine/src/main.rs` — MODIFIED (--datagen dispatch)
- `odin-engine/src/lib.rs` — MODIFIED (pub mod datagen)
- `odin-engine/src/datagen.rs` — CREATED
- `odin-nnue/model.py` — CREATED
- `odin-nnue/dataset.py` — CREATED
- `odin-nnue/train.py` — CREATED
- `odin-nnue/export.py` — CREATED
- `odin-nnue/requirements.txt` — CREATED
- `odin-nnue/test_pipeline.py` — CREATED
- `odin-engine/tests/stage_15_datagen.rs` — CREATED
- `masterplan/audit_log_stage_15.md` — FILLED
- `masterplan/downstream_log_stage_15.md` — FILLED
- `masterplan/STATUS.md` — UPDATED
- `masterplan/HANDOFF.md` — REWRITTEN (this file)
- `masterplan/sessions/Session-2026-02-28-Stage15-Training-Pipeline.md` — CREATED

## Test Counts

- Engine: 526 (305 unit + 221 integration, 6 ignored)
- Python: 8 (pytest)
- UI Vitest: 54
- Total: 0 failures, 0 clippy warnings
