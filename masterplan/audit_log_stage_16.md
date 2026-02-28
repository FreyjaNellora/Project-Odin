# Audit Log — Stage 16: NNUE Integration

## Pre-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes — `cargo build` passes, 0 warnings
- Tests pass: Yes — 526 engine tests (305 unit + 221 integration, 6 ignored), 54 UI Vitest
- Previous downstream flags reviewed: W17 (full refresh per eval call — Stage 16 resolves this), W18 (king refresh — carried), W19 (EP/castling refresh — carried), W20 (serde in datagen only — carried)

### Findings
- Clean codebase entry point. Stage 15 complete, Stage 14 NNUE infrastructure available.
- W17 is the primary target: wire `AccumulatorStack::push/pop` into BRS `make_move/unmake_move` and MCTS simulation paths.
- Key design: AccumulatorStack lives in searchers (BrsSearcher, MctsSearcher), not in evaluator. Evaluator trait is `&self` (immutable), push/pop needs `&mut`.
- Even in NNUE mode, both searchers keep `BootstrapEvaluator` for opponent move selection in free functions.

### Risks for This Stage
- **CRITICAL:** Push/pop mismatch (missing pop on early return, exception path) corrupts all subsequent evals.
- **CRITICAL:** MCTS `gs.apply_move()` triggers eliminations that `push()` doesn't know about — accumulator retains stale king features.
- **MEDIUM:** Null move pruning — doesn't move any piece, just advances side_to_move. Must NOT push/pop.
- **LOW:** Constructor signature changes break existing tests (mechanical fix, well-detected by compiler).

---

## Post-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Deliverables Check

| ID | Test | Status | Notes |
|----|------|--------|-------|
| T1 | `test_nnue_brs_push_pop_matches_full` | PASS | 8-move sequence: incremental matches full recompute at every step, all 4 perspectives |
| T2 | `test_nnue_mcts_sim_accumulator_depth` | PASS | 100 MCTS sims with NNUE, no stack overflow or depth mismatch |
| T3 | `test_fallback_without_nnue_file` | PASS | HybridController with `nnue_path: None` completes search using bootstrap eval |
| T4 | `test_perft_unchanged` | PASS | perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050 |
| T5 | `test_nnue_eval_non_degenerate` | PASS | BRS score non-zero, MCTS values in (0,1) with random weights |
| T6 | `test_brs_search_with_nnue` | PASS | BRS depth 6 with random NNUE weights, returns valid move |
| T7 | `test_mcts_search_with_nnue` | PASS | MCTS 500 sims with random NNUE weights, returns valid move |
| T8 | `test_hybrid_search_with_nnue` | PASS | Hybrid search with .onnue file loaded, returns valid move |
| T9 | `test_nnue_vs_bootstrap_no_crash` | PASS | 10 self-play games, 5 ply each, BRS+NNUE, no panics |
| T10 | `test_incremental_vs_full_speed` | PASS | Incremental faster than full recompute |

### Code Quality
#### Uniformity
- Push/pop pattern is identical across all 4 BRS sites and 2 MCTS simulation sites: `push(mv, board_before, weights)` → `make_move/apply_move` → ... → `unmake_move` → `pop()`.
- NNUE fields follow consistent pattern: `acc_stack: Option<AccumulatorStack>` + `nnue_weights: Option<Arc<NnueWeights>>` in both searcher structs.

#### Bloat
- No unnecessary abstractions. Direct `if let (Some(...), Some(...))` pattern for NNUE path — clean fallback to bootstrap when `None`.
- `nnue_eval_scalar()` helper method in BrsContext avoids code duplication at 6 eval call sites.

#### Efficiency
- AccumulatorStack push is O(features changed) per move, pop is O(1). Full refresh only on king/EP/castling moves.
- MCTS elimination-aware refresh (`needs_refresh = [true; 4]`) is cheap — eliminations are rare.
- `Arc<NnueWeights>` shared between BRS and MCTS — no weight duplication.

#### Dead Code
- None introduced. No feature flags, no commented-out code.

#### Broken Code
- None. All 536 tests pass.

#### Temporary Code
- None. Debug tracing is behind `#[cfg(debug_assertions)]` — compiled out in release builds.

### Search/Eval Integrity
- perft(1-4) invariants preserved (T4).
- Turn order R→B→Y→G unchanged.
- TT probe after repetition check — unchanged.
- Null move pruning: no push/pop (correct — no piece movement, only side_to_move change).
- Evaluator trait FROZEN — no signature changes. BrsContext keeps bootstrap evaluator ref for `select_best_opponent_reply`.
- Searcher trait FROZEN — no signature changes.

### Future Conflict Analysis
- Stage 17 (Game Mode Variant Tuning): No conflict. NNUE integration is transparent — evaluator trait unchanged.
- Stage 19 (SIMD): AccumulatorStack inner loops are hot candidates for SIMD. No conflict — SIMD would replace the scalar loops in `add_feature`/`sub_feature`/`compute_perspective`.

### Unaccounted Concerns
- Opponent move selection still uses `BootstrapEvaluator` (~10us per call). If NNUE becomes very fast with SIMD, consider using NNUE for opponent ranking too.

### Reasoning & Methods
- AccumulatorStack ownership in searchers (not evaluator) because Evaluator trait is `&self`. Searchers have `&mut self`.
- MCTS elimination gap: `gs.apply_move()` goes through `GameState` which triggers king removal on checkmate. `push()` only knows about the move itself. Solution: check `move_result.eliminations` after each `apply_move()` and force `needs_refresh = [true; 4]`.
- MCTS pop-all pattern: track depth via move count (1 for root child + N for selection), pop that many times after backpropagation.

---

## Related

- Stage spec: [[stage_16_nnue_integration]]
- Downstream log: [[downstream_log_stage_16]]
