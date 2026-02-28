# Downstream Log — Stage 11: Hybrid Integration

## Notes for Future Stages

### Must-Know

1. **Protocol now uses HybridController** — `protocol/mod.rs` creates `HybridController::new(profile)` instead of bare `BrsSearcher`. All `go` commands run BRS Phase 1 then MCTS Phase 2.
2. **Info output is two-phase** — BRS lines have `phase brs`, MCTS lines have `phase mcts`, plus `info string hybrid phase1 done ...` transition line. Any code parsing info output must handle both phases.
3. **BRS_MAX_DEPTH = 8** — When budget is depth-only, BRS Phase 1 runs at `min(depth, 8)`. With time budget, BRS gets 10-30% of total time depending on position type.
4. **Null move pruning has ply>0 guard** — Added to prevent root cutoff that would produce zero root_move_scores. Changes aspiration behavior slightly (~120cp variance on repeat searches).
5. **Time pressure threshold = 100ms** — If total time budget < 100ms, MCTS is skipped entirely and BRS gets the full budget.
6. **History table is per-search** — `BrsContext.history` is zeroed each search. `BrsSearcher.last_history` stores the most recent search's history. Not persistent across moves.

### API Contracts

| API | Signature | Notes |
|---|---|---|
| `HybridController::new` | `pub fn new(profile: EvalProfile) -> Self` | Creates both BrsSearcher + MctsSearcher internally. |
| `HybridController::set_info_callback` | `pub fn set_info_callback(&mut self, cb: Box<dyn FnMut(String)>)` | NOT on Searcher trait. Protocol calls directly. |
| `BrsSearcher::history_table` | `pub fn history_table(&self) -> Option<&HistoryTable>` | Returns last search's history. None before first search. |
| `BrsSearcher::root_move_scores` | `pub fn root_move_scores(&self) -> Option<&[(Move, i16)]>` | Scores from last completed depth. Clamped to ±9999. |
| `BrsSearcher::take_info_callback` | `pub fn take_info_callback(&mut self) -> Option<Box<dyn FnMut(String)>>` | Moves callback out. Used by HybridController. |
| `MctsSearcher::take_info_callback` | `pub fn take_info_callback(&mut self) -> Option<Box<dyn FnMut(String)>>` | Same pattern. |

### Known Limitations

- **W10:** BRS root_move_scores at depth 4 in 4-player only reflect 1 max_node layer (ply 0). For meaningful survivor filtering, BRS needs depth 5+ (second max_node at ply 4). Currently BRS_MAX_DEPTH=8 ensures this.
- **W11:** History table is sparse at low depths. In 4-player BRS, history updates only occur in max_node beta cutoffs (ply 0, 4, 8...). Depth 8 produces non-zero entries; depth 4 may not.
- **W12:** Adaptive time split uses capture ratio only (no check detection). Position classification could be enriched with check count, material imbalance, etc.
- **W13:** MCTS score is 9999 (max) in some positions — this is the MCTS win/loss encoding, not a mate score. Hybrid returns this as-is.
- **W14:** `external_priors` consumed via `take()` — one-shot per search. If MCTS is called twice without new priors, second call uses MVV-LVA defaults.

### Performance Baselines

| Metric | Value | Notes |
|---|---|---|
| Hybrid `go depth 8` (debug, starting pos) | ~10s | BRS depth 8 (~4s) + MCTS 2000 sims (~6s). |
| BRS Phase 1 depth 8 (debug) | ~4s / ~15k nodes | Starting position. |
| MCTS Phase 2 2000 sims (debug) | ~5-6s / ~2k nodes | ~350 sims/sec debug. |
| Hybrid `go depth 4` (debug) | ~5s | BRS depth 4 (~50ms) + MCTS 2000 sims (~5s). |
| Hybrid time_budget(2000ms) quiet | BRS ~200ms + MCTS ~1800ms | 10/90 split for quiet positions. |
| Hybrid time_budget(2000ms) tactical | BRS ~600ms + MCTS ~1400ms | 30/70 split for tactical positions. |
| Test count | 457 | 281 unit + 176 integration (4 ignored). +17 new Stage 11. |

### Open Questions

1. **Should history persist across moves?** Currently reset per search. Stage 13 could experiment with partial persistence (decayed history between moves).
2. **Adaptive MCTS_DEFAULT_SIMS:** Currently fixed at 2000 for depth-only budgets. Could scale with available processing power or position complexity.
3. **BRS score capping for survivors:** Root_move_scores clamped to ±9999. If all moves score near 9999 (phantom mates), survivor filtering degrades. May need BRS_SCORE_CAP adjustment.
4. **MCTS prior blending:** Currently external priors fully REPLACE MVV-LVA. Could blend (e.g., 70% BRS prior + 30% MVV-LVA) for robustness.

### Reasoning

- **Concrete `HybridController` in protocol** (not `Box<dyn Searcher>`): Avoids modifying frozen Searcher trait. `set_info_callback` is not on the trait.
- **`take_info_callback` pattern:** Moves callback ownership between sub-searchers cleanly. No cloning of `FnMut` closures.
- **Softmax over survivors only:** Non-survivors get prior 0.0. MCTS's `max(1e-10)` guard prevents division by zero. Avoids softmax over large score ranges.
- **Position classification by capture ratio:** Simple, cheap, effective. Captures are already identified in move generation (is_capture flag). No extra computation.
- **BRS_MAX_DEPTH = 8:** User-directed. Extra ~70ms is worth accurate survivor filtering. 15% time budget accommodates it.

---

## Related

- Stage spec: [[stage_11_hybrid_integration]]
- Audit log: [[audit_log_stage_11]]
