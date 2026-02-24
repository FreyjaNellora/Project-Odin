# Audit Log — Stage 08: BRS/Paranoid Hybrid Layer

## Pre-Audit
**Date:** 2026-02-23
**Auditor:** Claude Opus 4.6 (Stage 8 implementation session)

### Build State
- Compiles: Yes — `cargo build --release` passes, 0 warnings.
- Tests pass: Yes — 361 total (233 unit + 128 integration), 3 ignored, 0 failures.
- Previous downstream flags reviewed: Stages 0, 1, 2, 3, 6, 7 (full dependency chain per MASTERPLAN Appendix A).

### Findings

**From [[downstream_log_stage_07]] (immediate upstream):**
1. BRS uses natural turn order R→B→Y→G (ADR-012). Hybrid reply scoring at MIN nodes must not alter side_to_move between make and unmake. Confirmed: `select_hybrid_reply` makes eval calls within make/unmake pairs, restoring cleanly.
2. Non-board GameState fields (player_status, scores) are stale during search. Board scanner runs pre-search with accurate data. Eval during search reads root-position scores/statuses. Acceptable for Stage 8.
3. `TIME_CHECK_INTERVAL = 1024`. Node budgets can overrun by up to 1024 nodes. Stage 8 does not change this.
4. Searcher trait is frozen: `search(&mut self, &GameState, SearchBudget) -> SearchResult`. Not modified.
5. Evaluator trait is frozen: `eval_scalar(&self, &GameState, Player) -> i16`. Not modified.
6. Bootstrap eval lead-penalty causes tactical mismatch (W4). Addressed by using Aggressive profile (no lead penalty) for FFA tactical tests.
7. One `GameState::clone()` per `go` command — unchanged, no regression.

**From [[downstream_log_stage_06]] (eval):**
1. `BootstrapEvaluator::new(profile: EvalProfile)` now takes a profile parameter (Step 0 of Stage 8). All call sites updated.
2. Evaluator trait is permanent. Stage 8 adds no new trait methods.

**Active issues reviewed (per AGENT_CONDUCT 1.9):**
- `[[Issue-Vec-Clone-Cost-Pre-MCTS]]` (WARNING): Still relevant. Stage 8 does not worsen it.
- `[[Issue-Huginn-Gates-Unwired]]` (NOTE): Resolved. Huginn retired, replaced by `tracing` crate (ADR-015).
- `[[Issue-DKW-Halfmove-Clock]]` (NOTE): Not affected.
- `[[Issue-DKW-Invisible-Moves-UI]]` (NOTE): Not affected.

### Risks for This Stage

1. **Eval blind to opponent captures (AGENT_CONDUCT 2.26 — Semantic Correctness):** `material_score(board, player)` only counts the player's own pieces. Capturing an opponent's rook has zero impact on the evaluating player's score. Risk: engine has no incentive to capture. Mitigation: add `relative_material_advantage()` to eval.

2. **Board scanner reads stale GameState during search (AGENT_CONDUCT 2.6):** Scanner runs pre-search, so its data is accurate. During search, board mutations change the tactical landscape but scanner data is frozen. Risk: hybrid scoring uses stale context deep in the tree. Mitigation: acceptable for v1; delta updater deferred to v2.

3. **Progressive narrowing over-prunes (AGENT_CONDUCT 4.5):** Aggressive narrowing at deep depths might discard the actual best opponent reply. Risk: search quality degrades. Mitigation: conservative limits (10/6/3), cheap presort keeps captures first.

4. **Lead penalty + FFA tactical conflict (W4):** Standard profile may cause engine to avoid captures. Risk: tactical tests fail. Mitigation: use Aggressive profile (no lead penalty) for FFA tactical positions.

---

## Post-Audit
**Date:** 2026-02-23
**Auditor:** Claude Opus 4.6 (same session as implementation)

### Deliverables Check

