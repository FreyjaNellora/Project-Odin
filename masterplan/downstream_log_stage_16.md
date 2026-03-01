# Downstream Log — Stage 16: NNUE Integration

## Notes for Future Stages

### Must-Know

- **W17 RESOLVED:** AccumulatorStack is now incrementally updated in BRS and MCTS search. Push before make/unmake, pop after. No more full refresh per eval call.
- **W18 (carried):** King moves still mark `needs_refresh` even without king bucketing. Profile in Stage 19.
- **W19 (carried):** EP/castling fall back to full refresh. Profile in Stage 19.
- **W20 (carried):** `serde` + `serde_json` only in datagen CLI path. Do NOT import serde in eval/search hot path.
- **W23 (new):** Opponent move selection (`select_best_opponent_reply`, `select_hybrid_reply`, `pick_objectively_strongest`) still uses `BootstrapEvaluator`, not NNUE. By design — NNUE is only used at leaf eval nodes in the search tree.
- **W24 (new):** MCTS root expansion uses `child_gs.apply_move()` on clones without accumulator tracking. Correct by design — expansion creates tree structure, not simulation path.
- **W25 (new):** Constructor signatures changed: `BrsSearcher::new()`, `MctsSearcher::new()`, `with_seed()`, `with_info_callback()`, `HybridController::new()` all accept `nnue_weights`/`nnue_path` parameter. Existing callers pass `None` for bootstrap-only mode.

### API Contracts

- **`BrsSearcher::new(evaluator, nnue_weights: Option<Arc<NnueWeights>>)`** — creates BRS searcher with optional NNUE. If `Some`, creates `AccumulatorStack` and uses NNUE at leaf eval.
- **`MctsSearcher::new(evaluator, nnue_weights: Option<Arc<NnueWeights>>)`** — same pattern for MCTS.
- **`MctsSearcher::with_seed(evaluator, nnue_weights, seed)`** — deterministic constructor with NNUE option.
- **`MctsSearcher::with_info_callback(evaluator, nnue_weights, cb)`** — callback constructor with NNUE option.
- **`HybridController::new(profile, nnue_path: Option<&str>)`** — loads `.onnue` from disk, creates `Arc<NnueWeights>` shared between BRS and MCTS.
- **`EngineOptions::nnue_file: Option<String>`** — stores the .onnue path in engine config. Set via `setoption name NnueFile value <path>`. Changes invalidate the current searcher (recreated on next `go`).
- **BrsContext::nnue_eval_scalar(player)** — internal helper. Uses NNUE if available, falls back to bootstrap.

### Key Constants

| Constant | Value | Location |
|----------|-------|----------|
| `AccumulatorStack push/pop` | 4 sites in BRS, 2 in MCTS simulation | brs.rs, mcts.rs |
| `NNUE eval calls` | 3 in BRS (root seed, info line, qsearch stand-pat) | brs.rs |
| `MCTS leaf eval` | 1 site (forward_pass replaces eval_4vec) | mcts.rs |

### Known Limitations

- **W13 (carried):** MCTS score 9999 (max) in some positions — unchanged.
- **Pondering not implemented:** Deferred from Stage 13.
- **No SIMD:** Stage 19 target. Accumulator inner loops are hot candidates.
- **Random weights only.** Gen-0 pipeline from Stage 15 must be run to produce trained weights. Random weights produce random play quality.
- **Elimination-aware refresh is conservative:** After any `gs.apply_move()` with eliminations, ALL 4 perspectives are marked for refresh. Could be optimized to only refresh affected perspectives, but eliminations are rare enough that this is negligible.

### Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| BRS depth 6 with NNUE (random weights) | ~comparable to bootstrap | Debug build, debug assertions active |
| Incremental push+forward | Faster than full init+forward | Verified by T10 |
| MCTS 500 sims with NNUE | Completes without panic | T7 |
| 10 self-play games × 5 ply | No panics | T9 |

### Open Questions

- **Gen-0 trained weights quality:** Random weights produce random play. How much does NNUE improve play over bootstrap eval after Gen-0 training? Need A/B self-play tournament (T11, human-driven).
- **NNUE for opponent ranking:** Currently opponent moves are ranked by bootstrap eval. If NNUE becomes fast enough (post-SIMD), should opponent ranking use NNUE too?

### Reasoning

- AccumulatorStack ownership in searchers (not evaluator): Evaluator trait is `&self`. Push/pop needs `&mut`. Searchers have `&mut self`.
- MCTS elimination gap: `gs.apply_move()` can trigger eliminations (king removal) that `push()` doesn't track. Force `needs_refresh = [true; 4]` after any apply_move with eliminations.
- Null move pruning: No push/pop. It only changes `side_to_move`, no piece moves. Accumulator remains valid. `forward_pass()` takes `player: Player` explicitly.
- `Arc<NnueWeights>` shared between BRS and MCTS: Weights are read-only after loading. Single allocation, shared reference.

---

## Related

- Stage spec: [[stage_16_nnue_integration]]
- Audit log: [[audit_log_stage_16]]
