---
type: connection
tags: [stage/04, area/protocol, area/gamestate]
last_updated: 2026-02-20
---

# Connection-GameState-to-Protocol

## What Connects
[[Component-GameState]] (Stage 3) → [[Component-Protocol]] (Stage 4)

## How They Communicate
Protocol owns an `Option<GameState>`. On `position` commands, it creates a new GameState. On `go`, it calls `legal_moves()` and selects a move. Move strings from the protocol are matched against `to_algebraic()` of legal moves, then applied via `apply_move()`.

Key API calls:
- `GameState::new(board, game_mode, terrain_mode)` — from FEN4 positions
- `GameState::new_standard_ffa()` / `new_standard_ffa_terrain()` — from startpos
- `GameState::legal_moves()` — for move matching and go handler
- `GameState::apply_move(mv)` — for applying moves in position command
- `GameState::scores()` — for info string v1-v4 values
- `GameState::is_game_over()` — guard before applying moves

## Contract
1. Protocol MUST use `GameState::apply_move()` for position setup, not raw `make_move()`
2. Protocol MUST check `is_game_over()` before applying each move
3. Protocol MUST generate legal moves via `legal_moves()` to match move strings
4. `legal_moves()` requires `&mut self` — protocol holds mutable ownership

## Evolution
- **Stage 7:** Protocol's `handle_go()` will call `Searcher::search()` instead of picking random moves. The GameState will be passed to the searcher.
- **Stage 11:** Hybrid controller will receive GameState via protocol. Protocol may need to pass additional context from `go` command.
- **Stage 13:** Time management will consume `SearchLimits` from protocol alongside GameState.
