# Claude.T Prompt — Stage 10: MCTS Strategic Search (Gumbel MCTS)

You are building Stage 10 of Project Odin, a four-player chess engine. This stage adds a standalone MCTS searcher that implements the existing `Searcher` trait. It is NOT integrated with BRS yet — that's Stage 11.

---

## Before You Write Any Code

Follow AGENT_CONDUCT.md Section 1.1 (Stage Entry Protocol) exactly:

1. **Read `masterplan/STATUS.md`** — Project is post-Stage-9. 408 engine tests pass (267 unit + 141 integration, 3 ignored). 54 UI Vitest. Zero clippy warnings. Vec clone retrofit done (fixed-size piece_lists, Arc position_history).

2. **Read `masterplan/HANDOFF.md`** — Last session did pre-Stage-10 cleanup: audit fixes, eval mitigations (dev bonuses, pawn gate, king displacement), Vec clone cost resolved. User has approved Stage 10.

3. **Read `masterplan/AGENT_CONDUCT.md`** — Full agent behavior rules. Pay special attention to Section 1.2 (test-first), 1.4 (no extra features), 1.14 (session-end handoff).

4. **Read the Stage 10 spec** in `masterplan/MASTERPLAN.md` lines 825-904. This is the authoritative specification.

5. **Read all upstream downstream logs** — especially `masterplan/downstream_log_stage_09.md` (TT & Move Ordering) and `masterplan/downstream_log_stage_06.md` (Evaluator trait). These contain API contracts you must respect.

6. **Read `masterplan/stage_10_mcts.md`** — Implementation notes, ADR references.

7. **Run `cargo build --release && cargo test`** — Confirm clean foundation. Expect 408 tests, 0 failures, 0 warnings.

---

## What You're Building

A standalone Gumbel MCTS searcher. 13-step build order. Each step should compile and pass tests before moving to the next.

### Architecture Overview

```
search/
├── mod.rs          # Searcher trait, SearchBudget, SearchResult (EXISTS — DO NOT MODIFY)
├── brs.rs          # BrsSearcher (EXISTS — DO NOT MODIFY)
├── board_scanner.rs # Board scanner (EXISTS — DO NOT MODIFY)
├── tt.rs           # Transposition table (EXISTS — DO NOT MODIFY)
└── mcts.rs         # NEW — MctsSearcher, MctsNode, all MCTS logic
```

Create ONE new file: `odin-engine/src/search/mcts.rs`. Register it in `search/mod.rs` with `pub mod mcts;`.

### Core Traits You Must Implement Against

**Searcher trait** (FROZEN — `search/mod.rs`):
```rust
pub trait Searcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult;
}
```

**Evaluator trait** (FROZEN — `eval/mod.rs`):
```rust
pub trait Evaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16;
    fn eval_4vec(&self, position: &GameState) -> [f64; 4];
}
```

Use `eval_4vec` for leaf evaluation. It returns normalized [0,1] values for all 4 players via sigmoid.

**SearchBudget** (FROZEN):
```rust
pub struct SearchBudget {
    pub max_depth: Option<u8>,      // tree depth limit
    pub max_nodes: Option<u64>,     // simulation count limit
    pub max_time_ms: Option<u64>,   // wall-clock limit
}
```

For MCTS: `max_nodes` = simulation count, `max_depth` = max tree depth per simulation, `max_time_ms` = wall-clock cutoff.

**SearchResult** (FROZEN):
```rust
pub struct SearchResult {
    pub best_move: Move,
    pub score: i16,                 // centipawns from root player perspective
    pub depth: u8,                  // max depth reached in any simulation
    pub nodes: u64,                 // total simulations completed
    pub pv: Vec<Move>,              // most-visited path from root
}
```

---

## Build Order (13 Steps)

### Step 1: MCTS Node Struct

```rust
struct MctsNode {
    move_to_here: Option<Move>,     // None for root
    player_to_move: Player,         // whose turn at this node
    visit_count: u32,
    value_sum: [f64; 4],            // accumulated value per player
    prior: f32,                     // pi(a) from policy
    gumbel: f32,                    // Gumbel(0,1) noise — only meaningful at root
    children: Vec<MctsNode>,
    is_expanded: bool,
    is_terminal: bool,
}
```

