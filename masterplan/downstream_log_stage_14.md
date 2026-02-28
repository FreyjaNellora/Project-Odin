# Downstream Log — Stage 14: NNUE Feature Design & Architecture

## Notes for Future Stages

### Must-Know

- **W14 (resolved):** NNUE eval stays in centipawn scale — `OUTPUT_SCALE = 400` divisor converts raw int32 outputs to centipawn-scale. BRS head output range is [-30000, 30000]. TimeManager's `score_cp < 2000` threshold is compatible.
- **W15 (carried):** `PositionType::Endgame` triggers at `piece_count() <= 16`. Unchanged by Stage 14.
- **W16 (carried):** `limits_to_budget()` takes `current_player: Option<Player>`. Unchanged.
- **W17:** `NnueEvaluator` does a full refresh every call in Stage 14. Stage 16 must wire `AccumulatorStack::push/pop` into BRS `make_move/unmake_move` for incremental updates. The `RefCell<AccumulatorStack>` is ready but currently reset each eval call.
- **W18:** King moves mark `needs_refresh` for the owning perspective (even though Phase 1 HalfKP-4 doesn't require it). If Phase 2 king bucketing is added, this is correct. If not, it's wasted refresh — profile if it matters.
- **W19:** EP and castling fall back to full refresh rather than incremental delta. EP is rare; castling marks all perspectives. Could be optimized in Stage 19 if profiling warrants it.

### API Contracts

- `NnueEvaluator` implements the frozen `Evaluator` trait (`eval_scalar`, `eval_4vec`). Drop-in replacement for `BootstrapEvaluator`.
- `NnueWeights::random(seed)` — deterministic random weights for testing. Same seed = identical weights.
- `NnueWeights::save(path)` / `NnueWeights::load(path)` — `.onnue` binary format with CRC32 integrity check.
- `AccumulatorStack::push(mv, board_before, weights)` — incremental update. Must be called BEFORE `make_move` (needs `board_before`).
- `AccumulatorStack::pop()` — zero-cost restore. Must be paired with `push` (copy-on-push design).
- `AccumulatorStack::refresh_if_needed(board, weights)` — call before forward pass to ensure all perspectives are current.
- `forward_pass(acc, weights, player)` → `(i16, [f64; 4])` — BRS centipawns + MCTS per-player sigmoid values.
- Feature transformer weights are per-perspective (4 copies). Training (Stage 15) can learn perspective-specific weights.

### Key Constants

| Constant | Value | Location |
|----------|-------|----------|
| `FEATURES_PER_PERSPECTIVE` | 4,480 | `eval/nnue/features.rs` |
| `FT_OUT` | 256 | `eval/nnue/features.rs` |
| `HIDDEN_SIZE` | 32 | `eval/nnue/features.rs` |
| `QA` | 255 (i16) | `eval/nnue/features.rs` |
| `QB` | 64 (i32) | `eval/nnue/features.rs` |
| `OUTPUT_SCALE` | 400 | `eval/nnue/features.rs` |
| `MAX_STACK_DEPTH` | 128 | `eval/nnue/features.rs` |
| `SIGMOID_K` | 4000.0 | `eval/nnue/mod.rs` |
| `ELIMINATED_SCORE` | -30,000 | `eval/nnue/mod.rs` |
| `MCTS_OUTPUT` | 4 | `eval/nnue/features.rs` |

### Known Limitations

- **W13 (carried):** MCTS score 9999 (max) in some positions — unchanged.
- **Pondering not implemented:** Deferred from Stage 13.
- **No SIMD:** Accumulator add/sub and hidden layer matmul are scalar loops. Vectorization-friendly but not vectorized. Stage 19 target.
- **FT weights ~8.7 MB:** Per-perspective (4 × 4480 × 256 × 2 bytes). Could be shared for 4× reduction but kept separate for future perspective-specific training.

### Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| Full eval (NnueEvaluator, random weights) | ~30-50us | Starting position, release build |
| Incremental accumulator update | ~1-5us | Per push, release build |
| Forward pass (1024→32→dual) | ~2-5us | After accumulator ready, release build |
| AccumulatorStack memory | ~262 KB | 128 entries × 4 × 256 × 2 bytes pre-allocated |
| FT weight memory | ~8.7 MB | Per-perspective, no sharing |
| .onnue file size | ~9.2 MB | 48-byte header + weights + 4-byte CRC32 |

### Open Questions

- **Should Stage 16 share FT weights across perspectives?** Currently 4 separate matrices. Training may or may not learn perspective-specific weights. If shared, 4× memory reduction. Decision deferred to training results.
- **Accumulator refresh frequency in search:** Stage 14 does full refresh every eval call. Stage 16 must decide when to refresh vs incremental. The `needs_refresh` flags handle this automatically — just call `refresh_if_needed` before forward pass.

### Reasoning

- Built bottom-up (features → weights → accumulator → forward pass → evaluator) with each layer independently tested before integration.
- Used `RefCell<AccumulatorStack>` for interior mutability because the Evaluator trait is frozen at `&self`.
- Per-player sigmoid (NOT softmax) for MCTS head matches existing `normalize_4vec` behavior. The 4 values are independent probabilities.
- CRC32 and architecture hash implemented inline (no external crates) to maintain zero-dependency approach.

---

## Related

- Stage spec: [[stage_14_nnue_design]]
- Audit log: [[audit_log_stage_14]]
