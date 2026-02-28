# Stage 16 Prompt — NNUE Integration

You are implementing Stage 16 of Project Odin, a four-player chess engine (14x14 board, R/B/Y/G).

**Read these files before writing any code:**
1. `masterplan/STATUS.md` — project state (Stage 15 complete, 526 tests, v1.15)
2. `masterplan/HANDOFF.md` — last session summary
3. `masterplan/AGENT_CONDUCT.md` Section 1.1 — stage entry protocol
4. `masterplan/DECISIONS.md` — ADR-003 (Dual-Head NNUE), ADR-004 (HalfKP-4)
5. `masterplan/MASTERPLAN.md` — Stage 16 section
6. `masterplan/downstream_log_stage_14.md` — AccumulatorStack API contracts (W17-W19)
7. `masterplan/downstream_log_stage_15.md` — Training pipeline contracts (W20-W22)

---

## What You're Building

Replace the bootstrap eval throughout the engine with NNUE. This is the most dangerous swap — the new eval might expose bugs in accumulator management or produce worse play than the handcrafted eval. The `Evaluator` trait makes the swap clean, but the real work is in accumulator lifecycle management.

Currently:
- `HybridController::new(profile)` creates `BootstrapEvaluator` for both BRS and MCTS.
- `NnueEvaluator` exists (Stage 14) but does a **full refresh every eval call** — no incremental updates.
- `AccumulatorStack` exists with `push(mv, board_before, weights)` / `pop()` — tested but not wired into search.

After this stage:
- BRS search calls `push` before `make_move` and `pop` after `unmake_move`, giving incremental accumulator updates through the entire search tree.
- MCTS simulations save/restore accumulator state around each simulation path.
- Both leaf evals use the NNUE forward pass instead of bootstrap.
- Fallback to `BootstrapEvaluator` when no `.onnue` file is provided.

---

## Architecture Recap

### NNUE Inference (Stage 14 — already built)

```
Input: 4,480 sparse features per perspective (HalfKP-4)
  |
Feature Transformer: 4,480 -> 256 (SCReLU, int16 quantized) x4 perspectives
  |
Concatenate: 4 x 256 = 1024
  |
Hidden Layer: 1024 -> 32 (ClippedReLU, int8 weights)
  |
Dual Output Heads:
  - BRS Head: 32 -> 1 (centipawn scalar)
  - MCTS Head: 32 -> 4 (per-player sigmoid values)
```

### AccumulatorStack API (Stage 14 — already built)

| Method | Purpose |
|--------|---------|
| `AccumulatorStack::new()` | Pre-allocates 128-deep stack |
| `init_from_board(board, weights)` | Full compute at root (resets to depth 0) |
| `push(mv, board_before, weights)` | Copy-forward + incremental delta. **Must be called BEFORE make_move** |
| `pop()` | Zero-cost restore (just decrements pointer). Pair with `push`. |
| `refresh_if_needed(board, weights)` | Recomputes any perspective flagged `needs_refresh`. Call before forward pass. |
| `current()` -> `&Accumulator` | Returns the current accumulator for forward pass |

### Forward Pass (Stage 14 — already built)

```rust
forward_pass(acc: &Accumulator, weights: &NnueWeights, player: Player) -> (i16, [f64; 4])
//            BRS centipawns ↑          ↑ MCTS per-player sigmoid values
```

### Key Constants

| Constant | Value | Location |
|----------|-------|----------|
| `FEATURES_PER_PERSPECTIVE` | 4,480 | `eval/nnue/features.rs` |
| `FT_OUT` | 256 | `eval/nnue/features.rs` |
| `HIDDEN_SIZE` | 32 | `eval/nnue/features.rs` |
| `QA` (SCReLU clamp) | 255 | `eval/nnue/features.rs` |
| `QB` | 64 | `eval/nnue/features.rs` |
| `OUTPUT_SCALE` | 400 | `eval/nnue/features.rs` |
| `SIGMOID_K` | 4000.0 | `eval/nnue/mod.rs` |
| `ELIMINATED_SCORE` | -30,000 | `eval/nnue/mod.rs` |
| `MAX_STACK_DEPTH` | 128 | `eval/nnue/features.rs` |

---

## Current Search Architecture (What You're Modifying)

