---
type: session
date: 2026-02-20
stage: 4
tags:
  - stage/04
  - area/protocol
---

# Session: Stage 4 — Odin Protocol

**Date:** 2026-02-20
**Stage:** 4 — Odin Protocol
**Agent:** Claude Opus 4.6

## Summary

Implemented the Odin Protocol — a UCI-like text protocol extended for four-player chess. The protocol reads commands from stdin and writes responses to stdout. The `go` command returns a random legal move (actual search is Stage 7). Established the permanent invariant: "Protocol round-trip works — send position + go, get legal bestmove back."

## What Was Built

### Module Structure
```
odin-engine/src/protocol/
  mod.rs       — OdinEngine struct, command handlers, main loop (17 unit tests)
  parser.rs    — parse_command(): raw string → Command enum (23 unit tests)
  emitter.rs   — Response formatting functions (8 unit tests)
  types.rs     — Command, SearchLimits, EngineOptions types
```

### Key Design Decisions
- **Move matching via legal move list:** No `Move::from_algebraic()` exists. Protocol generates all legal moves, matches `to_algebraic()` output against input strings. Standard UCI approach.
- **LCG random for go stub:** `seed * 6364136223846793005 + 1`, consistent with DKW and test infrastructure.
- **Output buffer for testing:** `Vec<String>` on OdinEngine struct, captured via `take_output()`. Simplest approach — no trait abstraction needed.
- **No threading:** `go` is instant (random move). Threading deferred to Stage 7 when actual search is added.

### Commands Implemented
`odin`, `isready`, `setoption`, `position` (startpos + fen4 + moves), `go` (with SearchLimits), `stop`, `quit`

### 4PC Extensions
- Time controls: `wtime/btime/ytime/gtime`
- Per-player values: `v1 v2 v3 v4`
- Search phase: `phase` (brs/mcts)
- BRS survivors: `brs_surviving`
- MCTS simulations: `mcts_sims`

## Problems Encountered

1. **Invalid hex literal:** Initial RNG seed `0x0D1N_CAFE_4PC0_BEEF` contained non-hex digits (N, P). Fixed to `0x0D14_CAFE_0000_BEEF`.
2. **Unused import warning:** `crate::movegen` imported at module level but only used in tests. Moved to `#[cfg(test)]` block.
3. **Clippy derivable_impls:** Manual `Default` impls for `SearchInfo` and `EngineOptions` flagged. Replaced with `#[derive(Default)]`.

## Test Results

229 total tests (156 unit + 73 integration), all passing.
- 48 new unit tests (23 parser + 8 emitter + 17 engine)
- 17 new integration tests (3 permanent invariant + 5 acceptance + 9 edge cases/prior invariants)
- All prior-stage tests preserved and passing.

## Commits

1. `[Stage 04] Command parser with error handling` — types.rs + parser.rs
2. `[Stage 04] Protocol engine, emitter, position setting, go command, main loop` — emitter.rs, mod.rs rewrite, main.rs, lib.rs, board/mod.rs
3. `[Stage 04] Integration tests and protocol round-trip invariant` — stage_04_protocol.rs

## Files Changed

| File | Action |
|---|---|
| `odin-engine/src/protocol/types.rs` | Created |
| `odin-engine/src/protocol/parser.rs` | Created |
| `odin-engine/src/protocol/emitter.rs` | Created |
| `odin-engine/src/protocol/mod.rs` | Rewritten |
| `odin-engine/src/main.rs` | Modified |
| `odin-engine/src/lib.rs` | Modified |
| `odin-engine/src/board/mod.rs` | Modified |
| `odin-engine/tests/stage_04_protocol.rs` | Created |

## Vault Notes Created

- [[Component-Protocol]] — Protocol module documentation
- [[Connection-GameState-to-Protocol]] — GameState → Protocol integration

## Related

- Stage spec: [[stage_04_protocol]]
- Audit log: [[audit_log_stage_04]]
- Downstream log: [[downstream_log_stage_04]]
