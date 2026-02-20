---
type: connection
tags:
  - stage/01
  - stage/03
  - area/board
  - area/gamestate
last_updated: 2026-02-20
---

# Connection: Board -> GameState

How [[Component-Board]] feeds into [[Component-GameState]].

## What Connects

GameState wraps Board as its internal position representation. Board stores the piece layout, Zobrist hash, castling rights, en passant, side-to-move, and halfmove clock. GameState adds game-level state on top: scores, player statuses, position history, and game mode.

## How They Communicate

**Wrapping and access:**
- GameState owns a `Board` instance internally.
- `GameState::board(&self) -> &Board` -- exposes immutable access to downstream consumers (evaluation, protocol, UI).
- `GameState::board_mut(&mut self) -> &mut Board` -- exposes mutable access when direct board manipulation is needed.

**Piece status manipulation:**
- `Board::set_piece_status(sq, PieceStatus)` -- GameState calls this when a player is eliminated in terrain mode. The player's remaining pieces are converted from `Active` to `Terrain` status. This changes how MoveGen treats them (see [[Pattern-Terrain-Awareness]]).
- This is the primary mechanism for terrain conversion: GameState iterates the eliminated player's piece list and calls `set_piece_status` on each piece.

**Turn management:**
- `Board::set_side_to_move(Player)` -- GameState calls this to skip eliminated players in the turn rotation. When a player is eliminated, the next `apply_move` advances past them. Also used during DKW instant moves to temporarily set the side to move to the DKW player (see [[Pattern-DKW-Instant-Moves]]).
- `Board::side_to_move() -> Player` -- read by GameState to determine whose turn it is.

**Repetition detection:**
- `Board::zobrist() -> u64` -- GameState reads the Zobrist hash after every move and pushes it into `position_history: Vec<u64>`. Repetition (threefold or other) is detected by checking for duplicate hashes in the history.
- The hash is computed incrementally by Board's mutation methods. GameState trusts the hash is correct.

**50-move rule:**
- `Board::halfmove_clock() -> u16` -- GameState reads this to enforce the 50-move draw rule. The clock is incremented by `make_move` on every non-capture, non-pawn move. It is reset to 0 on captures and pawn moves.
- Note: DKW instant moves also go through `make_move`, which increments the halfmove clock. See [[Issue-DKW-Halfmove-Clock]].

## Contract

1. GameState **never accesses Board's internal array directly**. All access goes through the public API (`piece_at`, `king_square`, `piece_list`, etc.).
2. Board's mutation methods maintain Zobrist hash consistency automatically. GameState does not manually XOR Zobrist keys.
3. Board does not know about GameState concepts (scores, elimination, DKW). Board is a pure position container. GameState is the rules layer.
4. When GameState modifies piece status via `set_piece_status`, it changes Board state that MoveGen then reads. This is an indirect GameState -> MoveGen coupling through Board (see [[Connection-MoveGen-to-GameState]]).
5. The `position_history` lives in GameState, not in Board. Board only provides the current hash; GameState tracks the history.

## Evolution

- **Stage 4:** Protocol will construct GameState from FEN4 strings, using `Board::from_fen4()` internally.
- **Stage 6:** Evaluation will read Board state via `GameState::board()` for piece positions, material counts, etc.
- **Stage 10:** MCTS will clone GameState (including its Board) for playout simulations. Board's clone performance matters.
- **Stage 14+:** NNUE accumulator state may be stored alongside or within Board, adding to clone cost.
- **Stage 19:** Board internals may be optimized (bitboards) but the public API remains stable. GameState should not be affected.

## Related

- [[Component-Board]]
- [[Component-GameState]]
- [[Connection-Board-to-MoveGen]] -- the parallel Board->MoveGen connection
- [[downstream_log_stage_01]] -- Board API contracts
- [[downstream_log_stage_03]] -- GameState API contracts