### BRS Searcher (`search/brs.rs`)

```rust
pub struct BrsSearcher {
    evaluator: Box<dyn Evaluator>,   // Currently BootstrapEvaluator
    tt: TranspositionTable,
    // ...
}

impl BrsSearcher {
    pub fn new(evaluator: Box<dyn Evaluator>) -> Self { ... }
}
```

BRS uses `evaluator.eval_scalar(gs, player)` at:
- Root position scoring (line ~325, ~387-392)
- Quiescence stand-pat (line ~789)
- Min-node opponent reply selection (via `select_hybrid_reply` and `select_best_opponent_reply`)

The evaluator is accessed via `self.evaluator` in `BrsSearcher`, passed as `&'a dyn Evaluator` into `BrsContext`.

BRS calls `make_move` / `unmake_move` pairs throughout `alphabeta()` and `quiescence()`. Each of these call sites needs a corresponding `push` / `pop` on the accumulator stack.

### MCTS Searcher (`search/mcts.rs`)

```rust
pub struct MctsSearcher {
    evaluator: Box<dyn Evaluator>,   // Currently BootstrapEvaluator
    // ...
}
```

MCTS uses `evaluator.eval_4vec(gs)` at leaf nodes during simulation (line ~352). Each simulation plays out a path of moves from root to leaf, then evaluates. The accumulator state must be saved at root and restored after each simulation.

### Hybrid Controller (`search/hybrid.rs`)

```rust
pub struct HybridController {
    brs: BrsSearcher,
    mcts: MctsSearcher,
    // ...
}

impl HybridController {
    pub fn new(profile: EvalProfile) -> Self {
        let brs = BrsSearcher::new(Box::new(BootstrapEvaluator::new(profile)));
        let mcts = MctsSearcher::new(Box::new(BootstrapEvaluator::new(profile)));
        // ...
    }
}
```

### Protocol Layer (`protocol/mod.rs`)

```rust
let searcher = self.searcher.get_or_insert_with(|| {
    HybridController::new(profile)
});
```

This is where the evaluator choice happens. Stage 16 adds NNUE weight loading and evaluator selection here.

---

## Build Order

### Step 1: AccumulatorStack in BrsSearcher

**The core integration.** Wire `AccumulatorStack` into BRS so incremental NNUE updates happen through the search tree.

**Design decision:** The `AccumulatorStack` should live in `BrsSearcher` (not in the evaluator). The evaluator trait is `&self`, but the accumulator needs `&mut self` for push/pop. BrsSearcher already has `&mut self` access.

```rust
pub struct BrsSearcher {
    evaluator: Box<dyn Evaluator>,
    tt: TranspositionTable,
    // New: NNUE accumulator stack (None when using BootstrapEvaluator)
    acc_stack: Option<AccumulatorStack>,
    nnue_weights: Option<NnueWeights>,
    // ...
}
```

`BrsContext` gets access to the accumulator stack:
```rust
struct BrsContext<'a> {
    // ... existing fields ...
    acc_stack: Option<&'a mut AccumulatorStack>,
    nnue_weights: Option<&'a NnueWeights>,
}
```

**Lifecycle in BRS:**
1. At search start: `acc_stack.init_from_board(board, weights)` — full compute at root
2. Before every `make_move`: `acc_stack.push(mv, board_before, weights)`
3. After every `unmake_move`: `acc_stack.pop()`
4. At every leaf eval: `acc_stack.refresh_if_needed(board, weights)` then `forward_pass(acc_stack.current(), weights, player)` — replaces `evaluator.eval_scalar()`

**Critical:** `push` needs `board_before` (the board state BEFORE the move is applied). The BRS code already has this ordering — moves are scored after `make_move`, so `push` must be inserted BEFORE `make_move` and the board at that point IS `board_before`.

**Make/unmake call sites to instrument (search these in brs.rs):**
- `alphabeta()` — max node: `make_move` / `unmake_move` in the move loop
- `alphabeta()` — min node (opponent reply): `make_move` / `unmake_move`
- `alphabeta()` — null move: `make_move` / `unmake_move` (push null-move delta or mark refresh)
- `quiescence()` — max node captures: `make_move` / `unmake_move`
- `quiescence()` — min node opponent reply: `make_move` / `unmake_move`