Implement basic `MctsNode::new()`, `q_value(&self, player_idx: usize) -> f64` (returns `value_sum[player_idx] / visit_count` or 0.0 if unvisited).

**Tests:** Node creation, q_value with 0 visits returns 0, q_value after updates is correct.

### Step 2: Prior Policy Computation

```rust
fn compute_priors(moves: &[Move], position: &GameState) -> Vec<f32>
```

Pre-NNUE prior: `pi(a) = softmax(ordering_score(a) / PRIOR_TEMPERATURE)`. Temperature default: 50.0.

Ordering scores come from the same factors as BRS move ordering: captures get MVV-LVA scores, quiets get a baseline. You don't need TT hints, killers, or history here — just MVV-LVA for captures and a small base score for quiets. Keep it simple.

**Tests:** Priors sum to ~1.0, captures get higher priors than quiet moves, all priors > 0.

### Step 3: Gumbel Noise Sampling at Root

```rust
fn sample_gumbel() -> f32   // Gumbel(0,1) = -ln(-ln(U)), U ~ Uniform(0,1)
```

At root node creation, sample `g(a) ~ Gumbel(0,1)` for each legal move. Store in `MctsNode.gumbel`.

Compute `g(a) + log(pi(a))` to rank root children. Keep Top-k candidates (default `k = 16`). If fewer than k legal moves, keep all.

**Tests:** Gumbel samples are finite, Top-k selection keeps correct count, ranking is by g + log(pi).

### Step 4: Sequential Halving Framework

At root only. After Top-k selection:
1. Divide total simulation budget across `ceil(log2(k))` rounds
2. Each round: allocate `budget_per_round / num_remaining` sims per candidate
3. After simulating, score each candidate: `sigma(g(a) + log(pi(a)) + q(a))` where sigma is the logistic function `1/(1+exp(-x))` and `q(a)` is the updated Q-value
4. Drop bottom half of candidates
5. Repeat until 1 candidate remains

**Tests:** Halving reduces candidate count correctly, budget allocation covers all sims, single candidate survives.

### Step 5: Non-Root Tree Policy

Select child maximizing:
```
score = Q(node)[player] / N(node) + C_PRIOR * pi(a) / (1 + N(node)) + PH(a)
```

Where:
- `Q(node)[player]` = `value_sum[player_to_move.index()]`
- `N(node)` = `visit_count`
- `C_PRIOR` = 1.5 (default, tunable)
- `PH(a) = PROGRESSIVE_HISTORY_WEIGHT * H(a) / (N(a) + 1)` — defaults to 0 in standalone mode

For unvisited children (N=0): score = prior (ensures they get explored).

**Tests:** Unvisited child selected first, higher-Q child preferred when visit counts equal, prior exploration decays with visits.

### Step 6: Expansion + Leaf Evaluation

When a leaf node is reached (not expanded, not terminal):
1. Generate legal moves via `position.generate_legal_moves()`
2. Compute priors for all moves (Step 2)
3. Create child nodes with priors set
4. Check for terminal (no legal moves, or game over via `position.is_game_over()`)
5. Evaluate with `eval_4vec` → returns `[f64; 4]`
6. For terminal positions: use game result scoring

**Important:** You must clone the GameState and apply moves to reach child positions. The `Arc<Vec<u64>>` position_history makes cloning cheap.

**Tests:** Expansion creates correct number of children, terminal detection works, eval produces valid 4-vec.

### Step 7: Backpropagation (4-Player MaxN)

After leaf evaluation, walk back up the path:
```rust
for node in path.iter_mut().rev() {
    node.visit_count += 1;
    for i in 0..4 {
        node.value_sum[i] += leaf_values[i];
    }
}
```

Each player maximizes their own component. No negation — all 4 values propagate as-is.

**Tests:** Visit counts increment along path, all 4 value components propagate, root aggregates correctly.

### Step 8: Progressive Widening

Don't expand all children at once. Limit:
```
max_children = floor(W * N.powf(B))
```

Defaults: `W = 2.0`, `B = 0.5`. So at 1 visit = 2 children, 4 visits = 4, 16 visits = 8, etc.

When expanding, create children sorted by prior (highest first), expand up to `max_children`.

**Tests:** Child count respects limit, higher-prior moves expanded first, limit grows with visits.

