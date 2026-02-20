---
type: component
stage_introduced: 3
tags:
  - stage/03
  - area/gamestate
status: active
last_updated: 2026-02-20
---

# Component: GameState

The game-level wrapper around [[Component-Board]] that enforces 4PC rules, manages scoring, elimination, turn rotation, and game-over detection. This is the authoritative source of game state for all downstream consumers.

## Purpose

Provides a complete, rules-correct game simulation layer. While [[Component-Board]] stores the position and [[Component-MoveGen]] generates legal moves, GameState is responsible for everything that makes it a *game*: scores, eliminations, turn skipping, DKW instant moves, terrain conversions, and determining when the game is over.

## Files

- `gamestate/mod.rs` -- GameState struct, turn management, apply_move, elimination pipeline, DKW handling, game-over detection
- `gamestate/scoring.rs` -- point values for captures, check bonuses, stalemate awards
- `gamestate/rules.rs` -- elimination reasons, game mode, player status tracking, move result classification

## Key Types

- **GameState** -- wraps Board, adds scores, player statuses, position history, game mode, move history. The primary struct all downstream stages consume.
- **PlayerStatus** -- `Active`, `Eliminated(EliminationReason)`. Tracks each player's participation state.
- **EliminationReason** -- `Checkmated`, `Stalemated`, `Resigned`, `TimedOut`, `DkwKingStuck`. Why a player was removed from the game.
- **GameMode** -- `FFA` (Free For All), `Teams`, `DKW` (Dead King Walking), `Terrain`. Selects which rule variants are active.
- **MoveResult** -- classifies what happened after a move: normal, capture (with points), check bonus, elimination triggered, game over.

## Public API

Construction:
- `GameState::new() -> GameState` -- creates a new game from starting position with default scores and all players active

Board access:
- `board(&self) -> &Board` -- immutable reference to the wrapped board
- `board_mut(&mut self) -> &mut Board` -- mutable reference for direct board manipulation

Game queries:
- `current_player(&self) -> Player` -- whose turn it is (skips eliminated players)
- `scores(&self) -> &[i32; 4]` -- current scores for all four players
- `is_game_over(&self) -> bool` -- true when the game has ended
- `winner(&self) -> Option<Player>` -- the winning player, if the game is over

Move execution:
- `apply_move(&mut self, mv: Move) -> MoveResult` -- the primary entry point: executes a move and processes all consequences
- `legal_moves(&mut self) -> Vec<Move>` -- generates legal moves for the current player

Player management:
- `resign_player(&mut self, player: Player)` -- eliminates a player by resignation
- `timeout_player(&mut self, player: Player)` -- eliminates a player by timeout

Cloning:
- `clone(&self) -> GameState` -- deep copy for MCTS simulations and search

## Internal Flow: apply_move

The `apply_move` method is the core of GameState. Its internal sequence is:

1. **make_move** -- delegates to MoveGen's `make_move()` to mutate the board
2. **Score capture** -- if a piece was captured, award points to the capturing player per the scoring table
3. **Score check bonus** -- if the move delivers check, award the check bonus (+1 in FFA)
4. **Push history** -- record the board's Zobrist hash in `position_history` for repetition detection
5. **Advance turn** -- move to the next active player, skipping eliminated players
6. **Check elimination chain** -- for each player, check if they are checkmated or stalemated; if so, trigger elimination with scoring (checkmate awards points to the mater, stalemate awards 20 points to the stalemated player in FFA)
7. **Process DKW** -- if the game mode includes DKW, handle dead king walking instant moves for any newly eliminated player whose king remains on the board (see [[Pattern-DKW-Instant-Moves]])
8. **Check game-over** -- if only one player (or one team) remains active, the game is over

## Connections

- Depends on: [[Component-Board]], [[Component-MoveGen]]
- Depended on by: [[stage_04_protocol]], [[stage_06_bootstrap_eval]], [[stage_07_plain_brs]], [[stage_10_mcts]]
- Communicates via: [[Connection-Board-to-GameState]], [[Connection-MoveGen-to-GameState]]

## Huginn Gates

Specified in [[MASTERPLAN]] Stage 3:
- `turn_transition` -- track turn rotation, skipped players, and reasons
- `check_detection` -- which king tested, what attackers found
- `checkmate_stalemate` -- check/stalemate rulings
- `elimination` -- reason, points awarded, terrain conversions, DKW status
- `scoring` -- who scored, how many points, action type, running totals
- `dkw_move` -- DKW king position, move selected, legal move set
- `game_over` -- termination condition, final scores, winner

## Gotchas

1. **apply_move uses make_move but NOT unmake_move.** Moves are permanent at the game level. Unmake is only used by MoveGen internally for legal filtering and by search for tree traversal.
2. **Elimination can cascade.** Checkmating player A may expose player B to checkmate from player C. The elimination chain must process all players.
3. **DKW moves go through make_move.** This increments `halfmove_clock`, which may cause premature 50-move rule triggers (see [[Issue-DKW-Halfmove-Clock]]).
4. **Terrain pieces are set via Board::set_piece_status().** When a player is eliminated in terrain mode, their pieces become terrain. This affects MoveGen behavior -- terrain pieces block movement and do not give check (see [[Pattern-Terrain-Awareness]]).
5. **Scores are separate from evaluation.** Capture points (1 for pawn, 3 for bishop, etc.) are game-level scoring per 4PC rules. Centipawn eval values (100cp for pawn, 300cp for bishop, etc.) are search-level and handled by the Evaluator trait in Stage 6+.

## Performance Notes

GameState is designed to be cheaply cloneable for MCTS simulations. Position history uses `Vec<u64>` which grows unboundedly in long games -- see [[AGENT_CONDUCT]] Section 2.15 for the memory concern.

## Known Issues

- [[Issue-DKW-Halfmove-Clock]] -- DKW instant moves increment halfmove_clock (open, note-level)

## Build History

- [[Session-2026-02-20-Stage03]] -- initial implementation

## Related

- [[stage_03_gamestate]] -- spec
- [[audit_log_stage_03]] -- audit findings
- [[downstream_log_stage_03]] -- API contracts