**Null move handling:** Null moves don't have a real piece movement. You can either skip the push/pop (mark `needs_refresh` on the next real eval) or do a full refresh before the null-move subtree eval. Since null-move pruning is a rare fast-exit path, a full refresh is acceptable.

### Step 2: BRS Leaf Eval Swap

Replace all `evaluator.eval_scalar(gs, player)` calls in BRS with NNUE-based eval when `acc_stack` is `Some`.

```rust
fn nnue_eval_scalar(&mut self, player: Player) -> i16 {
    if let (Some(stack), Some(weights)) = (&mut self.acc_stack, &self.nnue_weights) {
        stack.refresh_if_needed(self.gs.board(), weights);
        let (brs_score, _) = forward_pass(stack.current(), weights, player);
        brs_score
    } else {
        self.evaluator.eval_scalar(&self.gs, player)
    }
}
```

This preserves the bootstrap fallback path.

### Step 3: AccumulatorStack in MctsSearcher

MCTS simulations play moves from root to leaf, then eval, then backtrack. The accumulator needs to follow this path.

**Option A (simpler):** Give MCTS its own `AccumulatorStack`. Before each simulation, reset to root accumulator state. During simulation, push for each move in the path. After eval, pop all the way back (or just re-init from root).

**Option B (efficient):** Share the accumulator with BRS. After BRS phase completes, pass its accumulator state to MCTS. MCTS uses push/pop along simulation paths.

**Recommended: Option A.** Simpler, avoids coupling. MCTS sims are short (typically 4-20 moves deep), so the overhead of init_from_board once per search (not per sim) is negligible.

```rust
pub struct MctsSearcher {
    evaluator: Box<dyn Evaluator>,
    acc_stack: Option<AccumulatorStack>,
    nnue_weights: Option<NnueWeights>,
    // ...
}
```

**Lifecycle in MCTS:**
1. At `search()` start: `acc_stack.init_from_board(root_board, weights)`
2. During each simulation: `push` before each move, `pop` after backpropagation unwinds
3. At leaf: `refresh_if_needed` then `forward_pass` for `eval_4vec` replacement

### Step 4: MCTS Leaf Eval Swap

Replace `evaluator.eval_4vec(gs)` at MCTS leaf nodes with NNUE forward pass.

```rust
fn nnue_eval_4vec(&mut self, gs: &GameState) -> [f64; 4] {
    if let (Some(stack), Some(weights)) = (&mut self.acc_stack, &self.nnue_weights) {
        stack.refresh_if_needed(gs.board(), weights);
        let root = gs.board().side_to_move();
        let (_, mcts_values) = forward_pass(stack.current(), weights, root);
        // Override eliminated players
        let mut result = mcts_values;
        for &p in &Player::ALL {
            if gs.player_status(p) == PlayerStatus::Eliminated {
                result[p.index()] = 0.001;
            }
        }
        result
    } else {
        self.evaluator.eval_4vec(gs)
    }
}
```

### Step 5: MCTS Policy Head (Optional/Deferred)

The MASTERPLAN mentions replacing uniform priors with NNUE policy head output. However, **the current architecture has no policy head** — the MCTS head outputs per-player values (4 scalars), not per-move probabilities.

**For this stage:** Keep using BRS-derived priors from the hybrid controller (the softmax over BRS root move scores). A policy head is a future architecture change.

If you want to add a lightweight policy signal: use the BRS score differential after the NNUE eval swap as the prior (same mechanism as now, just with NNUE-based scores instead of bootstrap). This requires no architecture change.

### Step 6: Fallback Mechanism

When no `.onnue` file is available, use `BootstrapEvaluator`.