### Step 9: Simulation Budget Control

The simulation loop:
```rust
loop {
    if simulations >= budget.max_nodes { break; }
    if elapsed >= budget.max_time_ms { break; }
    // select -> expand -> evaluate -> backpropagate
    simulations += 1;
}
```

Check time every 64 simulations (not every sim — syscall overhead).

**Tests:** Stops at node budget, stops at time budget, handles budget of 1 and 2 sims.

### Step 10: PV Extraction

Walk from root, always following the most-visited child:
```rust
fn extract_pv(root: &MctsNode) -> Vec<Move> {
    let mut pv = Vec::new();
    let mut node = root;
    while let Some(best) = node.children.iter().max_by_key(|c| c.visit_count) {
        if let Some(mv) = best.move_to_here {
            pv.push(mv);
        }
        node = best;
    }
    pv
}
```

**Tests:** PV starts with best move, PV length > 0 for non-trivial positions.

### Step 11: Temperature for Self-Play Exploration

For self-play data generation (future), add temperature parameter:
- `temperature = 0.0`: select most-visited (deterministic)
- `temperature = 1.0`: sample proportional to visit count
- `temperature > 0`: `prob(a) = N(a)^(1/T) / sum(N(b)^(1/T))`

Default to `temperature = 0.0` (deterministic) for now.

**Tests:** Temperature 0 selects most-visited, temperature 1 produces valid distribution.

### Step 12: `set_prior_policy()` and `set_history_table()`

These are called by the Stage 11 hybrid controller. For now, implement as stubs that store the data:

```rust
impl MctsSearcher {
    pub fn set_prior_policy(&mut self, priors: &[f32]) { ... }
    pub fn set_history_table(&mut self, history: &HistoryTable) { ... }
}
```

Define `HistoryTable` type alias matching BRS's format:
```rust
pub type HistoryTable = [[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT];
```

When `set_history_table` is called, enable Progressive History term in tree policy (Step 5).

**Tests:** Setting history enables PH term, PH weight affects selection, no-history mode works.

### Step 13: Implement `MctsSearcher` Against `Searcher` Trait

```rust
pub struct MctsSearcher {
    // config
    top_k: usize,              // default 16
    c_prior: f32,              // default 1.5
    prior_temperature: f32,    // default 50.0
    pw_w: f64,                 // progressive widening W, default 2.0
    pw_b: f64,                 // progressive widening B, default 0.5
    ph_weight: f32,            // progressive history weight, default 0.1
    temperature: f64,          // move selection temperature, default 0.0
    // state
    history_table: Option<HistoryTable>,
    external_priors: Option<Vec<f32>>,
    info_callback: Option<Box<dyn FnMut(String)>>,
}

impl Searcher for MctsSearcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult {
        // 1. Create root node
        // 2. If root has <=1 legal move, return immediately
        // 3. Expand root, compute priors, sample Gumbel
        // 4. Top-k selection
        // 5. Sequential Halving loop (allocate sims, simulate, eliminate)
        // 6. Return best candidate as SearchResult
    }
}
```

Wire up `info_callback` to emit UCI-style `info` strings (depth, nodes, score, pv, nps).

**Tests:** Full integration — search from starting position with 100 sims, search with 2 sims (Gumbel advantage), search with time budget.

---

## What You DON'T Build

- **Neural network priors** (Stage 16). Use softmax over ordering scores.
- **Root parallelism or virtual loss.** Single-threaded.
- **Integration with BRS** (Stage 11). MCTS is standalone. `set_history_table` stores it but BRS doesn't call it yet.
- **Persistent tree between moves** (Stage 13).
- **Protocol integration.** Don't modify `protocol/` or `OdinEngine`. Stage 11 wires MCTS into the hybrid controller.

---

## Acceptance Criteria

All must pass before Stage 10 is complete:

- **AC1:** Gumbel MCTS finds reasonable moves with only 2 simulations
- **AC2:** With 100+ sims, results match or beat UCB1 quality
- **AC3:** 4-player value backpropagation is correct (unit test each component independently)
- **AC4:** Progressive widening limits tree breadth (measurable child count ceiling)
- **AC5:** 1000+ simulations complete in reasonable time (<5s release build, starting position)
- **AC6:** `MctsSearcher` implements `Searcher` trait correctly (drop-in compatible)
- **AC7:** Progressive history warm-start measurably reduces wasted simulations vs cold start (when history table provided)
- **AC8:** Sequential Halving correctly eliminates weaker candidates across rounds

