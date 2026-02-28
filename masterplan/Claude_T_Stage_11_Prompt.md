# Claude.T Prompt — Stage 11: Hybrid Integration (BRS -> MCTS)

You are building Stage 11 of Project Odin, a four-player chess engine. This stage composes the BRS (Best Reply Search) and MCTS (Monte Carlo Tree Search) into a hybrid controller that runs BRS first for tactical filtering, then MCTS for strategic search.

---

## MANDATORY: Read These Files First (Stage Entry Protocol)

Follow AGENT_CONDUCT.md Section 1.1 exactly. Read in this order:

1. `masterplan/STATUS.md` — Current project state
2. `masterplan/HANDOFF.md` — Last session summary and what's next
3. `masterplan/MASTERPLAN.md` lines 908-990 — Stage 11 full specification
4. `masterplan/DECISIONS.md` — Read ADR-016 (Gumbel MCTS) and ADR-017 (Progressive History)
5. `masterplan/downstream_log_stage_10.md` — MCTS API contracts, must-know items
6. `masterplan/downstream_log_stage_09.md` — BRS/TT API contracts, history table details
7. `masterplan/audit_log_stage_10.md` — Stage 10 audit findings

Then build and run all tests to confirm baseline:
```
cd odin-engine && cargo build && cargo test
```
Expected: 440 passed, 0 failed, 4 ignored.

Complete the pre-audit section of `masterplan/audit_log_stage_11.md` before writing any code.

---

## What You're Building

A `HybridController` in `odin-engine/src/search/hybrid.rs` that:

1. Implements the `Searcher` trait (frozen: `fn search(&mut self, &GameState, SearchBudget) -> SearchResult`)
2. Owns a `BrsSearcher` and `MctsSearcher` internally
3. Orchestrates a two-phase search per the controller flow in MASTERPLAN Stage 11

### Controller Flow

```
search(position, budget) -> SearchResult:
    legal_moves = generate_legal_moves(position)
    if len == 0: return no-move. if len == 1: return it immediately.

    // Phase 1: BRS tactical filter
    brs_budget = allocate_brs_budget(budget, position_type)
    brs_result = brs_searcher.search(position, brs_budget)
    // Extract BRS knowledge AFTER search completes
    history = brs_searcher.history_table()   // NEW accessor needed
    surviving = filter_survivors(legal_moves, brs_scores, TACTICAL_MARGIN=150cp)
    // Always keep at least 2 survivors so MCTS has a choice.
    emit info "phase brs" lines during BRS search

    if surviving.len() == 1: return it.

    // Handoff: compute priors and pass BRS knowledge to MCTS
    priors = softmax(ordering_scores / PRIOR_TEMPERATURE) for surviving moves
    mcts_searcher.set_history_table(history)
    mcts_searcher.set_prior_policy(priors)

    // Phase 2: MCTS strategic search
    mcts_budget = remaining budget after Phase 1
    mcts_result = mcts_searcher.search(position, mcts_budget)
    emit info "phase mcts" lines during MCTS search

    return mcts_result (or blend/override logic)
```

---

## Build Order (Follow This Sequence)

### Step 1: BRS History Table Accessor

**Problem:** BRS history lives in private `BrsContext`, created inside `search()`, dropped when search returns. Stage 11 needs to extract it.

**Solution:** After `BrsContext::iterative_deepening()` completes but before context is dropped, copy the history table into `BrsSearcher`. Then expose it via a public accessor.

