# Audit Log — Stage 10: MCTS Strategic Search

## Pre-Audit
**Date:** 2026-02-27
**Auditor:** Claude Opus 4.6 (Stage 10 cleanup session)

### Build State
- Compiles: Yes — `cargo build --release` passes, 0 warnings, 0 clippy warnings.
- Tests pass: Yes — 440 total (281 unit + 159 integration, 4 ignored), 0 failures. 54 UI Vitest.
- Previous downstream flags reviewed: [[downstream_log_stage_06]] (direct dependency), [[downstream_log_stage_09]] (carries W4-W6 forward).

### Findings

**From [[downstream_log_stage_06]] (direct dependency — Evaluator trait):**
1. `eval_scalar(position, player) -> i16`: MCTS does not call `eval_scalar` directly. Uses `eval_4vec(&GameState) -> [f64; 4]` (all four players at once, sigmoid-normalized). Confirmed `eval_4vec` exists on `Evaluator` trait.
2. `PIECE_EVAL_VALUES: [i16; 7]` used in `compute_priors()` for MVV-LVA scoring. Indices verified: Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4, King=5, Promoted=6.
3. `SIGMOID_K: f64 = 400.0` — MCTS uses `q_to_centipawns` with same 400.0 constant for inverse sigmoid. Consistent.

**From [[downstream_log_stage_09]] (BRS + TT — not a dependency, but carries forward):**
1. W5 (stale GameState fields during search): Not relevant — MCTS clones GameState per simulation.
2. W4 (lead penalty tactical mismatch): MCTS uses `EvalProfile::Aggressive` in tests. Consistent with convention.
3. W6 (simplified SEE): Not relevant — MCTS does not use SEE.
4. History table format `[[[i32; 196]; 7]; 4]`: Stage 11 stub `set_history_table()` uses matching `HistoryTable` type alias. Verified.

**Active issues reviewed:**
- `Issue-Pawn-Push-Preference-King-Walk` (WARNING): MITIGATED — eval-side fixes applied. MCTS provides alternative search strategy.
- `Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch` (NOTE): Still open, not blocking.
- `Issue-DKW-Halfmove-Clock` (NOTE): Not affected.
- `Issue-Vec-Clone-Cost-Pre-MCTS` (WARNING): RESOLVED — fixed-size piece_lists and Arc<Vec> position_history implemented pre-Stage 10.

### Risks for This Stage

1. **GameState clone per simulation (INFO):** Each simulation clones `GameState` and replays moves from root. With fixed-size data structures (post Vec-clone-retrofit), clone is O(1) heap allocation (just the Arc bump). 1000 sims in 124ms release confirms acceptable cost.

2. **Nested Vec<MctsNode> tree memory (WARNING):** Each `MctsNode` contains `children: Vec<MctsNode>`. Deep trees with many children could consume significant memory. With progressive widening (PW_W=2.0, PW_B=0.5), internal nodes limited to ~floor(2*sqrt(N)) selectable children. At 1000 sims, this is manageable. May need arena allocation in Stage 19 if profiling warrants.

3. **PRNG quality for Gumbel noise (INFO):** SplitMix64 is a well-known high-quality PRNG but is NOT cryptographically secure. For game search purposes, quality is more than sufficient. Tested: 10,000 samples all in valid range.

4. **Sequential Halving budget fragmentation (INFO):** With small budgets (e.g., 2-10 sims), halving rounds may not distribute evenly. Handled by post-halving cleanup loop that runs remaining budget on unvisited candidates.

---

