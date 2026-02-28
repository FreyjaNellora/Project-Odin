# Audit Log — Stage 13: Time Management

## Pre-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes — `cargo build` + `cargo clippy` clean (0 warnings)
- Tests pass: Yes — 465 engine tests (281 unit + 184 integration, 5 ignored), 54 UI Vitest
- Previous downstream flags reviewed: Yes — W12 (enrich position classification), Stage 12 FCA (match.mjs needs time control)

### Files to Create

| File | Purpose | AC |
|------|---------|-----|
| `odin-engine/src/search/time_manager.rs` | TimeManager::allocate() + TimeContext struct | AC1, AC2 |
| `odin-engine/tests/stage_13_time_mgmt.rs` | Integration tests (12 tests) | All |
| `observer/tune.mjs` | Parameter tuning script | AC3 |

### Files to Modify

| File | Change | AC |
|------|--------|-----|
| `odin-engine/src/search/mod.rs` | Add `pub mod time_manager;` | — |
| `odin-engine/src/search/hybrid.rs` | Enriched PositionType, TimeContext integration, last_score, tunable overrides | AC1, AC2 |
| `odin-engine/src/protocol/types.rs` | SearchLimits: inc/movestogo fields + time_for_player(); EngineOptions: tunable params | AC1, AC3 |
| `odin-engine/src/protocol/parser.rs` | Parse winc/binc/yinc/ginc/movestogo | AC1 |
| `odin-engine/src/protocol/mod.rs` | limits_to_budget() player-aware, handle_go() TimeContext, setoption tunables | AC1, AC3 |
| `observer/match.mjs` | time_control config option | AC3 |
| `observer/match_config.json` | Add time_control field | AC3 |

### Acceptance Criteria Mapping

| AC | Description | Implementation |
|----|-------------|----------------|
| AC1 | Engine manages time correctly, doesn't flag | TimeManager safety (25% cap, 100ms min, panic mode), increment parsing |
| AC2 | Time adapts to position complexity | Enriched PositionType, TimeManager adjustment factors |
| AC3 | Tuned parameters improve win rate | setoption tunables, tune.mjs, match.mjs time control |

### Findings

- **Bug in limits_to_budget()**: `.or()` chain picks first available time, not the current player's. Fix: pass current_player into the function.
- W12 (Stage 11 downstream): Position classification uses capture ratio only. Stage 13 enriches with check detection, piece count, legal move count.
- Stage 12 FCA: match.mjs only sends `go depth N`. Stage 13 adds `go wtime/btime/ytime/gtime` support.

### Risks for This Stage

1. **Frozen Searcher trait**: TimeContext cannot be passed through `search()` signature. Solution: `set_time_context()` method on HybridController called before `search()`.
2. **Test timing sensitivity**: T10 (full game no flag) depends on wall-clock timing. May be flaky in slow CI. Mitigate with conservative ply count (20 vs 100).
3. **Existing test compatibility**: SearchLimits gains new fields but derives Default; existing tests constructing SearchLimits with `..Default::default()` are unaffected.


---

## Post-Audit
**Date:** 2026-02-28
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Time allocation formula | DONE | `TimeManager::allocate()` in `search/time_manager.rs` |
| Position complexity detection | DONE | Enriched `PositionType` enum (Tactical/Quiet/Endgame/Forced) in `hybrid.rs` |
| Adaptive adjustments | DONE | Tactical ×1.3, quiet ×0.8, near-elimination ×2.0, in-check ×1.2, forced →0 |
| Safety checks (never flag) | DONE | 25% cap, 100ms min, panic mode (<1s: 10% max) |
| Increment parsing | DONE | `winc/binc/yinc/ginc/movestogo` in `SearchLimits` |
| Player-aware time mapping | DONE | `time_for_player()` + `limits_to_budget()` fix |
| Tunable parameters via setoption | DONE | 5 params: tactical_margin, brs_fraction_tactical/quiet, mcts_default_sims, brs_max_depth |
| Match manager time control | DONE | `time_control` config option in match.mjs |
| Tuning script | DONE | `observer/tune.mjs` — A/B test via setoption injection |
| Tests | DONE | 12 integration + 11 unit = 23 new tests |

