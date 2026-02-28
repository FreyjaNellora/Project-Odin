# Downstream Log — Stage 10: MCTS Strategic Search

**Date:** 2026-02-27
**Author:** Claude Opus 4.6 (Stage 10 cleanup session)

## Notes for Future Stages

### Must-Know

1. **MCTS is standalone — not wired into the protocol.** The `go` command in `protocol/mod.rs` creates a `BrsSearcher`. `MctsSearcher` is only reachable via direct Rust instantiation (tests) or through Stage 11's HybridController. Do NOT add MCTS to the protocol handler — Stage 11 creates the `HybridController` which composes both searchers through the `Searcher` trait.

2. **Tree is rebuilt every search.** No persistent tree between `search()` calls. Each call creates a fresh root, expands, simulates, returns. Stage 13 may measure persistent-tree benefits (reuse subtree after opponent move).

3. **`external_priors` is stored but NOT consumed.** `set_prior_policy(&mut self, priors: &[f32])` populates `self.external_priors` but the search loop still uses `compute_priors()` (MVV-LVA softmax). Stage 11 or Stage 16 must wire `external_priors` into the expansion logic, either replacing `compute_priors()` entirely or blending with it.

4. **Progressive widening at non-root only.** Root expands ALL legal moves (needed for Gumbel Top-k). Internal nodes limit selectable children via `pw_limit(N, W=2.0, B=0.5) = floor(2 * sqrt(N))`, minimum 2. All children are CREATED at expansion but only a subset are SELECTABLE.

5. **Gumbel noise is sampled once at root expansion.** Each root child gets a permanent `gumbel` value. This noise is used in Top-k selection and Sequential Halving scoring. It is NOT re-sampled. Non-root nodes have `gumbel = 0.0` (unused).

6. **Score conversion: `q_to_centipawns` uses SIGMOID_K=400.** `cp = 400 * ln(q/(1-q))`, clamped to ±9999. This matches the evaluator's sigmoid. To convert centipawns back to Q-values: `q = 1/(1 + exp(-cp/400))`.

7. **SplitMix64 PRNG is embedded in `mcts.rs`.** `pub(crate)` visibility. If another module needs deterministic randomness, import it or extract to a shared utility. Do NOT add `rand` as a dependency.

8. **Budget uses `max_nodes` for simulation count.** `SearchBudget.max_nodes` is interpreted as maximum number of simulations (not tree nodes). Each simulation creates/visits one path. `max_time_ms` is checked every 64 sims.

### API Contracts

**MctsSearcher (public, `search/mcts.rs`):**
```rust
pub struct MctsSearcher { /* fields private */ }

impl MctsSearcher {
    pub fn new(evaluator: Box<dyn Evaluator>) -> Self
    pub fn with_info_callback(evaluator: Box<dyn Evaluator>, cb: Box<dyn FnMut(String)>) -> Self
    pub fn set_info_callback(&mut self, cb: Box<dyn FnMut(String)>)
    pub fn with_seed(evaluator: Box<dyn Evaluator>, seed: u64) -> Self
    pub fn set_prior_policy(&mut self, priors: &[f32])          // Stage 11/16 stub
    pub fn set_history_table(&mut self, history: &HistoryTable)  // Stage 11 stub
}

impl Searcher for MctsSearcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult;
}
```

**HistoryTable type alias (public, `search/mcts.rs`):**
```rust
pub type HistoryTable = [[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT];
// TOTAL_SQUARES = 196, PIECE_TYPE_COUNT = 7, PLAYER_COUNT = 4
// Indexed: history[player_idx][piece_type_idx][to_square]
// Matches BRS history table format exactly.
```

**Info line format (emitted via callback):**
```
info depth {max_depth} score cp {centipawns} v1 {f64} v2 {f64} v3 {f64} v4 {f64} nodes {sim_count} nps {sims_per_sec} time {ms} pv {move_list} phase mcts round {current}/{total}
```
- `v1..v4`: Root Q-values for Red/Blue/Yellow/Green (sigmoid-normalized, 0.0-1.0).
- `round X/Y`: Sequential Halving progress.
- `phase mcts`: Distinguishes from BRS `phase brs` lines.

**Internal functions (pub(crate) — available within crate):**
```rust
pub(crate) struct SplitMix64 { /* ... */ }
pub(crate) struct MctsNode { /* ... */ }
pub(crate) fn compute_priors(moves: &[Move], temperature: f32) -> Vec<f32>
pub(crate) fn top_k_selection(children: &[MctsNode], k: usize) -> Vec<usize>
```

