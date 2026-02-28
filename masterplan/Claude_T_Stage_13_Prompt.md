# Stage 13: Time Management — Implementation Prompt

## Context

You are implementing Stage 13 of Project Odin, a 4-player chess engine.
The engine is at tag `stage-12-complete` / `v1.12` with 465 tests passing (281 unit + 184 integration, 5 ignored), 0 clippy warnings.

**What exists:**
- `odin-engine/` — Rust engine with BRS+MCTS hybrid search (`search/hybrid.rs`), bootstrap eval, Odin protocol
- `search/hybrid.rs` — HybridController with adaptive BRS/MCTS time split (tactical 30/70, quiet 10/90), time pressure path (<100ms skips MCTS), position classification via capture ratio
- `protocol/mod.rs` — Odin protocol handler with `go` command supporting `wtime/btime/ytime/gtime`, `depth`, `nodes`, `movetime`, `infinite`
- `observer/` — Self-play tools: `match.mjs` (2-engine match manager), `elo.mjs`, `sprt.mjs`
- Engine binary at `target/release/odin-engine.exe`

**What Stage 13 builds (from MASTERPLAN.md):**
1. Time allocation formula — smart clock management per move
2. Position complexity detection — enrich beyond capture ratio
3. Adaptive adjustments — game phase, elimination proximity, forced moves
4. Safety checks — never flag (never use >25% remaining, minimum 100ms)
5. Search parameter tuning via self-play — tune TACTICAL_MARGIN, BRS fractions, MCTS exploration, etc.
6. Pondering (optional) — think on opponent's time

