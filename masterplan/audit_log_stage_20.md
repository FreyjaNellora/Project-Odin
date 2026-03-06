# Audit Log -- Stage 20: Gen-0 NNUE Training Run

**Auditor:** Claude Opus 4.6
**Date:** 2026-03-05
**Stage:** 20 -- Gen-0 NNUE Training Run

---

## Pre-Audit

### Build State
- `cargo build --release`: PASS (0 warnings, LTO enabled)
- `cargo test`: PASS (594 passed, 0 failed, 6 ignored)
- UI tests: 63 Vitest, 0 failures

### Upstream Log Review

**Audit Log Stage 14 (NNUE Feature Design):**
- No blocking findings
- W14: Output scale = 400, BRS head range [-30000, 30000] cp
- Architecture: HalfKP-4, 4480 features/perspective, FT 256, hidden 32, QA=255, QB=64

**Audit Log Stage 15 (NNUE Training Pipeline):**
- CRITICAL: Architecture hash (FNV-1a) must match between Python export and Rust loader
- CRITICAL: CRC32 must match or weights reject at load
- CRITICAL: Weight transposition: PyTorch [out,in] -> .onnue [in,out]
- T13 (`test_load_exported_weights`) is `#[ignore]` -- must run after Gen-0
- W20: Serde dependency only in datagen CLI path
- W21: Null v1-v4 positions excluded from training data (~20-30% gap)
- W22: Windows DataLoader `num_workers=0` (single-threaded)

**Audit Log Stage 16 (NNUE Integration):**
- No blocking findings
- W17 RESOLVED: Incremental accumulator updates wired in BRS + MCTS
- W23: Opponent move selection uses BootstrapEvaluator (by design)
- W24: MCTS root expansion doesn't track accumulator (correct)
- W25: Constructor signatures accept optional `nnue_weights`/`nnue_path`

**Downstream Log Stage 14:**
- .onnue format: 48-byte header + weight matrices + CRC32 footer (~9.2 MB total)
- Key constants: FT_IN=4480, FT_OUT=256, HIDDEN=32, QA=255, QB=64, OUTPUT_SCALE=400

**Downstream Log Stage 15:**
- SAMPLE_SIZE=556 bytes, BATCH_SIZE=4096, LR=0.01, EPOCHS=20
- Datagen CLI: `odin-engine --datagen --input <jsonl> --output <bin>`
- Observer datagen: `node match.mjs datagen_config.json` -> JSONL
- Key API: `NnueWeights::load(path)`, `NnueWeights::save(path)`
- T13 command: `cargo test -- test_load_exported_weights --ignored`

**Downstream Log Stage 16:**
- NNUE integrated into BRS + MCTS via Evaluator trait
- AccumulatorStack: push before make_move, pop after unmake_move
- Bootstrap fallback when no NNUE weights provided

### Risks Identified for This Stage
1. **Gen-0 quality will be poor.** Training on random self-play produces weak weights. Expected.
2. **Architecture hash mismatch.** If Python FNV-1a hash diverges from Rust, weights reject at load.
3. **Kaggle environment setup.** User has no existing Kaggle account; guided setup required.
4. **Data volume.** 1000 games at depth 8 may take hours. May need to reduce depth or game count.
5. **Windows DataLoader bottleneck.** If training locally, `num_workers=0` on Windows.

---

## Implementation Log

(To be filled during implementation)

---

## Post-Audit

(To be filled after implementation)
