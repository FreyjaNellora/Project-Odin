# Audit Log — Stage 19: Optimization & Hardening

## Pre-Audit
**Date:** 2026-03-01
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes — `cargo build` passes, 0 warnings
- Tests pass: Yes — 557 engine tests (308 unit + 249 integration, 6 ignored), 63 UI Vitest
- Previous downstream flags reviewed: Yes — `downstream_log_stage_16.md` and `downstream_log_stage_12.md`

### Upstream Dependency Review

**Stage 16 (NNUE Integration):**
- W17 RESOLVED: AccumulatorStack incrementally updated in BRS and MCTS
- W18 (carried): King moves mark `needs_refresh` — profile in this stage
- W19 (carried): EP/castling fall back to full refresh — profile in this stage
- W20 (carried): `serde` + `serde_json` only in datagen CLI path
- W23: Opponent move selection uses BootstrapEvaluator — by design
- W25: Constructor signatures accept nnue_weights/nnue_path parameter
- No SIMD yet — this stage's primary target

**Stage 12 (Self-Play & Regression Testing):**
- 9 regression tests, match manager (`observer/match.mjs`), Elo+SPRT tools available
- R7 king safety test `#[ignore]` — aspirational, not blocking

### Files to Create

| File | Purpose | AC |
|------|---------|-----|
| `odin-engine/benches/engine_bench.rs` | Criterion benchmarks | — |
| `odin-engine/src/eval/nnue/simd.rs` | AVX2 SIMD with scalar fallback | AC3 |
| `odin-engine/tests/stage_19_optimization.rs` | Performance regression tests | AC3-5 |
| `odin-engine/tests/stage_19_fuzz.rs` | Fuzz/edge-case tests | AC2 |

### Files to Modify

| File | Change |
|------|--------|
| `odin-engine/Cargo.toml` | `criterion`, `arrayvec`, `[profile.release]` LTO |
| `odin-engine/src/eval/nnue/mod.rs` | SIMD dispatch in forward_pass |
| `odin-engine/src/eval/nnue/accumulator.rs` | SIMD add/sub, assert→debug_assert |
| `odin-engine/src/eval/nnue/weights.rs` | Hidden weight transpose at load time |
| `odin-engine/src/movegen/generate.rs` | ArrayVec `generate_legal_into()` |
| `odin-engine/src/search/brs.rs` | Move ordering alloc elimination, countermoves persist |
| `odin-engine/src/search/mcts.rs` | Buffer reuse |
| `odin-engine/src/protocol/mod.rs` | Error handling hardening |

### AC Mapping

| AC | Description | Covered By |
|----|-------------|------------|
| AC1 | No crashes in 10K self-play | Stress test runner |
| AC2 | No panics from fuzz | `stage_19_fuzz.rs` |
| AC3 | NNUE eval < 1us incremental | `simd.rs` + benchmarks |
| AC4 | BRS > 500K NPS (pass) / > 1M (stretch) | Phase 1.5+2+3 |
| AC5 | MCTS > 5K sims/sec (pass) / > 10K (stretch) | Phase 1.5+2+3 |

### Risks for This Stage

1. **CRITICAL:** SIMD correctness — unsafe AVX2 intrinsics, subtle numerical bugs. Mitigation: sub-step checkpoints with scalar-vs-SIMD comparison tests after each sub-phase.
2. **MEDIUM:** NPS targets may plateau if movegen is the bottleneck. Profile-guided approach.
3. **MEDIUM:** 10K game stress test takes ~7-28 hours. Run parallel batches with checkpointing.
4. **LOW:** LTO increases release compile time 2-3x. Only affects release builds.
5. **LOW:** `arrayvec` new dependency — zero-dep, widely used in game engines.


---

## Implementation Progress (Phases 1-4)
**Date:** 2026-03-01
**Implementer:** Claude Opus 4.6

### Phase 1: Benchmarking Baseline (complete)
Created `odin-engine/benches/engine_bench.rs` with Criterion benchmarks for 9 functions.

**Baseline (release build, LTO=false, codegen-units=16):**

| Metric | Value |
|--------|-------|
| nnue_forward_pass | 55.9 µs |
| nnue_incremental_push | 948 ns |
| nnue_full_init | 9.6 µs |
| generate_legal_startpos | 4.5 µs |
| brs_depth_4 | 3.5 ms |
| brs_depth_6 | 62.3 ms |
| mcts_1000_sims | 133.7 ms |
| make_unmake_move | 52.7 ns |