| Deliverable | Status | Notes |
|-------------|--------|-------|
| `search/board_scanner.rs` — BoardContext, OpponentProfile, scan_board | Complete | Pre-search analysis, < 1ms release |
| `search/board_scanner.rs` — MoveClass, classify_move, classify_moves | Complete | Cheap filter at MIN nodes |
| `search/board_scanner.rs` — select_hybrid_reply, score_reply | Complete | Hybrid formula: harm * likelihood + strength * (1-likelihood) |
| `search/board_scanner.rs` — narrowing_limit, cheap_presort | Complete | Depth-based candidate limits: 10/6/3 |
| `search/brs.rs` — min_node uses select_hybrid_reply | Complete | Replaces select_best_opponent_reply at MIN nodes |
| `eval/material.rs` — relative_material_advantage | Complete | Rewards material superiority vs opponents |
| `eval/mod.rs` — rel_mat wired into eval_for_player | Complete | Added as 7th eval component |
| `gamestate/mod.rs` — GameMode::LastKingStanding, constructors | Complete | (Step 0) |
| `eval/mod.rs` — EvalProfile, EvalWeights, BootstrapEvaluator config | Complete | (Step 0) |
| `protocol/types.rs` — EngineOptions game_mode, eval_profile | Complete | (Step 0) |
| `protocol/mod.rs` — setoption parsing for gamemode/evalprofile | Complete | (Step 0) |
| UI: GameControls selectors, config tags, newGame() rewrite | Complete | (Step 0b) |
| `tests/stage_08_brs_hybrid.rs` — 23 integration tests (+1 ignored) | Complete | Tactical + smoke-play + narrowing |
| Audit log + downstream log | Complete | This document |

**Test count:** 361 total (233 unit + 128 integration). 3 ignored. 0 failures.

### Code Quality

#### Uniformity
All new code follows project conventions: `snake_case`, `// comment` style, no `unwrap()` in production paths. `BoardContext`, `OpponentProfile`, and `ScoredReply` follow naming conventions of `GameState`, `PlayerStatus`, and `SearchResult`. The hybrid reply scoring integrates naturally into `min_node` alongside the existing `select_best_opponent_reply` (which remains for quiescence MIN nodes).

#### Bloat
`BoardContext` is created once per search call (in `BrsContext::new`). It contains 3 `OpponentProfile` structs and 8 high-value target slots — all fixed-size, no heap allocation. `select_hybrid_reply` creates small `Vec`s for relevant moves and scored replies per MIN node invocation. These are proportional to the number of legal moves (~20-40), not the tree size.

The `pseudo_pick` helper function in integration tests is minimal (5 lines, deterministic). No `rand` dependency added.

#### Efficiency
- **Board scanner:** < 1ms per call in release build (measured at < 10ms even in debug). Uses `is_square_attacked_by` for king danger and aggression calculations — same attack query API as move generation.
- **Progressive narrowing:** ~49% node reduction at depth 6 (10,916 → 5,519), ~46% at depth 8 (31,896 → 17,315). Well above the 20% reduction target.
- **Hybrid scoring overhead:** One `eval_scalar` call per relevant candidate move at each MIN node. With progressive narrowing limiting candidates to 3-10 moves, overhead is bounded.
- **`relative_material_advantage`:** Iterates all 4 players' piece lists once. Negligible cost (< 1% of eval time).

#### Dead Code
- `select_best_opponent_reply` remains in `brs.rs` — used by quiescence MIN nodes. Not dead code.
- `ScoredReply` struct fields (`objective_strength`, `harm_to_root`, `likelihood`) are populated but only `hybrid_score` and `mv` are used for selection. The other fields exist for future tracing instrumentation and debugging. Acceptable.

#### Broken Code
**Pre-audit risks confirmed resolved:**

1. **Eval blind to opponent captures (Risk 1):** Fixed. `relative_material_advantage()` added to `eval/material.rs`. Rewards material superiority vs active opponents. The knight fork test now finds `f3e5` at depth 5 (score 408 vs 358 for king moves). Weight: advantage / 4, clamped to ±500cp.

2. **Stale board context during search (Risk 2):** Confirmed acceptable. Board scanner runs once pre-search. During search, `select_hybrid_reply` re-classifies opponent moves using the current board state (not cached). The `BoardContext` fields (aggression, danger, targets) are stale but provide directional guidance — not precise values. Delta updater deferred to v2.

