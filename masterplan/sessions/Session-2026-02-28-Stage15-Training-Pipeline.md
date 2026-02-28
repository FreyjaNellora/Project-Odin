# Session Note — 2026-02-28 — Stage 15: NNUE Training Pipeline

**Stage:** 15 (NNUE Training Pipeline)
**Status:** Implementation complete. Pending human review, Gen-0 pipeline run, T13 verification, and tag.
**Agent:** Claude Opus 4.6

---

## What Was Done

### Stage 15: NNUE Training Pipeline

Built the complete training pipeline: self-play data generation → feature extraction → PyTorch training → .onnue weight export.

1. **`observer/match.mjs` (MODIFIED)** — Added datagen mode. v1-v4 fields captured from engine `info` output. `runDatagen()` function: self-play loop → sample positions (skip first 4 plies, skip eliminated, skip null v1-v4) → backfill `game_result` → append JSONL.

2. **`observer/datagen_config.json` (CREATED)** — Datagen mode configuration: 1000 games, depth 6, sample_interval 4, FFA mode, aggressive eval profile.

3. **`odin-engine/Cargo.toml` (MODIFIED)** — Added `serde` (with derive feature) and `serde_json` dependencies for JSONL parsing.

4. **`odin-engine/src/main.rs` (MODIFIED)** — Added `--datagen` CLI flag dispatch before protocol loop.

5. **`odin-engine/src/lib.rs` (MODIFIED)** — Added `pub mod datagen;`.

6. **`odin-engine/src/datagen.rs` (CREATED, ~234 lines)** — JSONL reader, move replay (`replay_moves`), HalfKP-4 feature extraction (`extract_sample`), binary .bin writer. 556-byte fixed-size sample format. Skips null v1-v4 positions and eliminated players.

7. **`odin-nnue/model.py` (CREATED, ~65 lines)** — OdinNNUE PyTorch model: 4 separate `nn.Linear(4480, 256)` feature transformers, SCReLU activation, `nn.Linear(1024, 32)` hidden layer with ReLU, dual output heads (BRS 32→1, MCTS 32→4).

8. **`odin-nnue/dataset.py` (CREATED, ~55 lines)** — Binary .bin dataset loader. Reads entire file into memory, parses 556-byte samples, returns sparse features + targets.

9. **`odin-nnue/train.py` (CREATED, ~105 lines)** — Multi-task training loop. Loss: `λ_BRS × MSE(brs) + λ_MCTS × MSE(sigmoid, 0.7×search+0.3×result) + λ_result × MSE(sigmoid, result)`. Adam optimizer, StepLR, early stopping. Windows auto-detection for `num_workers=0`.

10. **`odin-nnue/export.py` (CREATED, ~130 lines)** — PyTorch → `.onnue` export. Architecture hash (FNV-1a, matches Rust), CRC32 (IEEE 802.3), weight quantization (FT→int16, hidden→int8, biases→int32), correct transposition (PyTorch [out,in] → .onnue [in,out]).

11. **`odin-nnue/requirements.txt` (CREATED)** — `torch>=2.0`, `numpy`.

12. **`odin-nnue/test_pipeline.py` (CREATED, ~315 lines)** — 8 Python tests (T6-T12): model shape, determinism, dataset loading, loss computation, magic bytes, architecture hash cross-verification, export roundtrip, training loss decrease.

13. **`odin-engine/tests/stage_15_datagen.rs` (CREATED, ~310 lines)** — 7 Rust tests (T1-T5, T13): replay startpos, replay moves (single/multiple/invalid), feature extraction with binary verification, binary roundtrip (all fields), eliminated player skip, weight loading integration (#[ignore]).

14. **Documentation** — `audit_log_stage_15.md`, `downstream_log_stage_15.md`, STATUS.md, HANDOFF.md, this session note.

---

## Issues Fixed During Implementation

- **Clippy: needless_range_loop** in `datagen.rs` — `for i in 0..count` → `for (i, &idx) in features.iter().enumerate().take(count)`
- **Clippy: unnecessary_min_or_max** in `datagen.rs` — `(sample.ply as u16).min(u16::MAX)` → `sample.ply as u16`
- **Clippy: manual_range_contains** in test file — `v >= 0.0 && v <= 1.0` → `(0.0..=1.0).contains(&v)`
- **T12 test flakiness** — LR=0.01 too high for random synthetic data, caused oscillation. Fixed: LR=0.001, 500 samples, 10 epochs, torch seed, compare best-loss vs first-loss.

---

## Test Counts

- Engine: 526 (305 unit + 221 integration, 6 ignored)
- Python: 8 (pytest)
- UI Vitest: 54
- Total: 0 failures, 0 clippy warnings

---

## What's Next

1. Human reviews Stage 15 changes
2. Run Gen-0 pipeline (Step 7 — human-driven, ~hours):
   ```bash
   cd observer && node match.mjs datagen_config.json
   cd ../odin-engine && cargo run --release -- --datagen --input ../observer/training_data_gen0.jsonl --output ../odin-nnue/training_data_gen0.bin
   cd ../odin-nnue && pip install -r requirements.txt && python train.py
   python export.py best_model.pt weights_gen0.onnue
   cd ../odin-engine && cargo test -- test_load_exported_weights --ignored
   ```
3. T13 must pass before tagging `stage-15-complete` / `v1.15`
4. Begin Stage 16 (NNUE Integration) — wires `NnueEvaluator` into BRS search with incremental accumulator updates

---

## Known Issues

- **W17 (carried):** Full refresh every eval call. Stage 16 wires incremental updates.
- **W18 (carried):** King refresh without bucketing. Profile in Stage 19.
- **W19 (carried):** EP/castling full refresh. Profile in Stage 19.
- **W20 (new):** serde/serde_json in engine. Scoped to datagen CLI path only.
- **W21 (new):** T13 must be run manually after Gen-0 pipeline.
- **W22 (new):** Null v1-v4 positions excluded from training data.

---

## Related

- Plan: [[Stage 15 Implementation Plan]]
- Audit: [[audit_log_stage_15]]
- Downstream: [[downstream_log_stage_15]]
- Previous: [[Session-2026-02-28-Stage14-NNUE-Design]]
