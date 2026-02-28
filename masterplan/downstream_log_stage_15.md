# Downstream Log — Stage 15: NNUE Training Pipeline

## Notes for Future Stages

### Must-Know

- **W17 (carried):** `NnueEvaluator` does a full refresh every call. Stage 16 must wire `AccumulatorStack::push/pop` into BRS `make_move/unmake_move`.
- **W18 (carried):** King moves mark `needs_refresh` even without king bucketing. Profile in Stage 19 if needed.
- **W19 (carried):** EP/castling fall back to full refresh. Optimize in Stage 19 if profiling warrants.
- **W20:** `serde` + `serde_json` are now engine dependencies. Only used in `datagen::run()` CLI path. Do NOT import serde in eval/search hot path.
- **W21:** T13 (`test_load_exported_weights`) is `#[ignore]` and must be run manually after the full Gen-0 pipeline completes. It proves the entire Python→Rust pipeline works end-to-end.
- **W22:** Positions with null v1-v4 (forced moves) are excluded from training data. If the MCTS head shows blind spots on forced-move positions, this is the cause.

### API Contracts

- **datagen CLI:** `odin-engine --datagen --input <file.jsonl> --output <file.bin>` — reads JSONL, writes binary .bin samples.
- **`replay_moves(move_strs: &[&str]) -> Result<GameState, String>`** — public, replays algebraic move strings from startpos. Used by datagen and available for other tools.
- **`extract_sample(gs: &GameState, sample: &TrainingSample) -> [u8; 556]`** — public, extracts binary training sample from a replayed position.
- **JSONL format:** `{"position_moves":"e2e4 j13j11 ...","ply":24,"side_to_move":"Yellow","score_cp":4312,"v1":0.73,...,"game_result":[0.0,1.0,0.0,0.0]}`
- **Binary .bin format:** 556 bytes per sample. Layout in `datagen.rs` doc comment.
- **Python model:** `OdinNNUE(qa=255)` with `forward(features) -> (brs, mcts)`. BRS: `[batch, 1]`, MCTS: `[batch, 4]`.
- **Python export:** `export(model_path, output_path, qa=255, qb=64)` writes `.onnue` compatible with `NnueWeights::load()`.
- **match.mjs datagen mode:** `config.mode === 'datagen'` triggers `runDatagen()`. Output is JSONL appended to `config.output_file`.

### Key Constants

| Constant | Value | Location |
|----------|-------|----------|
| `SAMPLE_SIZE` | 556 | `datagen.rs` |
| `MAX_FEATURES` | 64 | `datagen.rs` |
| `PERSPECTIVE_BYTES` | 129 | `datagen.rs` |
| `LAMBDA_BRS` | 1.0 | `train.py` |
| `LAMBDA_MCTS` | 0.5 | `train.py` |
| `LAMBDA_RESULT` | 0.25 | `train.py` |
| `SIGMOID_K` | 4000.0 | `train.py`, `eval/nnue/mod.rs` |
| `OUTPUT_SCALE` | 400 | `train.py`, `eval/nnue/features.rs` |
| `BATCH_SIZE` | 4096 | `train.py` |
| `LR` | 0.01 | `train.py` |
| `EPOCHS` | 20 | `train.py` |

### Known Limitations

- **W13 (carried):** MCTS score 9999 (max) in some positions — unchanged.
- **Pondering not implemented:** Deferred from Stage 13.
- **No SIMD:** Stage 19 target.
- **Gen-0 weights are bootstrapped from random self-play.** Quality will be poor. Gen-1+ training with stronger data is expected in future iterations.
- **Windows DataLoader:** `num_workers=0` auto-detected on Windows (multiprocessing issues). Training may be slower on Windows than Linux.

### Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| Rust datagen (1 sample extraction) | ~100-200us | `replay_moves` + `extract_sample`, debug build |
| Python training (1 epoch, 50K samples) | ~30-60s | Single GPU, batch_size=4096 |
| `.onnue` export | <1s | Model → quantized weights → file |
| `.bin` file size per sample | 556 bytes | Fixed-size binary records |
| Expected Gen-0 data size | ~28 MB | 50K samples × 556 bytes |

### Open Questions

- **Gen-0 quality:** Random weights produce random self-play data. The first generation of trained weights will be weak. How many generations of self-play → retrain are needed to reach competitive play? Not a Stage 15 concern — deferred to post-Stage 16 evaluation.
- **Training data diversity:** 1000 games with `sample_interval=4` produces ~50K positions. Is this sufficient for Gen-0? Likely yes for bootstrapping, may need more for Gen-1+.
- **Loss blend ratios:** `LAMBDA_BRS=1.0, LAMBDA_MCTS=0.5, LAMBDA_RESULT=0.25` are initial values. May need tuning based on training diagnostics.

### Reasoning

- Kept Rust changes minimal (one new module + CLI flag). No changes to eval/search.
- Python pipeline is standalone (`odin-nnue/` directory). Can be iterated independently of the engine.
- Binary format is fixed-size for simplicity and random access. Variable-length would save ~30% space but adds complexity.
- Multi-task loss blend (70% search target / 30% game result for MCTS head) follows Leela Chess Zero's approach adapted for 4-player.

---

## Related

- Stage spec: [[stage_15_nnue_training]]
- Audit log: [[audit_log_stage_15]]
