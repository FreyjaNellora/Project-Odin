---
type: component
stage_introduced: 4
tags: [stage/04, area/protocol]
status: active
last_updated: 2026-02-20
---

# Component-Protocol

## Purpose
The Odin Protocol is the UCI-like text protocol extended for four-player chess. It handles command parsing from stdin and response formatting to stdout, providing the communication layer between the engine core and frontends (Stage 5 UI).

## Key Types
- **`OdinEngine`** — Protocol handler struct. Owns `Option<GameState>`, engine options, RNG seed, and output buffer. Runs the main stdin loop.
- **`Command`** — Enum of all parsed commands: Odin, IsReady, SetOption, PositionFen4, PositionStartpos, Go, Stop, Quit, Unknown.
- **`SearchLimits`** — Time controls and search limits from `go` command: wtime/btime/ytime/gtime, depth, nodes, movetime, infinite.
- **`EngineOptions`** — Runtime options: debug mode, terrain mode.
- **`SearchInfo`** — All optional fields for `info` string formatting (depth, score, v1-v4, nodes, nps, time, pv, phase, brs_surviving, mcts_sims).

## Public API
- `OdinEngine::new() -> Self` — create with default settings
- `OdinEngine::run(&mut self)` — main stdin loop
- `OdinEngine::handle_command(&mut self, Command) -> bool` — process one command (returns true on Quit)
- `OdinEngine::take_output(&mut self) -> Vec<String>` — for testing
- `OdinEngine::game_state(&self) -> Option<&GameState>` — for testing
- `parse_command(&str) -> Command` — parse raw input line

## Internal Design
- **Parser** (`parser.rs`): Tokenizes input, matches command keyword, delegates to specialized parsers. FEN4 strings are reconstructed by joining tokens between `fen4` and `moves`.
- **Emitter** (`emitter.rs`): Pure formatting functions. `format_info` builds output string from optional fields — only present fields appear.
- **Move matching**: No `Move::from_algebraic()`. Instead, generate legal moves and match `to_algebraic()` output against input string. Standard UCI approach.
- **Random move**: LCG with seed `0x0D14_CAFE_0000_BEEF`, same algorithm as DKW and test infrastructure.

## Connections
- Depends on: [[Component-GameState]], [[Component-Board]], [[Component-MoveGen]]
- Depended on by: Stage 5 Basic UI (spawns engine as child process)
- Communicates via: stdin/stdout text protocol, [[Component-GameState]] API

## Huginn Gates
Defined but not wired (deferred per established pattern):
- `command_receive` — raw string, parsed type, parse errors
- `response_send` — full string, what triggered it
- `position_set` — FEN4/startpos, move list, resulting hash
- `search_request` — time controls, depth limits, options

## Gotchas
- `position ... moves` uses `GameState::apply_move()` (game-aware), not raw `make_move()`
- `GameState::legal_moves()` requires `&mut self`
- `GameState::apply_move()` panics if game is over — protocol guards against this
- No threading — `go` blocks, `stop` is a no-op

## Performance Notes
Not applicable for Stage 4. Protocol is not in any hot path.

## Known Issues
- [[Issue-Huginn-Gates-Unwired]] — now includes 4 Stage 4 gates

## Build History
- [[Session-2026-02-20-Stage04]] — initial implementation