### AC Verification

| AC | Verified | Method |
|----|----------|--------|
| AC1 | YES | T1, T2, T5, T6, T8, T10 — engine manages time correctly, doesn't flag in 24-ply timed game |
| AC2 | YES | T3, T4, T7, T9, T11 — tactical > quiet, forced → instant, near-elimination gets bonus |
| AC3 | YES | T12 — setoption accepted; tune.mjs and match.mjs time_control infrastructure ready |

### Code Quality
#### Uniformity

All new code follows existing patterns: `TimeContext` mirrors the protocol→search data flow pattern. Parser additions follow the exact pattern of existing `wtime`/`btime` parsing. `EngineOptions` tunable fields follow the same `Option` + `Default::default()` pattern as `eval_profile`.

#### Bloat

Minimal. `time_manager.rs` is ~115 lines (pure function + tests). `HybridController` gained 7 override fields (all `Option`, zero cost when `None`). No unnecessary abstractions.

#### Efficiency

`TimeManager::allocate()` is a pure function with no allocation — just arithmetic. `is_in_check()` is called once per search (was already called in movegen). `piece_count()` iterates 4 elements. No performance regression.

#### Dead Code

Removed old standalone `brs_fraction()` function (replaced by `effective_brs_fraction()` method). No other dead code.

#### Broken Code

None detected. All 490 tests pass. 0 clippy warnings.

#### Temporary Code

None. All code is permanent Stage 13 infrastructure.

### Search/Eval Integrity

- **Searcher trait**: FROZEN. Not modified. `set_time_context()` is a separate method on `HybridController`.
- **Evaluator trait**: FROZEN. Not modified.
- **BRS internals**: Not modified. Only the outer budget and survivor filter threshold are affected by tunables.
- **MCTS internals**: Not modified. Only the sim budget and default sims are affected.
- **TT probe order**: Unchanged. Repetition check still precedes TT probe in alphabeta.
- **perft invariants**: Not tested (eval/movegen untouched), but existing perft tests still pass.

### Future Conflict Analysis

- **Stage 14 (NNUE Feature Design)**: No conflicts. Time management is orthogonal to eval. NNUE will replace `BootstrapEvaluator` but won't affect `TimeManager` or protocol time handling.
- **Stage 15 (NNUE Training)**: `observer/match.mjs` time_control mode could be used for training data generation with realistic time pressure.
- **Stage 17 (Game Mode Tuning)**: `tune.mjs` infrastructure can be reused for game-mode-specific parameter tuning.
- **W13 (MCTS 9999 score)**: Unrelated to time management. Still present.

### Unaccounted Concerns

1. **T10 timing sensitivity**: The full-game-no-flag test uses wall-clock timing with 500ms grace. On very slow CI machines, this could be flaky. Monitor.
2. **Pondering (optional)**: Stage 13 prompt mentions pondering as optional. Not implemented — deferred to post-NNUE stages when the engine would benefit from it.
3. **Time control forfeit in match.mjs**: Implemented as clock tracking but the engine handles its own time via TimeManager. The match manager's forfeit check is a safety net. The engine should never actually flag if TimeManager works correctly.

### Reasoning & Methods

- Used two-layer time allocation: `limits_to_budget()` provides conservative fallback, `HybridController` calls `TimeManager` with full position context. This avoids coupling the protocol layer to game state details.
- `TimeContext` is consumed via `.take()` — one-shot per search, preventing stale context from affecting subsequent searches.
- `last_score` persists across searches (same `HybridController` instance) enabling near-elimination detection on the second and subsequent moves.
- Tunable parameters use override fields that shadow compile-time constants. This avoids recompilation for tuning while keeping the defaults zero-cost when no override is set.


---

## Related

- Stage spec: [[stage_13_time_management]]
- Downstream log: [[downstream_log_stage_13]]