### Known Limitations

**W7 — Nested Vec tree structure (INFO):**
`MctsNode.children: Vec<MctsNode>` causes heap allocation per expansion. At 1000 sims this is fine (124ms). At 10K+ sims, allocation overhead may dominate. Arena-based allocation planned for Stage 19.

**W8 — No GameState in tree nodes (INFO):**
Each simulation replays moves from root: O(depth) `apply_move` calls per sim. Design choice D2 avoids ~1KB per node of stored state. At 1000 sims with avg depth 5, this is 5000 `apply_move` calls — negligible at 124ms total. May revisit if sim counts increase 10x+.

**W9 — MVV-LVA priors only (INFO):**
`compute_priors()` uses `softmax(victim_val - attacker_val/10 + 100)` for captures and `10.0` for quiets. This is a coarse prior that ranks captures above quiets but doesn't distinguish between quiet moves. NNUE policy head (Stage 16) will replace this with learned move probabilities.

**W5 — Stale GameState fields during search (carried from Stage 7):**
Not relevant to MCTS — each simulation clones a fresh GameState.

### Performance Baselines

| Metric | Value | Build | Notes |
|--------|-------|-------|-------|
| 1000 sims, starting position | 124ms, 986 nodes | Release | AC5: <5s ✓ |
| Best move at 1000 sims | e2e4 | Release | Reasonable opening |
| 2 sims, starting position | <1ms | Debug | Returns legal move (AC1) |
| 100 sims, starting position | ~10ms | Debug | Bounded scores (AC2) |
| Max depth reached (1000 sims) | ~5-8 | Release | Varies with PW/tree shape |
| Memory (1000 sims estimate) | <5MB | Release | Node struct ~100 bytes + children Vec |

### Open Questions

1. **Should MCTS persist its tree between searches?** Currently rebuilds from scratch each `search()` call. Stage 13 (Time Management) should measure whether reusing the subtree after opponent moves provides measurable sim efficiency. If yes, add `reuse_tree(&mut self, opponent_move: Move)` to MctsSearcher.

2. **External priors: replace or blend?** `set_prior_policy()` stores priors but they're unused. When wired in, should they completely replace `compute_priors()` output, or blend (e.g., `0.5 * NNUE_prior + 0.5 * MVV_LVA_prior`)? Stage 11 should decide for BRS-informed priors; Stage 16 for NNUE priors.

3. **PUCT C_PRIOR tuning.** Default `C_PRIOR = 1.5` is borrowed from AlphaZero. With coarse MVV-LVA priors (not NNUE), a higher value may encourage more exploration. Measure after Stage 11 integration to see if BRS history improves prior quality enough to lower C_PRIOR.

4. **PW parameters (W=2.0, B=0.5).** These control non-root branching. `floor(2*sqrt(N))` means a node with 100 visits can select from 20 children. Whether this is too conservative or too generous depends on the average branching factor in mid-game 4PC positions (~30-50 legal moves). Tune after Stage 11 hybrid runs.

### Reasoning

**D1: No `rand` crate.** Project philosophy is minimal dependencies. SplitMix64 is ~15 lines, high-quality, deterministic with seed. Avoids pulling in the `rand` ecosystem.

**D2: No GameState in nodes.** Storing GameState in every node (~1KB each) would make the tree 10x heavier. With progressive widening, a 1000-sim tree might have 500+ nodes — that's 500KB of state vs. replaying 5000 `apply_move` calls (which costs ~50ms in release). Replay wins until sim counts reach 10K+.

**D3: Nested Vec children.** Simple ownership model — no arena, no indices, no unsafe. Each node owns its children. Rust borrow checker is happy. Arena optimization deferred to Stage 19 because it adds complexity (index-based access, separate allocation pool) for marginal gain at current sim counts.

**D4: PW at non-root only.** Root must see ALL legal moves for Gumbel Top-k to work correctly — the noise sampling assumes the full action space. PW at internal nodes prevents exponential tree growth (4 players × ~30 moves each).

**D5: Score conversion via inverse sigmoid.** MCTS operates in [0,1] value space (sigmoid-normalized). Protocol and UI expect centipawns. `q_to_centipawns(q) = 400 * ln(q/(1-q))` is the exact inverse of the evaluator's sigmoid normalization. Clamped to ±9999 matching BRS_SCORE_CAP for consistent display.

---

## Related

- Stage spec: [[stage_10_mcts]]
- Audit log: [[audit_log_stage_10]]