---

## Test File

Create `odin-engine/tests/stage_10_mcts.rs`. Follow the pattern of existing stage test files (see `tests/stage_07_brs.rs` or `tests/stage_09_tt_ordering.rs`).

Minimum test categories:
1. **Node basics** — creation, q_value, visit counting
2. **Prior computation** — softmax, temperature, captures > quiets
3. **Gumbel sampling** — finite values, Top-k selection
4. **Sequential Halving** — correct elimination, budget allocation
5. **Tree policy** — unvisited preference, Q-value exploitation, prior exploration
6. **Expansion** — child count, terminal detection, progressive widening
7. **Backpropagation** — 4-player value propagation
8. **Full search** — 2 sims, 100 sims, 1000 sims, time-budgeted
9. **Searcher trait** — MctsSearcher as `Box<dyn Searcher>` works
10. **Progressive history** — with/without history table comparison

---

## Tracing Points (Add These)

Per the MASTERPLAN:
- **Simulation:** selection path, leaf evaluation (4-vec), visit count before/after
- **Gumbel root:** Top-k candidates with `g(a) + log(pi(a))` scores, Sequential Halving rounds, eliminations
- **Selection (Verbose+):** children scores (Q + prior + PH), child selected, player perspective
- **Expansion:** position, move, prior assigned, progressive widening check
- **Root summary:** full visit distribution, selected move, temperature, halving rounds completed

Use the `info_callback` for standard info lines. Use `tracing` crate macros (`tracing::debug!`, `tracing::trace!`) for verbose internal logging — the engine already uses tracing from Stage 4.

---

## Key Constants (Defaults)

```rust
const MCTS_TOP_K: usize = 16;
const MCTS_C_PRIOR: f32 = 1.5;
const MCTS_PRIOR_TEMPERATURE: f32 = 50.0;
const MCTS_PW_W: f64 = 2.0;         // progressive widening W
const MCTS_PW_B: f64 = 0.5;         // progressive widening B
const MCTS_PH_WEIGHT: f32 = 0.1;    // progressive history weight
const MCTS_TIME_CHECK_INTERVAL: u64 = 64;  // check clock every N sims
```

---

## Performance Context

Current BRS at depth 8: 13,009 nodes in 120ms (release). MCTS doesn't need to beat BRS — it needs to explore broadly where BRS explores deeply. Target: 1000 sims in <5 seconds from starting position.

Diagnostic baseline: engine currently plays at ~2100-2300 Elo equivalent on the 4PC FFA scale (see `observer/baselines/README.md`). Zero captures in first 20 rounds, excessive piece shuffling, late queen activation. MCTS is expected to address shuffling/tempo waste by evaluating entire game trees statistically rather than through the paranoid BRS single-reply model.

---

## Session-End Protocol

When you finish Stage 10:
1. Run `cargo test` — all tests pass (existing 408 + new Stage 10 tests)
2. Run `cargo clippy` — zero warnings
3. Update `masterplan/HANDOFF.md` with what was done and what's next
4. Update `masterplan/STATUS.md` with new test counts and stage status
5. Create `masterplan/sessions/Session-YYYY-MM-DD-Stage10-MCTS.md`
6. Write `masterplan/downstream_log_stage_10.md` with API contracts and notes for Stage 11
7. Fill in `masterplan/audit_log_stage_10.md`
8. Do NOT tag the stage — user confirms and tags

---

## Critical Reminders

- **Do not modify existing search files** (mod.rs, brs.rs, board_scanner.rs, tt.rs). MCTS is additive.
- **The Searcher trait is FROZEN.** Do not change its signature.
- **The Evaluator trait is FROZEN.** Use `eval_4vec` for leaf eval.
- **Turn order is R→B→Y→G.** Use `Player::next()` to cycle.
- **`unmake_move` takes 3 args:** `unmake_move(board, mv, undo)`.
- **perft invariants:** 20/395/7800/152050 — these must never change.
- **Read AGENT_CONDUCT.md Section 1.18** — Only the top-level orchestrating agent runs the engine/builds. Subagents must not independently start the engine binary.
