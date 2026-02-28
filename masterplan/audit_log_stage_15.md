# Audit Log — Stage 15: NNUE Training Pipeline

## Pre-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes — `cargo build` passes, 0 warnings
- Tests pass: Yes — 519 engine tests (305 unit + 214 integration, 5 ignored), 54 UI Vitest
- Previous downstream flags reviewed: W17 (full refresh), W18 (king refresh), W19 (EP/castling refresh) — all carried, not affected by Stage 15 (training pipeline only)

### Findings
- Clean codebase entry point. No blocking issues.
- Stage 14 `.onnue` format, architecture hash, CRC32 are the cross-language invariants that Stage 15 must match exactly.
- `serde` + `serde_json` are the first external crates added to odin-engine. Scoped to datagen CLI path only — not in eval/search hot path.

### Risks for This Stage
- **CRITICAL:** Architecture hash mismatch between Python export and Rust loader would cause silent weight corruption.
- **CRITICAL:** CRC32 mismatch would prevent weight loading entirely.
- **MEDIUM:** Weight transposition (PyTorch [out,in] → .onnue [in,out]) off by one dimension would corrupt all inference.
- **LOW:** serde dependency in hot path — mitigated by isolating to `datagen::run()` only.

---

## Post-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Deliverables Check

| ID | Test | Status | Notes |
|----|------|--------|-------|
| T1 | `test_datagen_replay_startpos` | PASS | Empty move list → starting position verified |
| T2 | `test_datagen_replay_moves` | PASS | Single move, multiple moves, invalid move |
| T3 | `test_datagen_feature_extraction` | PASS | Binary features match `active_features()` for all 4 perspectives |
| T4 | `test_datagen_binary_roundtrip` | PASS | All 556-byte fields verified: BRS, MCTS, game_result, ply, game_id |
| T5 | `test_datagen_skips_eliminated` | PASS | PlayerStatus::Active verified, elimination skip logic tested |
| T6 | `test_model_forward_shape` | PASS | OdinNNUE output shapes correct (Python) |
| T6b | `test_model_deterministic` | PASS | Same input → same output (Python) |
| T7 | `test_dataset_loading` | PASS | Synthetic .bin parsed correctly (Python) |
| T8 | `test_loss_computation` | PASS | Multi-task loss computes without NaN (Python) |
| T9 | `test_export_magic` | PASS | .onnue magic bytes "ONUE" verified (Python) |
| T10 | `test_architecture_hash` | PASS | Python FNV-1a hash matches Rust exactly (Python) |
| T11 | `test_export_roundtrip` | PASS | Header fields survive write/read cycle (Python) |
| T12 | `test_training_loss_decreases` | PASS | Loss decreases over 10 epochs on synthetic data (Python) |
| T13 | `test_load_exported_weights` | IGNORED | Integration test — requires full pipeline run (human-driven) |

**Rust tests:** 526 total (305 unit + 221 integration, 6 ignored), 0 clippy warnings
**Python tests:** 8 total, all pass

### Code Quality

#### Uniformity
- `datagen.rs` follows existing engine patterns: `Result<T, String>` error handling, `GameState` + `Board` usage, `active_features()` integration.
- Python files follow standard PyTorch conventions: `nn.Module` subclass, `Dataset` subclass, standard training loop.
- Binary format documentation in both Rust doc comments and Python docstrings.

#### Bloat
- `serde` + `serde_json` added (~200KB binary increase). Justified: JSONL hand-parsing is fragile for nested optional fields. Only used in `datagen::run()` CLI path.
- No unnecessary abstractions. `extract_sample()` is one function, not a builder pattern.

#### Efficiency
- `replay_moves()` uses `gs.legal_moves()` + `to_algebraic()` matching. O(legal_moves) per move. Acceptable for offline datagen — not a hot path.
- `extract_sample()` calls `active_features()` 4 times (once per perspective). No heap allocation in the extraction loop.
- Python dataset loads entire .bin into memory — fine for Gen-0 (~28MB for 50K samples).

#### Dead Code
- None. All public functions are used by tests or the CLI entry point.
- `parse_player()` and `find_arg()` are private helpers used by `run()`.

#### Broken Code
- None. All tests pass.

#### Temporary Code
- None. No TODO/FIXME/HACK markers.

### Search/Eval Integrity
- **No changes to eval or search code.** Stage 15 only adds the `datagen` module and its CLI entry point.
- perft invariants unaffected (verified: perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050).
- All 519 pre-existing tests still pass.

### Future Conflict Analysis
- **Stage 16 (NNUE Integration):** `datagen.rs` is independent. No conflicts expected. Stage 16 wires `NnueEvaluator` into search — completely separate code path.
- **Weight format:** Python export matches `.onnue` format exactly (T10, T11 verify). Stage 16 will use `NnueWeights::load()` to consume exported weights.
- **match.mjs datagen mode:** Additive change. Existing match modes (`standard`, `regression`) are unaffected. Datagen mode is a separate code path triggered by `config.mode === 'datagen'`.

### Unaccounted Concerns
- **T13 must be run before declaring Stage 15 fully complete.** It is `#[ignore]` because it requires the full Python pipeline to have run first. The human must run the Gen-0 pipeline (Step 7) and then `cargo test -- test_load_exported_weights --ignored`.
- **v1-v4 null handling:** Positions with null v1-v4 (forced moves, instant returns) are skipped in both match.mjs sampling and Rust datagen parsing. This is correct behavior but means some game states are not represented in training data.

### Reasoning & Methods
- Verified all 526 Rust tests pass via `cargo test`.
- Verified 0 clippy warnings via `cargo clippy -- -D warnings`.
- Verified all 8 Python tests pass via `python -m pytest odin-nnue/test_pipeline.py`.
- Cross-referenced binary format offsets in `datagen.rs` against `stage_15_datagen.rs` test assertions.
- Verified architecture hash cross-language invariant via T10 (Python hash matches Rust constant).
- Verified weight transposition logic: PyTorch `nn.Linear` stores `[out, in]`, export transposes to `[in, out]` matching Rust's `[feature][neuron]` layout.

---

## Related

- Stage spec: [[stage_15_nnue_training]]
- Downstream log: [[downstream_log_stage_15]]
