---
type: connection
tags:
  - stage/02
  - stage/03
  - area/movegen
  - area/gamestate
last_updated: 2026-02-20
---

# Connection: MoveGen -> GameState

How [[Component-MoveGen]] feeds into [[Component-GameState]].

## What Connects

GameState depends on MoveGen for three categories of functionality: legal move generation, board mutation, and check/attack detection. MoveGen is consumed as a library -- GameState calls its public functions, passing the wrapped Board.

## How They Communicate

**Legal move generation:**
- `generate_legal(&mut board) -> Vec<Move>` -- GameState calls this to implement `legal_moves()`. Also called during elimination detection to determine if a player has any legal moves (checkmate vs stalemate distinction).
- `determine_status_at_turn(&mut board) -> ...` -- used to determine whether the current player is in check, checkmated, or stalemated at the start of their turn.

**Board mutation (one-way):**
- `make_move(&mut board, mv) -> MoveUndo` -- GameState calls this to apply moves permanently. **GameState does NOT use `unmake_move`.** At the game level, moves are irreversible -- the undo struct is discarded. This is a key difference from how search (Stage 7+) uses make/unmake for tree traversal.

**Check and attack detection:**
- `is_in_check(player, &board) -> bool` -- used after move execution to detect check delivery (for check bonuses in scoring) and during elimination chain processing.
- `is_square_attacked_by(sq, attacker, &board) -> bool` -- used for check bonus scoring and elimination detection. Determines if a specific player's king is under attack.

**Terrain interaction (GameState -> MoveGen, indirect):**
- GameState modifies MoveGen behavior by calling `Board::set_piece_status()` to mark pieces as terrain. MoveGen's attack and generation functions then treat terrain pieces differently -- they block movement and do not deliver check (see [[Pattern-Terrain-Awareness]]). This is an indirect connection: GameState mutates Board state, and MoveGen reads that state.

**DKW moves (temporary side_to_move manipulation):**
- For DKW instant moves, GameState saves `board.side_to_move()`, calls `board.set_side_to_move(dkw_player)`, generates king-only legal moves via MoveGen, picks a random move, calls `make_move`, then restores the original `side_to_move`. This is a legitimate but unusual usage pattern -- MoveGen generates moves for a player who is not truly "active" (see [[Pattern-DKW-Instant-Moves]]).

## Contract

1. GameState **never calls `unmake_move`**. All moves are permanent at the game level. Search (Stage 7+) owns the make/unmake lifecycle.
2. GameState passes `&mut Board` to MoveGen functions that need it. The Board reference comes from GameState's internal wrapped Board.
3. MoveGen's `make_move` advances `side_to_move` internally. GameState then further manages turn rotation by skipping eliminated players.
4. MoveGen does not know about game rules (scoring, elimination, DKW). It only knows about board state and legal moves. GameState is the rules layer on top.
5. Terrain awareness is implemented at the MoveGen level via `PieceStatus` checks, not at the GameState level. GameState only triggers the conversion; MoveGen enforces the movement restrictions.

## Evolution

- **Stage 7:** Search will use `make_move`/`unmake_move` through the Board directly, not through GameState. GameState wraps make_move for game-level concerns; search wraps it for tree traversal.
- **Stage 9:** Move ordering will consume `generate_pseudo_legal` and add scoring. GameState continues using `generate_legal`.
- **Stage 10:** MCTS will clone GameState and call `apply_move` for playout simulations. It does not touch MoveGen directly.
- **Stage 14+:** NNUE accumulator updates will hook into the make/unmake path at the MoveGen level, transparent to GameState.

## Related

- [[Component-MoveGen]]
- [[Component-GameState]]
- [[Connection-Board-to-MoveGen]] -- the lower-level Board->MoveGen connection
- [[downstream_log_stage_02]] -- MoveGen API contracts
- [[downstream_log_stage_03]] -- GameState API contracts