**Acceptance criteria:**
- AC1: Engine manages time correctly across full games (doesn't flag)
- AC2: Time adapts to position complexity
- AC3: Tuned parameters improve win rate vs. defaults (measured via Stage 12 match manager)

---

## Step 0: Orientation (READ FIRST)

Read these files to understand the current architecture:

| File | Why |
|------|-----|
| `masterplan/STATUS.md` | Current project state |
| `masterplan/HANDOFF.md` | Last session context |
| `masterplan/AGENT_CONDUCT.md` Section 1.1 | Stage entry protocol |
| `odin-engine/src/search/hybrid.rs` | **CRITICAL** — Current time split logic, constants, position classification |
| `odin-engine/src/search/mod.rs` | SearchBudget struct, Searcher trait |
| `odin-engine/src/protocol/mod.rs` | `go` command parsing, `limits_to_budget()` function |
| `odin-engine/src/protocol/types.rs` | GoLimits struct — see what fields exist |
| `odin-engine/src/protocol/parser.rs` | How `go` parameters are parsed |
| `odin-engine/tests/stage_11_hybrid.rs` | Existing time-related tests (time_pressure_skips_mcts, adaptive_time_split, no_crash_tiny_time_budget, protocol_go_movetime_hybrid) |
| `masterplan/downstream_log_stage_11.md` | API contracts and warnings (W6, W10, W12, W13) |
| `masterplan/downstream_log_stage_12.md` | Stage 12 downstream notes |
| `masterplan/audit_log_stage_12.md` | Future Conflict Analysis section mentions Stage 13 |

---

## Step 1: Pre-Audit

Fill the **pre-audit** section of `masterplan/audit_log_stage_13.md`:
- List all files you plan to create or modify
- Confirm acceptance criteria mapping
- Note any risks or open questions

---

## Step 2: Protocol — Increment Parsing (AC1 prerequisite)

### Files: `protocol/parser.rs`, `protocol/types.rs`

Add increment parsing to the `go` command. Currently missing:

| Parameter | Meaning | Type |
|-----------|---------|------|
| `winc` | Red increment per move (ms) | u64 |
| `binc` | Blue increment per move (ms) | u64 |
| `yinc` | Yellow increment per move (ms) | u64 |
| `ginc` | Green increment per move (ms) | u64 |
| `movestogo` | Moves until next time control reset | u32 |

Add these fields to the `GoLimits` struct and parse them in the `go` command handler, following the exact pattern used for `wtime/btime/ytime/gtime`.

**Test:** Verify `go wtime 60000 winc 1000 btime 60000 binc 1000 ytime 60000 yinc 1000 gtime 60000 ginc 1000` parses correctly.

---

## Step 3: Time Allocation Formula (AC1, AC2)

### File: `search/time_manager.rs` (CREATE)

Create a dedicated time management module. This is the core deliverable.

**Formula:**
```
base_time = remaining / moves_left + increment

moves_left = max(movestogo, estimated_remaining_moves)
estimated_remaining_moves = clamp(50 - ply/4, 10, 50)
```

**Adjustments (multiplicative):**
```
factor = 1.0
if position is tactical:           factor *= 1.3
if position is quiet:              factor *= 0.8
if near_elimination (score < 2000): factor *= 2.0
if forced_move (1 legal move):      factor = 0.0  // instant return
if in_check:                        factor *= 1.2
```

**Safety constraints (CRITICAL — never flag):**
```
allocated = base_time * factor
allocated = min(allocated, remaining * 0.25)    // Never use >25% of clock
allocated = max(allocated, 100)                  // Minimum 100ms
if remaining < 1000:
    allocated = min(allocated, remaining * 0.10) // Panic mode: 10% max
```

**API:**
```rust
pub struct TimeManager {
    // No persistent state needed — pure function
}

impl TimeManager {
    /// Calculate time budget for this move.
    pub fn allocate(
        remaining_ms: u64,
        increment_ms: u64,
        ply: u32,
        movestogo: Option<u32>,
        num_legal_moves: usize,
        is_tactical: bool,
        is_in_check: bool,
        score_cp: Option<i16>,  // From previous search, if available
    ) -> u64;  // Returns allocated time in ms
}
```

**Integration point:** `limits_to_budget()` in `protocol/mod.rs` currently does `remaining / 50` for time controls. Replace this with a call to `TimeManager::allocate()`.

---

## Step 4: Enriched Position Classification (AC2)

### File: `search/hybrid.rs` (MODIFY)

Currently, position classification uses only capture ratio (line ~71-82):
```rust
let captures = legal_moves.iter().filter(|m| m.is_capture()).count();
let is_tactical = (captures as f64 / legal_moves.len() as f64) >= TACTICAL_CAPTURE_RATIO;
```

Enrich with additional signals:

1. **Check detection:** If side-to-move is in check → tactical
2. **Material imbalance:** Large material difference between players → tactical
3. **Few legal moves:** < 5 legal moves → tactical (likely forced or near-elimination)
4. **Piece count:** < 8 total pieces on board → endgame (different time profile)

Create a `PositionType` enum:
```rust
enum PositionType {
    Tactical,   // Captures, checks, threats — give BRS more time
    Quiet,      // Calm position — lean on MCTS
    Endgame,    // Few pieces — need deeper search
    Forced,     // 1 legal move — instant return
}
```

Wire this into both `TimeManager::allocate()` and the existing BRS/MCTS split logic.

---

## Step 5: Integration with HybridController (AC1, AC2)

### File: `search/hybrid.rs` (MODIFY)

Update the `search()` method to:

1. Call `TimeManager::allocate()` when the budget has a time component
2. Use the enriched `PositionType` for BRS/MCTS split decisions
3. **Forced move fast path:** If only 1 legal move, return it immediately without searching (this partially exists — formalize it)
4. Add timing info to the info callback output:
   ```
   info string time_alloc total=1500ms brs=150ms mcts=1350ms type=quiet remaining=45000ms
   ```

**IMPORTANT:** Do NOT change the Searcher trait signature. The time manager works inside the existing `SearchBudget` framework.

---

## Step 6: Parameter Tuning via Self-Play (AC3)

### File: `observer/tune.mjs` (CREATE)

Create a parameter tuning script that uses the Stage 12 match manager to A/B test different parameter values.

**Approach:** Manual grid search. Run matches between the engine compiled with different constants.

**Parameters to tune (priority order):**

| Parameter | Current Value | Test Range | What It Controls |
|-----------|--------------|------------|-----------------|
| `TACTICAL_MARGIN` | 150cp | 100, 150, 200, 250 | BRS survivor threshold |
| `BRS_FRACTION_TACTICAL` | 0.30 | 0.20, 0.30, 0.40 | Time to BRS in tactical positions |
| `BRS_FRACTION_QUIET` | 0.10 | 0.05, 0.10, 0.15 | Time to BRS in quiet positions |
| `MCTS_DEFAULT_SIMS` | 2000 | 1000, 2000, 4000 | MCTS simulation budget (depth-only mode) |
| `BRS_MAX_DEPTH` | 8 | 6, 8, 10 | Maximum BRS search depth |

**Implementation approach:**
Since these are compile-time constants, the tuning script should:
1. Accept a parameter name and value list from command line
2. For each value: modify the constant in source, rebuild, run N-game match vs baseline
3. Report Elo difference per value
4. Recommend the best value

**Simpler alternative (recommended for Stage 13):** Create a `tuning_config.json` that the engine reads at startup via `setoption` commands, rather than recompiling. Add `setoption name tactical_margin value 200` support to the protocol handler for tunable parameters.

**Usage:** `node tune.mjs --param tactical_margin --values 100,150,200,250 --games 50`

---

## Step 7: Tests (All ACs)

### File: `odin-engine/tests/stage_13_time_mgmt.rs` (CREATE)

**Required tests (minimum 10):**

| # | Test | What It Checks | AC |
|---|------|---------------|----|
| T1 | `test_time_allocation_basic` | `allocate(60000, 0, 0, None, 20, false, false, None)` returns reasonable value (500-3000ms) | AC1 |
| T2 | `test_time_allocation_with_increment` | Increment increases allocated time | AC1 |
| T3 | `test_time_allocation_tactical_bonus` | Tactical positions get more time than quiet | AC2 |
| T4 | `test_time_allocation_forced_move` | 1 legal move → 0ms or minimal allocation | AC2 |
| T5 | `test_time_allocation_safety_cap` | Never exceeds 25% of remaining | AC1 |
| T6 | `test_time_allocation_panic_mode` | Low clock (<1s) → very conservative | AC1 |
| T7 | `test_time_allocation_near_elimination` | Low score → bonus time | AC2 |
| T8 | `test_protocol_go_with_increments` | `go wtime 60000 winc 1000` parses and produces correct budget | AC1 |
| T9 | `test_hybrid_forced_move_instant` | Position with 1 legal move returns instantly (<50ms) | AC2 |
| T10 | `test_full_game_no_flag` | Play 100-ply game with 60s+1s increment, never exceed time | AC1 |
| T11 | `test_enriched_position_classification` | In-check → tactical, 1 move → forced, etc. | AC2 |
| T12 | `test_setoption_tunable_params` | Protocol accepts `setoption name tactical_margin value 200` | AC3 |

**Test helpers:**
```rust
fn time_budget(ms: u64) -> SearchBudget {
    SearchBudget { max_time_ms: Some(ms), max_depth: None, max_nodes: None }
}
```

---

## Step 8: Integration — Update Match Manager for Time Control

### File: `observer/match.mjs` (MODIFY)

Add a config option for time-controlled matches (currently only `go depth N`):

```json
{
  "time_control": {
    "initial_ms": 60000,
    "increment_ms": 1000
  }
}
```

When `time_control` is present in config, send `go wtime X winc Y btime X binc Y ytime X yinc Y gtime X ginc Y` instead of `go depth N`. Track remaining time per player, subtracting elapsed time after each move and adding increment.

This enables AC3 verification: run timed matches to confirm the engine doesn't flag and allocates time reasonably.

---

## Build Order Summary

| Step | What | Files | AC |
|------|------|-------|-----|
| 1 | Pre-audit | `masterplan/audit_log_stage_13.md` | — |
| 2 | Increment parsing | `protocol/parser.rs`, `protocol/types.rs` | AC1 |
| 3 | Time allocation formula | `search/time_manager.rs` (CREATE) | AC1, AC2 |
| 4 | Enriched position classification | `search/hybrid.rs` | AC2 |
| 5 | HybridController integration | `search/hybrid.rs`, `protocol/mod.rs` | AC1, AC2 |
| 6 | Parameter tuning infrastructure | `observer/tune.mjs`, protocol `setoption` additions | AC3 |
| 7 | Tests | `tests/stage_13_time_mgmt.rs` | All |
| 8 | Match manager time control | `observer/match.mjs` | AC3 |
| 9 | Post-audit + docs | audit log, downstream log, STATUS.md, HANDOFF.md | — |

---

## Scope Boundaries — DO NOT CHANGE

- **DO NOT** modify the eval code
- **DO NOT** modify BRS search logic (alpha-beta, quiescence, move ordering internals)
- **DO NOT** modify MCTS tree policy or backpropagation
- **DO NOT** modify existing tests (stage_07, stage_09, stage_11, stage_12)
- **DO NOT** add opening books or endgame tablebases
- The `Searcher` trait is FROZEN: `search(&mut self, &GameState, SearchBudget) -> SearchResult`
- The `Evaluator` trait is FROZEN: `eval_scalar(&self, &GameState, Player) -> i16`
- You MAY modify `search/hybrid.rs` (the HybridController internals) and `protocol/mod.rs` (go command parsing, option handling)

---

## Critical Invariants

1. **465 existing tests must still pass.** Run `cargo test` before and after.
2. **The `Searcher` trait is FROZEN.** `search(&mut self, &GameState, SearchBudget) -> SearchResult`
3. **The `Evaluator` trait is FROZEN.** `eval_scalar(&self, &GameState, Player) -> i16`
4. **Time pressure threshold (100ms) must remain a safety boundary.** You can tune it but not remove it.
5. **Turn order: R→B→Y→G.** `Player::ALL = [Red, Blue, Yellow, Green]`.
6. **perft invariants:** perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050.
7. **BRS_MAX_DEPTH=8 is a tested baseline.** If you change it, verify regression tests still pass.

---

## Known Limitations (DO NOT try to fix these)

These are known issues from prior stages. Document them but don't fix:

1. **Bootstrap eval weakness** — Static eval can't assess positional factors. Fixed by NNUE (Stages 14-16).
2. **BRS 0cp spread** — BRS sometimes can't distinguish moves. All survivors go to MCTS with thin budget.
3. **MCTS Q-value compression** — v1-v4 values cluster around 0.75. Needs better eval to differentiate.
4. **No piece activity in eval** — Bishop retreats, knight undevelopment aren't penalized. NNUE fixes this.

---

## Verification Checklist

Before declaring Stage 13 complete:
- [ ] `cargo test` — all 465+ tests pass (original 465 + new time management tests)
- [ ] `cargo clippy` — 0 warnings
- [ ] `go wtime 60000 winc 1000 btime 60000 binc 1000 ytime 60000 yinc 1000 gtime 60000 ginc 1000` works via protocol
- [ ] Engine doesn't flag in a 100-ply timed game (60s + 1s increment)
- [ ] Tactical positions receive more time than quiet positions (verified by info output)
- [ ] Forced moves (1 legal move) return instantly
- [ ] Safety cap: never uses >25% of remaining clock
- [ ] At least one parameter tuning run completed (even if results are neutral)
- [ ] Post-audit in `masterplan/audit_log_stage_13.md`
- [ ] `masterplan/STATUS.md` and `masterplan/HANDOFF.md` updated