**In `HybridController::new()`:**
```rust
impl HybridController {
    pub fn new(profile: EvalProfile, nnue_path: Option<&str>) -> Self {
        let (brs_eval, mcts_eval, nnue_weights): (
            Box<dyn Evaluator>, Box<dyn Evaluator>, Option<NnueWeights>
        ) = match nnue_path {
            Some(path) => match NnueWeights::load(path) {
                Ok(w) => {
                    // NNUE mode: evaluators are still needed for eliminated-player checks
                    // but leaf eval uses forward_pass directly
                    let w2 = NnueWeights::load(path).unwrap(); // second copy for MCTS
                    (
                        Box::new(BootstrapEvaluator::new(profile)), // fallback
                        Box::new(BootstrapEvaluator::new(profile)), // fallback
                        Some(w),
                    )
                },
                Err(e) => {
                    eprintln!("warning: failed to load NNUE weights from {}: {}. Using bootstrap eval.", path, e);
                    (
                        Box::new(BootstrapEvaluator::new(profile)),
                        Box::new(BootstrapEvaluator::new(profile)),
                        None,
                    )
                }
            },
            None => (
                Box::new(BootstrapEvaluator::new(profile)),
                Box::new(BootstrapEvaluator::new(profile)),
                None,
            ),
        };
        // ...
    }
}
```

**In protocol layer:** Add an engine option for the NNUE weight file path:
```
setoption name NnueFile value weights_gen0.onnue
```

Or auto-detect: look for `weights.onnue` in the engine's working directory.

**In `NnueWeights`:** The weights can be shared via `Arc<NnueWeights>` between BRS and MCTS instead of loading twice. The weights are read-only during search, so shared references work.

### Step 7: A/B Self-Play Comparison

Use `observer/match.mjs` (Stage 12 infrastructure):

1. Build two engine binaries:
   - `odin-engine.exe` (with NNUE weights loaded)
   - `odin-engine-baseline.exe` (bootstrap eval, or same binary without NNUE file)
2. Run `node match.mjs match_config.json` with 100+ games
3. Check: NNUE engine should beat bootstrap engine >55% win rate

This step requires Gen-0 weights from the Stage 15 training pipeline. If Gen-0 weights are not yet available, defer this step.

---

## Critical Warnings from Previous Stages

### W17 (Stage 14): AccumulatorStack full refresh every call
The current `NnueEvaluator::eval_scalar()` calls `init_from_board()` every time — O(n) full recompute. Stage 16 MUST wire push/pop for incremental updates. This is the entire point of the stage.

### W18 (Stage 14): King moves mark needs_refresh
King moves flag `needs_refresh` for the owning perspective (even in Phase 1 HalfKP-4 without king bucketing). This means king moves cost a full perspective recompute (~10-15us) instead of incremental (~1-5us). Acceptable for now; profile in Stage 19.

### W19 (Stage 14): EP/castling fall back to full refresh
En passant and castling mark all perspectives as `needs_refresh`. Both are rare moves, so the overhead is negligible. Don't try to optimize this.

### W20 (Stage 15): serde only in datagen CLI path
Do NOT import serde in eval/search hot path. It's only used by `datagen.rs`.

### export.py QB bias fix (applied during Stage 15 review)
The original `export.py` template had `b_q = torch.round(bias * scale * QB)` which multiplied biases by 64x. This was fixed — the current `export.py` correctly uses `b_q = torch.round(bias * scale)` without the `* QB`. The Rust inference (`eval/nnue/mod.rs:81`) adds biases directly as i32 without QB division. **If you re-export weights, use the corrected export.py.**

### pv none fix (applied during Stage 15 review)
A guard was added in `brs.rs` to prevent empty PV from aspiration re-search overwriting the previous depth's PV. This is already committed.

---

## Performance Targets

| Metric | Target | Baseline (Stage 14) |
|--------|--------|---------------------|
| NNUE incremental eval (push + forward) | < 2us (target 1us) | ~1-5us push, ~2-5us forward |
| NNUE full eval (init_from_board + forward) | < 50us | ~30-50us |
| BRS with NNUE NPS | > 500K nps | ~100K nps (bootstrap, depth 8) |
| MCTS with NNUE sims/sec | > 5K sims/sec | ~8K sims/sec (bootstrap) |

The 500K NPS target for BRS with NNUE is ambitious — bootstrap currently does ~100K at depth 8. The goal is that incremental accumulator updates (1-5us) replacing per-node eval (~10us bootstrap) should yield a net speedup, especially since many BRS nodes use the TT and skip eval entirely.

If the NPS target isn't met, focus on correctness first. Optimization is Stage 19.

---

## Acceptance Criteria (Tests Required)