## Post-Audit
**Date:** 2026-02-27
**Auditor:** Claude Opus 4.6 (Stage 10 cleanup session)

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| MCTS node struct (MctsNode) | ✓ Complete | visit_count, value_sum[4], prior, gumbel, children Vec, terminal/expanded flags, total_children |
| Gumbel-Top-k root selection | ✓ Complete | `g(a) + log(pi(a))` ranking, `top_k_selection()` returns indices |
| Sequential Halving | ✓ Complete | `ceil(log2(k))` rounds, bottom half eliminated by `sigma(g + log(pi) + Q)` |
| Non-root tree policy (PUCT) | ✓ Complete | `Q/N + C*pi/(1+N) + PH(a)`, unvisited=+1e6 priority |
| Prior policy (softmax over MVV-LVA) | ✓ Complete | `compute_priors()` with temperature, captures > quiets |
| 4-player MaxN backpropagation | ✓ Complete | All 4 values propagate unchanged, no negation |
| Progressive widening | ✓ Complete | `floor(W * N^B)`, non-root only, minimum 2 |
| Leaf evaluation via `eval_4vec` | ✓ Complete | Clone → apply → evaluate, sigmoid-normalized [f64; 4] |
| Simulation budget control | ✓ Complete | max_nodes and max_time_ms, time checked every 64 sims |
| PV extraction | ✓ Complete | Most-visited path from selected root child |
| Temperature selection | ✓ Complete | 0.0=deterministic, >0=N^(1/T) sampling |
| `MctsSearcher` implements `Searcher` | ✓ Complete | `Box<dyn Searcher>` works (test_mcts_as_box_dyn_searcher) |
| Stage 11 stubs | ✓ Complete | `set_prior_policy()`, `set_history_table()`, `HistoryTable` type alias |
| SplitMix64 PRNG | ✓ Complete | Embedded, no `rand` dependency, configurable seed |
| 440 tests pass | ✓ | 281 unit + 159 integration, 4 ignored, 0 failures |

Acceptance criteria:
- [x] AC1: 2 sims finds reasonable move (`test_search_2_sims_returns_legal_move`) ✓
- [x] AC2: 100+ sims returns legal moves with bounded scores (`test_search_100_sims_returns_legal_move`) ✓
- [x] AC3: 4-player value backprop correct (`test_search_scores_are_bounded`, `test_search_depth_positive`, unit tests) ✓
- [x] AC4: Progressive widening limits breadth (`test_progressive_widening_limits_breadth`, `test_pw_limit_grows_with_visits` unit test) ✓
- [x] AC5: 1000 sims < 5s release — **124ms** (`test_1000_sims_under_5_seconds_release`, ignored in debug) ✓
- [x] AC6: `MctsSearcher` implements `Searcher` (`test_mcts_as_box_dyn_searcher`) ✓
- [x] AC7: Progressive history API works (`test_progressive_history_changes_behavior`) ✓
- [x] AC8: Sequential Halving correctly eliminates (`test_sequential_halving_allocates_budget`) ✓

### Code Quality
#### Uniformity
- Naming follows existing `snake_case` convention: `visit_count`, `value_sum`, `prior_temperature`, `top_k_selection`, `pw_limit`. Constants follow `SCREAMING_SNAKE`: `MCTS_TOP_K`, `MCTS_C_PRIOR`, `MCTS_SCORE_CAP`.
- `TOTAL_SQUARES`, `PIECE_TYPE_COUNT`, `PLAYER_COUNT` constants match existing definitions in `brs.rs`.
- `SearchResult` fields populated consistently: `best_move`, `score`, `depth`, `nodes`, `pv` — same structure as BRS.
- Info line format follows BRS pattern: `info depth {} score cp {} ... nodes {} nps {} time {} pv {} phase mcts`.

#### Bloat
- `MctsNode.children: Vec<MctsNode>` — nested Vecs for tree structure. Simple and correct. Arena allocation deferred to Stage 19 if profiling warrants.
- `compute_priors()` allocates a `Vec<f64>` (scores) and `Vec<f32>` (result). Called once per expansion, not a hot path concern.
- `expand_node()` creates indexed pairs for sorting — one-time allocation per expansion.
- No unnecessary abstractions. Single file (~1030 lines including 200+ lines of unit tests).

#### Efficiency
- Simulation cost: O(depth) GameState clones + apply_move per sim. 1000 sims / 124ms = ~8K sims/sec in release. Acceptable for 50-500 sim budgets in hybrid mode.
- `select_child_idx()` is O(selectable_children) per selection step — linear scan is fine for PW-limited child counts.
- `softmax()` uses max-subtraction for numerical stability. One pass for max, one for exp, one for normalize.
- `q_to_centipawns()` called once per info emission and once at search end. Negligible.

#### Dead Code
- None. All functions are called. `external_priors` field in MctsSearcher is set by `set_prior_policy()` stub but not yet read — this is intentional for Stage 11 readiness, not dead code.

#### Broken Code
- None. All 440 tests pass, 0 warnings. Deterministic reproducibility verified (`test_deterministic_with_same_seed`). PV well-formedness verified (`test_pv_well_formed` replays entire PV checking legality at each ply).

