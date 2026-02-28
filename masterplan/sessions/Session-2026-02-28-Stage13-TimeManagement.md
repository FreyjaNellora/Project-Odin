# Session: Stage 13 — Time Management

**Date:** 2026-02-28
**Agent:** Claude Opus 4.6
**Stage:** 13
**Duration:** Single session (continued from context overflow)

## Summary

Implemented the full Stage 13 deliverable set: position-aware time allocation (TimeManager), enriched position classification, increment parsing, tunable parameters via `setoption`, timed match support, and parameter tuning script. All acceptance criteria (AC1-AC3) met.

## Changes

### TimeManager (`search/time_manager.rs`)
Core deliverable — pure-function time allocator:
- `TimeContext` struct carries clock info from protocol to search
- `TimeManager::allocate()` formula: `base_time = remaining / moves_left + increment`
- Multiplicative factors: tactical ×1.3, quiet ×0.8, near-elimination ×2.0, in-check ×1.2, forced →0
- Safety: 25% cap, 100ms min, panic mode (<1s: 10% max)
- Two-layer design: protocol extracts clock, HybridController applies position context

### Enriched Position Classification (`search/hybrid.rs`)
- `PositionType` enum expanded: +Endgame (piece_count ≤16), +Forced (1 legal move)
- `classify_position()` now uses `is_in_check()`, `piece_count()`, legal move count
- Forced move fast path returns immediately (0 nodes, 0 depth)

### Protocol Integration (`protocol/types.rs`, `parser.rs`, `mod.rs`)
- Increment parsing: `winc`/`binc`/`yinc`/`ginc`/`movestogo`
- `time_for_player(Player)` method on SearchLimits
- Fixed `limits_to_budget()` player-time mapping bug (`.or()` chain picked wrong player)
- 5 tunable params via `setoption`: `tactical_margin`, `brs_fraction_tactical`, `brs_fraction_quiet`, `mcts_default_sims`, `brs_max_depth`

### HybridController Overrides (`search/hybrid.rs`)
- `time_context: Option<TimeContext>` — consumed via `.take()` per search
- `last_score: Option<i16>` — updated after each search
- 5 override fields shadow compile-time constants when set via `apply_options()`
- `effective_*()` methods provide seamless override resolution

### Match Manager Time Control (`observer/match.mjs`)
- `time_control: { initial_ms, increment_ms }` config option
- Per-player clock tracking with subtract-elapsed + add-increment
- Time forfeit detection
- `go wtime/btime/ytime/gtime` command generation

### Parameter Tuning (`observer/tune.mjs`)
- CLI: `node tune.mjs --param tactical_margin --values 100,150,200,250 --games 50`
- A/B match: engine A with setoption override vs engine B with defaults
- Elo difference reporting per value with 95% CI

## Key Design Decisions

1. **Two-layer time allocation** — Protocol extracts clock, HybridController applies position context. Avoids coupling protocol to GameState.
2. **Override fields as `Option<T>`** — Zero cost when unused (common case). Only tuning runs set them.
3. **TimeContext consumed via `.take()`** — One-shot per search. If not set, search uses raw budget unchanged.
4. **Player-aware time mapping** — Fixed real bug where `.or()` chain could pick wrong player's time.

## Test Results
- Before: 465 (281 unit + 184 integration, 5 ignored)
- After: 490 (292 unit + 198 integration, 5 ignored)
  - +11 time_manager unit tests
  - +12 stage_13 integration tests
  - +2 parser unit tests
- Clippy: 0 warnings

## Links
- Audit: [[audit_log_stage_13]]
- Downstream: [[downstream_log_stage_13]]
- Status: [[STATUS]]
- Handoff: [[HANDOFF]]
