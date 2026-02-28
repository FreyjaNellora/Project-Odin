# Audit Log — Stage 14: NNUE Feature Design & Architecture

## Pre-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes — `cargo build` + `cargo clippy` clean (0 warnings)
- Tests pass: Yes — 292 unit + 196 integration = 488 passing (5 ignored)
- Previous downstream flags reviewed: Yes — W14 (NNUE score scale), W15 (endgame threshold), W16 (limits_to_budget player param)

### Files to Create

| File | Purpose |
|------|---------|
| `odin-engine/src/util.rs` | SplitMix64 PRNG (shared module) |
| `odin-engine/src/eval/nnue/mod.rs` | NnueEvaluator, forward pass |
| `odin-engine/src/eval/nnue/features.rs` | HalfKP-4 feature indexing, square mapping |
| `odin-engine/src/eval/nnue/accumulator.rs` | Accumulator, AccumulatorStack, incremental |
| `odin-engine/src/eval/nnue/weights.rs` | NnueWeights, .onnue format, CRC32 |
| `odin-engine/tests/stage_14_nnue.rs` | 18 acceptance tests |

### Files to Modify

| File | Change |
|------|--------|
| `odin-engine/src/lib.rs` | Add `pub mod util;` |
| `odin-engine/src/eval/mod.rs` | Add `pub mod nnue;` + re-export |
| `odin-engine/src/search/mcts.rs` | Replace SplitMix64 with import from util |

### Findings

- Pre-existing audit_log_stage_14.md template was already created (empty).
- FT weights are per-perspective (4 copies × 4480 × 256 = ~8.7 MB). Intentional for 4-player asymmetric perspectives.
- SplitMix64 needs extraction from search/mcts.rs to shared util module.

### Risks for This Stage

1. **Evaluator trait FROZEN (`&self`)**: NnueEvaluator uses `RefCell<AccumulatorStack>` for interior mutability.
2. **FT weight size**: 4 × 4480 × 256 × 2 bytes ≈ 8.7 MB. Fine for desktop.
3. **i32 overflow in hidden layer**: Max 1024 × 127 × 255 ≈ 33M. Within i32 range.
4. **CRC32 + architecture hash**: No external crates — implementing minimal versions inline.

---

## Post-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|-------------|--------|-------|
| HalfKP-4 feature indexing | DONE | 160 squares × 7 types × 4 relative owners = 4,480 features. Dense square mapping, relative owner rotation. |
| Square-to-dense mapping | DONE | Static const tables, 160 valid → 0-159, invalid → 255. Roundtrip verified. |
| Feature transformer (per-perspective) | DONE | 4 separate weight matrices, int16 accumulator, 256 neurons. |
| Accumulator + stack | DONE | Copy-on-push, zero-cost pop. Full refresh + incremental delta updates. |
| Incremental updates | DONE | Quiet moves, captures (normal), promotions. King/EP/castling fall back to refresh. |
| Forward pass (quantized) | DONE | SCReLU (QA=255), hidden layer (int8→i32), ClippedReLU, dual output heads. |
| BRS scalar head | DONE | 32→1, rescaled to centipawns, clamped [-30000, 30000]. |
| MCTS value head | DONE | 32→4, per-player sigmoid (SIGMOID_K=4000.0), [0.0, 1.0]. |
| NnueEvaluator | DONE | Implements frozen Evaluator trait with RefCell<AccumulatorStack>. |
| .onnue file format | DONE | 48-byte header (magic "ONUE", version, arch hash, dims), CRC32 footer. |
| NnueWeights::random(seed) | DONE | SplitMix64-based, deterministic. |
| Save/load roundtrip | DONE | Bit-exact after save + load. |
| SplitMix64 extraction | DONE | Moved to `util.rs`, shared by MCTS and NNUE. |
| 18 acceptance tests | DONE | All pass. T1-T18 in stage_14_nnue.rs. |

### AC Verification

| Test | Verified | Method |
|------|----------|--------|
| T1 | YES | All 4,480 feature indices in [0, 4480), no duplicates |
| T2 | YES | Same piece from different perspectives gives different indices |
| T3 | YES | 160 valid → 0-159, invalid → 255 |
| T4 | YES | Same seed → identical weights |
| T5 | YES | Save to temp, load back, bit-identical |
| T6 | YES | Starting position → all 4 perspectives non-zero |
| T7 | YES | Incremental == full refresh after 1, 3, 5, 10 moves |
| T8 | YES | Push 10, pop 10 → bit-for-bit match |
| T9 | YES | Same accumulator + weights → same output twice |
| T10 | YES | All 4 players' scalar in [-30000, 30000] |
| T11 | YES | All 4 values in [0.0, 1.0] |
| T12 | YES | Both heads produce different outputs for different positions |
| T13 | YES | Capture move: incremental matches full |
| T14 | YES | Castling: refresh-based matches full |
| T15 | YES | Promotion: incremental matches full |
| T16 | YES | Bad magic → NnueLoadError::InvalidMagic |
| T17 | YES | Benchmark prints timing (release-only assertion) |
| T18 | YES | Benchmark prints timing (release-only assertion) |

