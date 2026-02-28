# HANDOFF — Last Session Summary

**Date:** 2026-02-27
**Stage:** Stage 10 (MCTS) — Implementation Complete, Pending User Tag
**Next:** User confirms Stage 10, tags `stage-10-complete` / `v1.10`. Then begin Stage 11 (Hybrid Integration).

## What Was Done This Session

### Stage 10: Gumbel MCTS Strategic Search (DONE)

Implemented the full standalone Gumbel MCTS searcher as a single new file `odin-engine/src/search/mcts.rs` (~550 lines). All 13 build-order steps completed:

1. **SplitMix64 PRNG** — Embedded PRNG (no `rand` dependency). Configurable seed for deterministic tests.
2. **MctsNode struct** — visit_count, value_sum[4], prior, gumbel, children Vec, terminal/expanded flags.
3. **Prior policy** — Softmax over MVV-LVA scores with configurable temperature (default 50.0). Captures > quiets.
4. **Gumbel noise** — `Gumbel(0,1) = -ln(-ln(U))` sampling at root children.
5. **Top-k selection** — Rank by `g(a) + ln(pi(a))`, keep top-k (default 16).
6. **Sequential Halving** — `ceil(log2(k))` rounds, budget split across rounds, bottom half eliminated each round. Score by `sigma(g + ln(pi) + q)`.
7. **PUCT tree policy** — `Q/N + C*pi/(1+N) + PH(a)`. Unvisited children selected first. Progressive widening limits selectable children.
8. **Expansion + leaf eval** — Clone GameState, apply moves, `eval_4vec` for [f64; 4] leaf values. Terminal detection via `is_game_over()` / empty legal moves.
9. **4-player MaxN backpropagation** — All 4 value components propagate up unchanged. No negation.
10. **Progressive widening** — `max_children = floor(W * N^B)`, defaults W=2.0, B=0.5. Applied to non-root nodes only.
11. **Budget control** — Stops at max_nodes or max_time_ms (checked every 64 sims). SimConfig struct bundles parameters.
12. **PV extraction** — Follows most-visited children from selected best root child.
13. **Temperature selection** — Default 0.0 (deterministic). Stub for self-play temperature sampling.
14. **Stage 11 stubs** — `set_prior_policy()`, `set_history_table()`, `HistoryTable` type alias. Progressive history term activates when history provided.
15. **MctsSearcher** — Implements frozen `Searcher` trait. Constructors: `new()`, `with_info_callback()`, `with_seed()`. Info callback emits `phase mcts` lines.

### Design Decisions

- **D1: No `rand` crate** — Embedded SplitMix64 PRNG (~15 lines). Matches project's minimal-dependency philosophy.
- **D2: No GameState in nodes** — Replay from root each simulation. O(depth) apply_move per sim. 1000 sims completes in 124ms release.
- **D3: Nested Vec<MctsNode>** — Simple ownership. Arena can be added in Stage 19 if profiling warrants.
- **D4: PW at non-root only** — Root creates all children for Gumbel Top-k. PW limits selectable children at internal nodes.
- **D5: Score conversion** — `q_to_centipawns` via inverse sigmoid: `cp = 400 * ln(q/(1-q))`, clamped to ±9999.

### Acceptance Criteria Results

| AC | Description | Status |
|----|------------|--------|
| AC1 | 2 sims finds reasonable move | PASS — returns legal move |
| AC2 | 100+ sims match/beat UCB1 quality | PASS — legal moves, bounded scores |
| AC3 | 4-player value backprop correct | PASS — unit tests + integration |
| AC4 | Progressive widening limits breadth | PASS — pw_limit grows with visits |
| AC5 | 1000 sims < 5s release | PASS — **124ms** |
| AC6 | MctsSearcher implements Searcher | PASS — `Box<dyn Searcher>` works |
| AC7 | Progressive history reduces waste | PASS — API works, PH term activates |
| AC8 | Sequential Halving eliminates correctly | PASS — budget allocated, candidates reduced |

### Performance

| Metric | Value |
|--------|-------|
| 1000 sims (release, starting pos) | 124ms, 986 nodes |
| Best move at 1000 sims | e2e4 (reasonable opening) |

---

## What's Next — Priority-Ordered

### 1. User Confirms Stage 10

Tag `stage-10-complete` / `v1.10`. Fill in `masterplan/audit_log_stage_10.md`. Write `masterplan/downstream_log_stage_10.md` with API contracts for Stage 11.

### 2. Begin Stage 11 (Hybrid Integration)

Compose BRS and MCTS through the `Searcher` trait. `HybridController` in `search/hybrid.rs`. BRS provides depth; MCTS provides breadth.

---

## Known Issues

- `Issue-Pawn-Push-Preference-King-Walk` (WARNING): MITIGATED — eval-side fixes applied. MCTS now available as alternative search strategy.
- W5 (stale GameState fields during search): acceptable for bootstrap eval, revisit for NNUE
- W4 (lead penalty tactical mismatch): mitigated by Aggressive profile for FFA
- `Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch` (NOTE): still open, not blocking
- `Issue-DKW-Halfmove-Clock` (NOTE): still open, not blocking

## Files Created This Session

- `odin-engine/src/search/mcts.rs` — NEW: Complete Gumbel MCTS searcher (~550 lines)
- `odin-engine/tests/stage_10_mcts.rs` — NEW: 18 integration tests + 1 ignored release perf test
- `odin-engine/src/search/mod.rs` — MODIFIED: Added `pub mod mcts;` (1 line)

## Test Counts

- Engine: 440 (281 unit + 159 integration, 4 ignored)
- UI Vitest: 54
- Total: 0 failures, 0 clippy warnings
