# HANDOFF — Last Session Summary

**Date:** 2026-02-28
**Stage:** Stage 14 (NNUE Feature Design & Architecture) — IMPLEMENTATION COMPLETE. Pending human review + tag.
**Next:** Human reviews, tags `stage-14-complete` / `v1.14`, then begin Stage 15 (NNUE Training Pipeline).

## What Was Done This Session

### Stage 14: NNUE Feature Design & Architecture

1. **`odin-engine/src/util.rs` (CREATE)** — Extracted `SplitMix64` PRNG from `search/mcts.rs` to shared module. Added `next_i16()`, `next_i8()`, `next_i32()` convenience methods. Pure refactor.

2. **`odin-engine/src/eval/nnue/features.rs` (CREATE)** — HalfKP-4 feature encoding. 160 valid squares × 7 piece types × 4 relative owners = 4,480 features per perspective. Static const dense square mapping tables. `relative_owner()` CW rotation. `active_features()` returns fixed `[u16; 64]` + count (zero heap allocation). 7 unit tests.

3. **`odin-engine/src/eval/nnue/weights.rs` (CREATE)** — `NnueWeights` struct with per-perspective FT weights (~8.7 MB), hidden layer, dual output heads. `.onnue` binary format (48-byte header, CRC32 footer). Inline CRC32 IEEE 802.3 + FNV-1a architecture hash (no external crates). `random(seed)`, `save(path)`, `load(path)`. 5 unit tests.

4. **`odin-engine/src/eval/nnue/accumulator.rs` (CREATE)** — `Accumulator` (4 perspectives × 256 int16) + `AccumulatorStack` (128 pre-allocated entries, ~262 KB). Copy-on-push, zero-cost pop. Full refresh + incremental delta updates. King moves mark `needs_refresh` for owner's perspective; EP/castling fall back to full refresh.

5. **`odin-engine/src/eval/nnue/mod.rs` (CREATE)** — Quantized forward pass: SCReLU (QA=255) → hidden layer (1024→32, int8 weights) → dual output heads (BRS scalar centipawns + MCTS 4-player sigmoid). `NnueEvaluator` implements frozen Evaluator trait via `RefCell<AccumulatorStack>`. Stage 14: full refresh every eval call.

6. **`odin-engine/tests/stage_14_nnue.rs` (CREATE, 18 tests)** — T1-T18: feature indexing, square mapping, weight determinism, save/load roundtrip, accumulator full/incremental, push/pop, forward pass determinism, eval range, sensitivity, captures, castling, promotion, magic validation, benchmarks.

7. **Module wiring** — `lib.rs`: added `pub mod util;`. `eval/mod.rs`: added `pub mod nnue;` + `pub use nnue::NnueEvaluator;`. `search/mcts.rs`: replaced inline SplitMix64 with `use crate::util::SplitMix64;`.

8. **Documentation** — `audit_log_stage_14.md` (pre+post audit), `downstream_log_stage_14.md` (W17-W19, API contracts, baselines), STATUS.md, HANDOFF.md, session note.

---

## What's Next — Priority-Ordered

### 1. Human Review + Tag Stage 14

Review the changes. Tag `stage-14-complete` / `v1.14`.

### 2. Begin Stage 15 (NNUE Training Pipeline)

Per MASTERPLAN. Training data generation, self-play data pipeline, NNUE weight training.

---

## Known Issues

- **W17:** `NnueEvaluator` does full refresh every eval call. Stage 16 must wire `AccumulatorStack::push/pop` into BRS make/unmake for incremental updates.
- **W18:** King moves mark `needs_refresh` even without king bucketing (Phase 1). Correct but wasteful — profile in Stage 19 if needed.
- **W19:** EP/castling fall back to full refresh. Conservative but correct. Optimize in Stage 19 if profiling warrants.
- **W15 (carried):** `PositionType::Endgame` triggers at `piece_count() <= 16`. May need tuning.
- **W16 (carried):** `limits_to_budget()` takes `current_player: Option<Player>`.
- **W13 (carried):** MCTS score 9999 (max) — unchanged.
- **Pondering not implemented:** Deferred from Stage 13.

## Files Created/Modified This Session

- `odin-engine/src/util.rs` — CREATED (SplitMix64 shared module)
- `odin-engine/src/eval/nnue/mod.rs` — CREATED (forward pass, NnueEvaluator)
- `odin-engine/src/eval/nnue/features.rs` — CREATED (HalfKP-4 feature indexing)
- `odin-engine/src/eval/nnue/accumulator.rs` — CREATED (Accumulator, AccumulatorStack)
- `odin-engine/src/eval/nnue/weights.rs` — CREATED (NnueWeights, .onnue format)
- `odin-engine/tests/stage_14_nnue.rs` — CREATED (18 acceptance tests)
- `odin-engine/src/lib.rs` — MODIFIED (pub mod util)
- `odin-engine/src/eval/mod.rs` — MODIFIED (pub mod nnue + re-export)
- `odin-engine/src/search/mcts.rs` — MODIFIED (use crate::util::SplitMix64)
- `masterplan/audit_log_stage_14.md` — FILLED
- `masterplan/downstream_log_stage_14.md` — FILLED
- `masterplan/STATUS.md` — UPDATED
- `masterplan/HANDOFF.md` — REWRITTEN (this file)
- `masterplan/sessions/Session-2026-02-28-Stage14-NNUE-Design.md` — CREATED

## Test Counts

- Engine: 519 (305 unit + 214 integration, 5 ignored)
- UI Vitest: 54
- Total: 0 failures, 0 clippy warnings
