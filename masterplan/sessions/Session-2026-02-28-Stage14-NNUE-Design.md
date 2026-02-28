# Session: Stage 14 — NNUE Feature Design & Architecture

**Date:** 2026-02-28
**Agent:** Claude Opus 4.6
**Stage:** 14
**Duration:** Single session (continued from context overflow)

## Summary

Implemented the full Stage 14 deliverable set: HalfKP-4 feature encoding (4,480 features per perspective), quantized NNUE inference pipeline (FT→hidden→dual output heads), accumulator with incremental updates, .onnue weight file format, and NnueEvaluator implementing the frozen Evaluator trait. All 18 acceptance tests pass. Tested with random weights; no training (Stage 15) or search integration (Stage 16).

## Changes

### SplitMix64 Extraction (`util.rs`)
- Moved `SplitMix64` from `search/mcts.rs` to shared `util.rs`
- Added `next_i16()`, `next_i8()`, `next_i32()` convenience methods
- Pure refactor — no behavior change

### Feature Indexing (`eval/nnue/features.rs`)
- HalfKP-4 feature set: 160 valid squares × 7 piece types × 4 relative owners = 4,480 features
- Static const `SQUARE_TO_DENSE` (196→0-159/255) and `DENSE_TO_SQUARE` (160→square) tables
- `relative_owner(perspective, piece_owner)` — CW rotation: 0=Own, 1=Next, 2=Across, 3=Prev
- `feature_index(sq, piece_type, rel_owner)` → `Option<u16>` (None for invalid squares)
- `active_features(board, perspective)` → `([u16; 64], usize)` — fixed array + count, no heap

### Weights + .onnue Format (`eval/nnue/weights.rs`)
- `NnueWeights`: per-perspective FT (4 × 4480 × 256 int16), hidden (1024 × 32 int8), BRS head (32→1), MCTS head (32→4)
- `.onnue` binary: 48-byte header (magic "ONUE", version, arch hash, dims), weight data, CRC32 footer
- CRC32 IEEE 802.3 table-based implementation (const table, no external crate)
- Architecture hash: FNV-1a based, 32 bytes from "HalfKP4-4480-256-32-1-4"
- `random(seed)`, `save(path)`, `load(path)` with error handling

### Accumulator (`eval/nnue/accumulator.rs`)
- `Accumulator`: 4 perspectives × 256 int16, per-perspective `needs_refresh` flags
- `AccumulatorStack`: pre-allocated 128 entries (~262 KB), copy-on-push / zero-cost pop
- Full refresh: bias + sum of active feature columns
- Incremental: add/sub individual feature columns (O(256) per feature)
- Push logic: king moves → refresh owner's perspective, EP/castling → refresh all, else incremental
- `refresh_if_needed()` — call before forward pass

### Forward Pass + NnueEvaluator (`eval/nnue/mod.rs`)
- SCReLU activation: `clamp(x, 0, 255)^2`
- Hidden layer: 1024→32, int8 weights × (activated / QA), ClippedReLU
- BRS head: 32→1, rescaled by OUTPUT_SCALE=400, clamped [-30000, 30000]
- MCTS head: 32→4, per-player sigmoid (SIGMOID_K=4000.0), values in [0.0, 1.0]
- `NnueEvaluator`: `RefCell<AccumulatorStack>` for interior mutability
- Stage 14: full refresh every eval call (incremental tested separately)

### Acceptance Tests (`tests/stage_14_nnue.rs`)
18 tests (T1-T18): feature indexing, square mapping, weight determinism, save/load roundtrip, accumulator full/incremental, push/pop, forward pass determinism, eval range, sensitivity, captures, castling, promotion, magic validation, benchmarks.

## Key Design Decisions

1. **Per-perspective FT weights (4 copies)** — Not shared. Training may learn perspective-specific weights for 4-player asymmetry.
2. **Fixed `[u16; 64]` for active features** — No Vec/SmallVec, zero heap allocation. Max ~30 pieces per side.
3. **King moves mark refresh** — Even though Phase 1 doesn't need it (no king bucketing). Future-proofs for Phase 2.
4. **EP/castling fall back to refresh** — Conservative but correct. EP is rare; castling is complex (player-dependent rook positions).
5. **Per-player sigmoid (NOT softmax)** — Matches existing `normalize_4vec`. Independent probabilities per player.

## Errors Encountered + Fixed

- Import paths: `crate::board::square::Square` → `crate::board::Square` (private submodule)
- Clippy: `needless_range_loop`, `new_without_default`, `manual_clamp` — all fixed
- T15 stack overflow: 200 moves without pop exceeded MAX_STACK_DEPTH=128 → re-init every 100 moves
- `GameState::make_move()` doesn't exist → `apply_move()` in T12
- Unused imports cleaned up

## Test Results
- Before: 490 (292 unit + 198 integration, 5 ignored)
- After: 519 (305 unit + 214 integration, 5 ignored)
  - +13 nnue unit tests (in features.rs, weights.rs)
  - +18 stage_14 integration tests (T1-T18, 2 are benchmarks)
  - Note: some existing tests were reclassified in the count
- Clippy: 0 warnings

## Links
- Audit: [[audit_log_stage_14]]
- Downstream: [[downstream_log_stage_14]]
- Status: [[STATUS]]
- Handoff: [[HANDOFF]]