In `odin-engine/src/search/brs.rs`:
- Add a field to `BrsSearcher`: `last_history: Option<Box<HistoryTable>>`
- At the end of `search()` (after `ctx.iterative_deepening()` returns, before ctx drops), copy: `self.last_history = Some(Box::new(ctx.history));`
- Add public accessor: `pub fn history_table(&self) -> Option<&HistoryTable>`
- Import `HistoryTable` from `search::mcts` (it's already `pub type HistoryTable = [[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT]`)

**Key files:**
- `odin-engine/src/search/brs.rs` — BrsSearcher struct (line 98), search() method (line 137), BrsContext.history field (line 192)
- `odin-engine/src/search/mcts.rs` — HistoryTable type alias (line ~495)

### Step 2: Wire external_priors Into MCTS

**Problem:** `set_prior_policy()` stores priors in `self.external_priors` but the search loop still calls `compute_priors()` and ignores external priors.

**Solution:** In `MctsSearcher::search()`, after root expansion, if `self.external_priors` is `Some`, use those priors instead of (or blended with) `compute_priors()` output. The external priors from Stage 11 are BRS-informed softmax scores — they should REPLACE the MVV-LVA priors when available.

**Key location:** `odin-engine/src/search/mcts.rs` — search() method (~line 606-620) where root children are created with priors.

**Important:** External priors may cover only surviving moves (not all legal moves). The prior array indices correspond to surviving move indices. Handle the mapping correctly — surviving moves are a subset of legal_moves.

### Step 3: HybridController Skeleton

Create `odin-engine/src/search/hybrid.rs`:
- `pub struct HybridController` — owns `BrsSearcher`, `MctsSearcher`, config
- Implement `Searcher` trait
- Config constants: `TACTICAL_MARGIN: i16 = 150` (cp), `PRIOR_TEMPERATURE: f32 = 50.0`

Register the module in `odin-engine/src/search/mod.rs` with `pub mod hybrid;`

### Step 4: Surviving Move Filter

After BRS Phase 1, determine which moves survive for MCTS:
- Run BRS search to get best_move and score
- To get scores for ALL moves, you need per-move scores from BRS. **Design decision:** Run BRS normally (it returns the best move/score), then use the root node's move ordering + TT to estimate scores for alternative moves. OR: modify BRS to expose root move scores during iterative deepening.
- **Simpler approach:** Use the PV move's score as the baseline. For other moves, use the history heuristic scores as a proxy for relative quality. Any move with history score > 0 (caused a cutoff at some point) survives. Always keep top-2 by history score plus the PV move.
- **Alternative (recommended):** Add a `root_move_scores` field to BrsContext that records the alpha-beta score for each root move at the last completed depth. Extract alongside history. This gives exact scores for filtering.

**TACTICAL_MARGIN = 150cp:** Any move within 150cp of the best BRS score survives. Minimum 2 survivors always.

### Step 5: Prior Policy Computation

For surviving moves, compute priors:
```
pi(a) = softmax(ordering_score(a) / PRIOR_TEMPERATURE)
```
Where `ordering_score(a)` comes from BRS move ordering infrastructure (history heuristic scores, capture values, etc.). PRIOR_TEMPERATURE = 50.0 (from ADR-016).

### Step 6: Time/Budget Allocation

Split the SearchBudget between BRS and MCTS phases:

**Fixed split first (get it working):**
- BRS: 15% of time budget (or depth-limited to depth 6)
- MCTS: 85% of remaining time, convert to simulation count

**Then adaptive (Step 7):**
- Tactical positions (many captures/checks in legal moves): 30% BRS, 70% MCTS
- Quiet positions: 10% BRS, 90% MCTS
- If BRS spread < 50cp (can't distinguish moves): give more to MCTS

**Budget conversion:** For MCTS, convert remaining milliseconds to sim count. Use baseline: ~8000 sims/sec (from Stage 10 performance data). So 1000ms remaining = ~8000 sims budget.

### Step 7: Protocol Wiring

In `odin-engine/src/protocol/mod.rs`:
- Change `searcher: Option<BrsSearcher>` (line 39) to `searcher: Option<Box<dyn Searcher>>`
- Create `HybridController` instead of bare `BrsSearcher` at line 281-282
- Pass the info callback through to HybridController, which forwards to both sub-searchers
- `HybridController::new()` takes `evaluator` and creates both internal searchers

**Important:** The `set_info_callback` method needs to work on the trait object. Either add it to the `Searcher` trait or handle it within HybridController's constructor.

### Step 8: Unified Info Output

During search:
- Phase 1 (BRS): emit `info ... phase brs` lines (BrsSearcher already does this)
- Phase transition: emit `info string hybrid phase1 done survivors N threshold Xcp time Yms`
- Phase 2 (MCTS): emit `info ... phase mcts` lines (MctsSearcher already does this)

### Step 9: Edge Cases

- **One legal move:** Return immediately, no search needed
- **Zero surviving moves after filter:** Return BRS best move (should never happen if min survivors = 2)
- **BRS times out during Phase 1:** Use whatever partial result BRS produced, pass all moves to MCTS
- **Time pressure:** If total budget < 100ms, skip MCTS entirely, return BRS result

---

## Acceptance Criteria (from MASTERPLAN)

- **AC1:** Hybrid finds better moves than BRS alone or MCTS alone (test positions or observer self-play)
- **AC2:** BRS phase correctly filters losing moves
- **AC3:** MCTS phase respects the surviving move set
- **AC4:** Time allocation adapts to position type
- **AC5:** Engine never crashes or returns illegal moves under time pressure
- **AC6:** History table successfully transfers from BRS to MCTS (nonzero entries, measurable)
- **AC7:** MCTS with Progressive History warm-start outperforms cold-start MCTS (A/B comparison)

---

## Test File

Create `odin-engine/tests/stage_11_hybrid.rs`. Tests should cover:

1. **AC1 — Hybrid vs standalone comparison:** Run hybrid search AND standalone BRS on same tactical positions. Hybrid should find at least as many best moves.
2. **AC2 — Survivor filtering:** Given known BRS scores, verify correct moves survive the 150cp threshold. Verify minimum 2 survivors.
3. **AC3 — MCTS respects survivors:** Verify MCTS only considers surviving moves (best move is always from the survivor set).
4. **AC4 — Adaptive time split:** Verify tactical positions get more BRS time, quiet positions get more MCTS time.
5. **AC5 — No crashes under pressure:** Run hybrid with very small budgets (10ms, 1 node). Must return a legal move.
6. **AC6 — History handoff:** After hybrid search, verify history table was non-empty when passed to MCTS.
7. **AC7 — Progressive History improvement:** Compare MCTS-with-history vs MCTS-cold-start on same position. History version should use fewer sims to find the same move (or find a better move with same sims).
8. **Edge cases:** One legal move returns instantly. Time pressure skips MCTS.
9. **Protocol integration:** Send `go depth 8` through protocol, verify hybrid runs both phases and returns bestmove.
10. **Regression:** All 440 existing tests still pass.

---

## Files You Will Create or Modify

| File | Action | What |
|------|--------|------|
| `odin-engine/src/search/hybrid.rs` | CREATE | HybridController struct + Searcher impl |
| `odin-engine/src/search/mod.rs` | MODIFY | Add `pub mod hybrid;` |
| `odin-engine/src/search/brs.rs` | MODIFY | Add `last_history` field, history extraction in search(), `history_table()` accessor, optionally `root_move_scores` |
| `odin-engine/src/search/mcts.rs` | MODIFY | Wire `external_priors` into root expansion |
| `odin-engine/src/protocol/mod.rs` | MODIFY | Change searcher type to Box<dyn Searcher>, create HybridController |
| `odin-engine/tests/stage_11_hybrid.rs` | CREATE | Integration tests for all AC |
| `masterplan/audit_log_stage_11.md` | MODIFY | Fill pre-audit before coding, post-audit after |
| `masterplan/downstream_log_stage_11.md` | MODIFY | Fill after implementation |

---

## What You DON'T Need (Scope Boundaries)

- **NNUE** (Stage 16). Both phases use bootstrap eval via `Evaluator` trait.
- **Pondering** (Stage 13). Just allocate the given time budget.
- **Persistent history across moves** (measure in Stage 13).
- **Opening book or endgame tables.** Not until Stage 17.
- **Arena-based MCTS tree.** Not until Stage 19.
- **Full recursive SEE.** Current simplified SEE is fine. Stage 19.

---

## Critical Invariants (Do Not Break)

- Perft values: perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050
- Zobrist make/unmake round-trip
- Turn order R→B→Y→G
- Searcher trait signature frozen
- TT probe MUST come AFTER repetition check in alphabeta()
- All 440 existing tests must pass after your changes

---

## Key Source File Locations

| What | File | Key Lines |
|------|------|-----------|
| BrsSearcher struct | `src/search/brs.rs` | ~98-104 |
| BrsContext struct + history field | `src/search/brs.rs` | ~153-200 (history at ~192) |
| BrsSearcher::search() | `src/search/brs.rs` | ~137-141 |
| BrsContext::new() (history zeroed) | `src/search/brs.rs` | ~203-234 (history at ~230) |
| History update (depth² bonus) | `src/search/brs.rs` | ~637-641 |
| MctsSearcher struct | `src/search/mcts.rs` | ~506-523 |
| set_prior_policy() stub | `src/search/mcts.rs` | ~571-573 |
| set_history_table() | `src/search/mcts.rs` | ~576-578 |
| HistoryTable type alias | `src/search/mcts.rs` | ~493-495 |
| compute_priors() (MVV-LVA) | `src/search/mcts.rs` | ~147-167 |
| MCTS search() root expansion | `src/search/mcts.rs` | ~606-620 |
| Searcher trait | `src/search/mod.rs` | ~54-59 |
| SearchBudget struct | `src/search/mod.rs` | ~22-29 |
| SearchResult struct | `src/search/mod.rs` | ~32-44 |
| Protocol go handler | `src/protocol/mod.rs` | ~280-285 |
| Protocol searcher field | `src/protocol/mod.rs` | ~39 |

---

## Session End Protocol

When done, per AGENT_CONDUCT.md:

1. Fill `masterplan/audit_log_stage_11.md` post-audit section (deliverables check, code quality, search/eval integrity, future conflict analysis)
2. Fill `masterplan/downstream_log_stage_11.md` (must-know items, API contracts, known limitations, performance baselines, open questions)
3. Update `masterplan/STATUS.md` and `masterplan/HANDOFF.md`
4. Create session note in `masterplan/sessions/`
5. Run `cargo test` — all tests must pass (existing 440 + new Stage 11 tests)
6. Run `cargo clippy` — zero warnings

Do NOT tag the stage or push to git. The human will review first.