### Correctness Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T1 | `test_nnue_brs_push_pop_matches_full` | Incremental accumulator through BRS make/unmake matches full recompute at each position |
| T2 | `test_nnue_mcts_sim_accumulator` | Accumulator state is correctly saved/restored across MCTS simulations |
| T3 | `test_fallback_without_nnue_file` | Engine runs with BootstrapEvaluator when no .onnue file is provided |
| T4 | `test_perft_unchanged` | perft(1-4) results are identical (NNUE doesn't affect move generation) |
| T5 | `test_nnue_eval_non_degenerate` | NNUE eval on starting position returns values in valid range (not 0, not extreme) |

### Integration Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T6 | `test_brs_search_with_nnue` | BRS search with NNUE completes without panic, returns valid move |
| T7 | `test_mcts_search_with_nnue` | MCTS search with NNUE completes without panic, returns valid move |
| T8 | `test_hybrid_search_with_nnue` | Full hybrid BRS+MCTS with NNUE completes without panic |
| T9 | `test_nnue_vs_bootstrap_no_crash` | Run 10 games (5 ply each) with NNUE eval — no panics or assertion failures |

### Performance Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T10 | `test_incremental_vs_full_speed` | Incremental accumulator update is faster than full recompute (>2x) |

### A/B Test (human-driven, after Gen-0 weights available)

| ID | Test | What it verifies |
|----|------|-----------------|
| T11 | Self-play tournament | NNUE engine beats bootstrap engine >55% win rate over 100+ games |

---

## Critical Invariants — DO NOT VIOLATE

1. **`push` BEFORE `make_move`.** The accumulator needs `board_before` to compute deltas. Every `push` must happen before the corresponding `make_move`.
2. **Every `push` has a matching `pop`.** Unmatched push/pop will corrupt the accumulator stack depth.
3. **`refresh_if_needed` before every `forward_pass`.** King moves, EP, and castling flag perspectives for refresh. Always call this before reading accumulator values.
4. **Evaluator trait is FROZEN.** Do not change `eval_scalar(&self, &GameState, Player) -> i16` or `eval_4vec(&self, &GameState) -> [f64; 4]`.
5. **Searcher trait is FROZEN.** Do not change `search(&mut self, &GameState, SearchBudget) -> SearchResult`.
6. **perft invariants are permanent.** perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050.
7. **Turn order R→B→Y→G.** Never alter.
8. **SIGMOID_K = 4000.0, OUTPUT_SCALE = 400.** Must be consistent between training and inference.
9. **The `.onnue` format is frozen from Stage 14.** Do not change weight layout or header format.
10. **TT probe MUST come AFTER repetition check in alphabeta().** Critical invariant from Stage 9.

---

## Scope Boundaries — What NOT To Build

- Retraining or new weight generation (use Gen-0 weights from Stage 15)
- NNUE architecture changes (frozen from Stage 14)
- Policy head for MCTS (future work)
- SIMD vectorization of accumulator ops (Stage 19)
- King bucketing Phase 2 (deferred)
- Distributed search or threading (deferred)
- Changes to the `.onnue` file format

---

## Tracing Points (Temporary Diagnostics)

During development, add temporary diagnostic output (behind a flag or `#[cfg(debug_assertions)]`):

1. **Accumulator correctness:** At select positions (every 1000th node), compare incremental accumulator vs full recompute. Flag >0 divergence.
2. **NNUE vs bootstrap comparison:** At root, log both NNUE and bootstrap eval side by side. Flag positions with >200cp disagreement.
3. **Accumulator stack depth:** Log max stack depth reached per search. Should never exceed ~60 for depth 8 BRS.

Remove or gate these behind `#[cfg(debug_assertions)]` before the final commit.

---

## Pre-Audit Checklist

1. `cargo build && cargo test` — verify all existing 526 + new tests pass
2. `cargo clippy` — verify 0 warnings
3. Create `masterplan/audit_log_stage_16.md` with pre-audit section

## Post-Audit Checklist

1. `cargo test` — all existing + new tests pass
2. `cargo clippy` — 0 warnings
3. Fill post-audit section of `audit_log_stage_16.md`
4. Create `masterplan/downstream_log_stage_16.md`
5. Update `masterplan/STATUS.md` and `masterplan/HANDOFF.md`
6. Create session note in `masterplan/sessions/`