### Phase 1.5: Release Profile Tuning (complete)
Added to workspace `Cargo.toml`: `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`.

### Phase 2: SIMD NNUE (complete)
Created `odin-engine/src/eval/nnue/simd.rs` with:
- AVX2 accumulator add/sub (`_mm256_adds_epi16` / `_mm256_subs_epi16`)
- AVX2 SCReLU activation (clamp + unpack i16→i32 + square)
- AVX2 hidden layer MatVec (`_mm256_maddubs_epi16` + `_mm256_madd_epi16`)
- Runtime AVX2 detection via `OnceLock<bool>`, scalar fallbacks for all
- 8 SIMD-vs-scalar correctness tests

Modified: `accumulator.rs` (add_feature/sub_feature/compute_perspective use SIMD), `mod.rs` (forward_pass uses SIMD SCReLU + hidden layer), `weights.rs` (hidden_weights_t transpose at load time).

**After SIMD:**

| Metric | Before | After | Speedup |
|--------|--------|-------|---------|
| forward_pass | 55.9 µs | 1.37 µs | **40.8x** |
| full_init | 9.6 µs | 3.78 µs | 2.5x |
| incremental_push | 948 ns | 798 ns | 1.2x |

**AC3 PASS:** incremental push 798ns < 1µs target.

### Phase 3: Memory Optimization (complete)
- Added `arrayvec = "0.7"` dependency
- Created `MoveBuffer` trait + `generate_legal_into()`, `generate_legal_captures_into()`, `generate_pseudo_legal_into()` (zero-heap variants)
- Converted BRS `alphabeta()` to use ArrayVec movegen
- Converted BRS `quiescence()` to use `generate_legal_captures_into()` (avoids generating all moves just to filter captures)
- Converted `order_moves()`: all 6 Vec allocations → ArrayVec + `[bool; 256]` stack array
- Changed `max_node()`/`min_node()` signatures from `Vec<Move>` to `&[Move]`
- Changed `BrsContext.game_history` from `Vec<u64>` (heap clone) to `Arc<Vec<u64>>` (O(1) ref-count clone)
- Added `GameState::position_history_arc()` for Arc sharing

**After Memory Optimization:**

| Metric | Before | After | Speedup |
|--------|--------|-------|---------|
| BRS depth 4 | 3.58 ms | 3.18 ms | 1.13x |
| BRS depth 6 | 62.4 ms | 25.3 ms | **2.46x** |
| MCTS 1000 | 158.7 ms | 124.9 ms | 1.27x |

### Phase 4: Bitboard Decision (SKIP)
After SIMD + memory optimization, board scanning is not the dominant cost. BRS depth 6 dropped from 62.3ms → 25.3ms without any board representation changes. The 14×14 board would need u256 bitboards (non-standard), making a retrofit impractical for marginal gains. **Decision: Skip.**

### Overall Results (Phases 1-4)

| Metric | Baseline | Final | Improvement |
|--------|---------|-------|-------------|
| forward_pass | 55.9 µs | 1.37 µs | **40.8x** |
| full_init | 9.6 µs | 3.78 µs | **2.5x** |
| incremental_push | 948 ns | 798 ns | **1.2x** |
| BRS depth 4 | 3.5 ms | 3.18 ms | **1.1x** |
| BRS depth 6 | 62.3 ms | 25.3 ms | **2.46x** |
| MCTS 1000 sims | 133.7 ms | 124.9 ms | **1.07x** |

Tests: 567 passed (557 original + 8 SIMD tests + 2 bench-related), 6 ignored, 0 failures.

### Remaining Phases
- Phase 5: Stress test — reduced to 1K games/batch (2x/day) per user direction
- Phase 6: Fuzz testing
- Phase 7: Error handling hardening
- Post-audit

---

## Post-Audit
**Date:**
**Auditor:**

### Deliverables Check


### Code Quality
#### Uniformity

#### Bloat

#### Efficiency

#### Dead Code

#### Broken Code

#### Temporary Code


### Search/Eval Integrity


### Future Conflict Analysis


### Unaccounted Concerns


### Reasoning & Methods


---

## Related

- Stage spec: [[stage_19_polish]]
- Downstream log: [[downstream_log_stage_19]]