### Code Quality

#### Uniformity

All new code follows existing patterns: module structure mirrors eval/ submodules, test file follows stage_NN_*.rs naming, SplitMix64 extraction preserves exact behavior. `NnueEvaluator` mirrors `BootstrapEvaluator` pattern (struct + Evaluator impl). CRC32/architecture hash are self-contained with no external crates.

#### Bloat

Minimal. 4 source files (~600 lines total), 1 utility extraction, 1 test file (18 tests). The NnueWeights struct stores ~8.7 MB of FT weights (per-perspective) — this is inherent to the architecture, not bloat.

#### Efficiency

- Accumulator add/sub are O(256) per feature — tight inner loop.
- Forward pass is O(1024×32 + 32×4) — single matrix multiply.
- SCReLU uses `i16::clamp()` — compiler optimizes to branch-free.
- CRC32 table is `const` — computed at compile time.
- AccumulatorStack pre-allocates 128 entries (262 KB) — no runtime allocation.

#### Dead Code

None. All public types and functions are used by tests or will be consumed by Stage 16.

#### Broken Code

None detected. All 519 tests pass. 0 clippy warnings.

#### Temporary Code

None. All code is permanent Stage 14 infrastructure.

### Search/Eval Integrity

- **Evaluator trait**: FROZEN. Not modified. NnueEvaluator implements it cleanly.
- **BootstrapEvaluator**: Not modified. NnueEvaluator is a parallel implementation.
- **BRS/MCTS internals**: Not modified. Only SplitMix64 was extracted (pure refactor, behavior identical).
- **TT probe order**: Unchanged.
- **perft invariants**: Untouched (movegen not modified).
- **SIGMOID_K = 4000.0**: Matching constant used in both BootstrapEvaluator and NnueEvaluator.
- **ELIMINATED_SCORE = -30000**: Used consistently for eliminated players.

### Future Conflict Analysis

- **Stage 15 (NNUE Training)**: Training will use `NnueWeights::save()` to write trained weights. The `.onnue` format and weight layout are ready.
- **Stage 16 (NNUE Integration)**: Will wire `AccumulatorStack::push/pop` into BRS `make_move/unmake_move`. The `NnueEvaluator::accumulator_stack()` accessor (via RefCell borrow) is not exposed yet but the RefCell pattern is in place. Stage 16 will likely replace the full-refresh-every-call pattern with search-integrated incremental updates.
- **Stage 19 (Optimization)**: SIMD can be applied to accumulator add/sub and hidden layer matmul. The current scalar loops are vectorization-friendly (contiguous memory, no data dependencies).

### Unaccounted Concerns

1. **EP and castling use refresh fallback**: En passant captures and castling mark `needs_refresh` rather than computing incremental deltas. This is correct but slightly wasteful. EP is rare; castling marks refresh only for the moving player's perspective (king move). Stage 16 can optimize if profiling shows it matters.
2. **King moves refresh only owner's perspective**: Other 3 perspectives handle king moves incrementally. This is correct for Phase 1 (no king bucketing). Phase 2 king bucketing would require all-perspective refresh.
3. **FT weights per-perspective (~8.7 MB)**: Could be shared (all use same weights) for 4× memory reduction. Kept separate per the prompt spec — training may learn perspective-specific weights.

### Reasoning & Methods

- Built bottom-up: features → weights → accumulator → forward pass → evaluator → tests. Each layer tested independently before integration.
- SplitMix64 extracted first to unblock both MCTS (existing) and NNUE (new). Pure refactor verified by `cargo test` before any new code.
- Used `RefCell<AccumulatorStack>` for interior mutability as specified — the Evaluator trait's `&self` signature is frozen and cannot be changed.
- Forward pass uses integer-only arithmetic until the final sigmoid, matching the quantization scheme from the prompt. The `OUTPUT_SCALE = 400` divisor converts raw int32 outputs to centipawn-scale values.
- Per-player sigmoid (NOT softmax) for MCTS head — matches existing `normalize_4vec` in `eval/mod.rs`.

---

## Related

- Stage spec: [[stage_14_nnue_design]]
- Downstream log: [[downstream_log_stage_14]]