#### Temporary Code
- `external_priors: Option<Vec<f32>>` is stored but not used in search logic. Stub for Stage 11/16. Documented in the field comment.
- Single-legal-move early return at search start returns `score: 0` — acceptable since there is no choice to evaluate.

### Search/Eval Integrity

**Gumbel MCTS produces sane moves:** `test_finds_free_queen_capture` verifies that with a free queen hanging, MCTS returns a positive score after 200 sims. Prior policy correctly weights captures higher (MVV-LVA scoring + softmax).

**4-player value propagation (MaxN):** `backpropagate()` adds all 4 components unchanged. No negation, no min/max selection — each node accumulates raw sums. Player perspective is applied at selection time via `q_value(player_idx)` and at scoring time via `q_to_centipawns(q_value(root_player_idx))`. Verified by unit test `test_q_value_after_updates`.

**Score conversion consistency:** `q_to_centipawns` uses `400.0 * ln(q/(1-q))` with SIGMOID_K=400.0 matching the evaluator. Clamped to ±9999 matching BRS_SCORE_CAP. Edge cases handled: q<=0.001 → -9999, q>=0.999 → +9999.

**Position immutability:** `test_search_does_not_modify_position` verifies the input GameState is unchanged after search (current_player, game_over, zobrist all preserved).

**No interaction with BRS/TT:** MCTS is standalone. No TT access, no shared state with BrsSearcher. Clean separation confirmed by code inspection — `mcts.rs` does not import anything from `search/brs.rs` or `search/tt.rs`.

### Future Conflict Analysis

1. **Stage 11 (Hybrid Integration):** MCTS exposes `set_prior_policy(&mut self, priors: &[f32])` and `set_history_table(&mut self, history: &HistoryTable)`. `HistoryTable` type alias matches BRS format `[[[i32; 196]; 7]; 4]`. The HybridController will: (a) run BRS Phase 1, (b) extract history table, (c) pass to MctsSearcher, (d) run MCTS Phase 2. `external_priors` is stored but not yet consumed — Stage 11 or 16 must wire it into `expand_node()` or `compute_priors()`.

2. **Stage 12 (Self-Play & Regression):** MCTS supports temperature-based move selection (`temperature > 0.0` → probabilistic). Self-play can use `temperature = 1.0` for exploration and `temperature = 0.0` for evaluation games. No conflict.

3. **Stage 13 (Time Management):** MCTS respects `max_time_ms` in budget. Time management will set this budget based on game clock. No conflict. Persistent tree (reusing MCTS tree between moves) mentioned in MASTERPLAN as a Stage 13 measurement — tree structure supports this (root's children become new root after opponent moves).

4. **Stage 16 (NNUE Integration):** NNUE will replace `compute_priors()` with neural policy head output via `set_prior_policy()`. The `external_priors` field is ready. NNUE will also replace `eval_4vec()` as the leaf evaluator — the Evaluator trait abstraction handles this cleanly.

5. **Stage 19 (Optimization):** Arena allocation for MctsNode tree could replace nested Vecs. The `run_simulation → expand_node → backpropagate` loop would need index-based access instead of reference-based. Clean refactor path.

### Unaccounted Concerns

1. **`expand_node()` applies moves to GameState but children don't store the resulting state.** Each simulation replays from root. This is the documented design choice (D2: No GameState in nodes). At ~1000 sims it's fine (124ms). At 10K+ sims (Stage 19 optimization territory) this replay cost may become the bottleneck.

2. **`is_multiple_of` for time check interval** — uses nightly-like syntax `total_sims_done.is_multiple_of(64)`. Verified this compiles on stable Rust. If it doesn't compile on some toolchains, replace with `total_sims_done % 64 == 0`.

### Reasoning & Methods

Post-audit conducted by reading the full `mcts.rs` source (1030 lines), all 18 integration tests in `stage_10_mcts.rs`, and 14 unit tests in `mcts.rs::tests`. Cross-referenced against MASTERPLAN Stage 10 spec (lines 825-904) for completeness of deliverables and acceptance criteria. Verified API surface matches Stage 11 stubs documented in HANDOFF.md. Checked imports to confirm no coupling with BRS/TT modules. Reviewed game output from first post-Stage-10 game (13 rounds, `phase brs` only — confirms MCTS is standalone and not wired into protocol, as designed).

---

## Related

- Stage spec: [[stage_10_mcts]]
- Downstream log: [[downstream_log_stage_10]]
