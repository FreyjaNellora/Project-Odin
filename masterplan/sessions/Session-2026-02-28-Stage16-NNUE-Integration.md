# Session — 2026-02-28 — Stage 16: NNUE Integration

## Summary

Wired `AccumulatorStack` push/pop into BRS and MCTS search paths for incremental NNUE evaluation, replacing the full-refresh-per-call approach from Stage 14 (W17). Added fallback to bootstrap eval when no `.onnue` file is provided.

## Key Decisions

1. **AccumulatorStack lives in searchers, not evaluator.** Evaluator trait is `&self` (immutable). Push/pop needs `&mut self`. Searchers have `&mut self`.
2. **Even in NNUE mode, both searchers keep `BootstrapEvaluator`.** Used for opponent move selection in free functions (`select_best_opponent_reply`, `select_hybrid_reply`, `pick_objectively_strongest`). NNUE is only at leaf eval.
3. **MCTS elimination-aware refresh.** `gs.apply_move()` can trigger eliminations (king removal) that `push()` doesn't know about. After each `apply_move()` with eliminations, force `needs_refresh = [true; 4]`.
4. **Null move pruning: no push/pop.** Only changes `side_to_move`, no piece movement. Accumulator remains valid.
5. **`Arc<NnueWeights>` shared between BRS and MCTS.** Single allocation, read-only after loading.

## Files Changed

### Engine Core
- `search/hybrid.rs` — NnueWeights loading in `new(profile, nnue_path)`
- `search/brs.rs` — AccumulatorStack in BrsSearcher+BrsContext, push/pop at 4 make/unmake sites, `nnue_eval_scalar()` helper, debug tracing
- `search/mcts.rs` — AccumulatorStack in MctsSearcher, simulation push/pop with elimination-aware refresh, NNUE leaf eval

### Tests
- `tests/stage_16_nnue_integration.rs` — 10 new tests (T1-T10)
- 7 existing test files updated for new constructor signatures

### Documentation
- `masterplan/audit_log_stage_16.md`
- `masterplan/downstream_log_stage_16.md`
- `masterplan/STATUS.md`
- `masterplan/HANDOFF.md`
- This session note

## Test Results

- Engine: 536 (305 unit + 231 integration, 6 ignored)
- Stage 16 new: 10 (T1-T10, all passing)
- Python: 8 (pytest)
- UI Vitest: 54
- Clippy: 0 warnings

## W-Notes

- W17 RESOLVED (incremental accumulator in BRS+MCTS)
- W23 NEW (opponent selection still uses bootstrap)
- W24 NEW (MCTS expansion doesn't track accumulator)
- W25 NEW (constructor signatures changed)

## Related

- Plan: [[squishy-discovering-bachman]]
- Audit: [[audit_log_stage_16]]
- Downstream: [[downstream_log_stage_16]]
- Stage spec: [[stage_16_nnue_integration]]