3. **Progressive narrowing over-prunes (Risk 3):** Not observed. All tactical tests pass. `cheap_presort` ensures captures are evaluated first. At depth 7+, only 3 candidates are kept but the presort guarantees the most valuable captures survive.

4. **Lead penalty tactical conflict (Risk 4):** Addressed. Tactical tests (capture, fork) use `EvalProfile::Aggressive`. Separate tests verify both Standard and Aggressive profiles produce valid search results.

#### Temporary Code
- `hybrid_depth_progression_analysis` is `#[ignore]` — permanent analysis tool, not temporary.
- No temporary `println!` or debug instrumentation in production code.

### Search/Eval Integrity

**Relative material advantage correctness:** At starting position (all players equal), `relative_material_advantage` returns 0 for all players. When Blue loses a queen (900cp), Red's advantage = (4300 - (4300+3400+4300)/3) / 4 = (4300 - 4000) / 4 = 75cp. Verified by unit tests.

**Hybrid scoring formula:** `score = harm_to_root * likelihood + objective_strength * (1 - likelihood)`. At likelihood = 1.0 (opponent certainly targets root), score = harm_to_root (pure pessimism). At likelihood = 0.0, score = objective_strength (BRS-like best reply). The blend produces realistic opponent modeling.

**Fork detection:** Engine finds knight fork `f3e5` at depth 5 with Aggressive profile. At depth 4, BRS with 4 players only gives Red turns at depth 4 and depth 0 — not enough to see fork follow-through. Depth 5 is the minimum for 4-player fork detection.

**Smoke-play validation:** 10 games (5 FFA + 5 LKS), 20 moves each, engine as Red at depth 4 with pseudo-random opponents. No panics, no illegal moves. Engine consistently produces legal moves within budget.

### Future Conflict Analysis

**Stage 9 (TT & Move Ordering):** Board scanner data could inform move ordering heuristics. `BoardContext::high_value_targets` identifies pieces the engine should consider capturing. TT will store hash → (score, depth, best_move) which is independent of hybrid scoring.

**Stage 10 (MCTS):** MCTS `MctsSearcher` will implement `Searcher` trait. Board scanner could provide playout policy hints. The `relative_material_advantage` eval component benefits MCTS rollouts as well.

**Stage 11 (Hybrid controller):** The hybrid controller will compose BRS + MCTS through the `Searcher` trait. Board scanner runs once per search, shared by both engines.

### Unaccounted Concerns

1. **Stale player_status during search (W5 — carried from Stage 7):** If a player is eliminated mid-search-tree, the cloned GameState still shows them as Active. `select_hybrid_reply` reads the board for classification (correct) but `BoardContext.per_opponent` profiles may reference eliminated players. The likelihood calculation tolerates this — an eliminated player's aggression is 0.0 anyway. **RATING: INFO** — no functional impact.

2. **FFA points not considered during search (INFO):** Score differences between players are read pre-search by the board scanner. During search, captures change material but FFA scores are not updated (W5). The `ffa_points_eval` component in eval reads root-position scores. **RATING: INFO** — acceptable for bootstrap eval.

3. **Quiescence MIN nodes use old `select_best_opponent_reply` (INFO):** The hybrid scoring is only applied at main-search MIN nodes, not quiescence MIN nodes. Quiescence MIN nodes still use plain "pick move that minimizes root eval." This is intentional — quiescence is captures-only with a small set, so hybrid likelihood adds no value. **RATING: INFO** — by design.

### Reasoning & Methods
Audit conducted by reading all modified engine source files (`board_scanner.rs`, `brs.rs`, `material.rs`, `mod.rs`), running the full test suite (361 tests), and analyzing node count reduction data. Tactical correctness verified through 6 purpose-built positions (capture, fork, defense, quiet, trap). Performance validated via smoke-play across both game modes.

---

## Related

- Stage spec: [[stage_08_brs_hybrid]]
- Downstream log: [[downstream_log_stage_08]]
