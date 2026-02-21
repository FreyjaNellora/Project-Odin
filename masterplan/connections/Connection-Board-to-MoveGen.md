---
type: connection
tags:
  - stage/01
  - stage/02
  - area/board
  - area/movegen
last_updated: 2026-02-20
---

# Connection: Board -> MoveGen

How the [[Component-Board]] feeds into [[Component-MoveGen]].

## What Connects

Board provides the position state. MoveGen reads it to generate moves, attack queries, and legal filtering. MoveGen also **mutates** Board via make/unmake.

## How They Communicate

**Board -> MoveGen (read path):**
- `board.piece_list(player)` -- iterate pieces for move generation
- `board.piece_at(sq)` -- check destination squares for captures/blocking
- `board.king_square(player)` -- find king for check detection
- `board.side_to_move()` -- determine whose moves to generate
- `board.castling_rights()` -- check castling availability
- `board.en_passant()` -- check en passant target

**MoveGen -> Board (write path):**
- `board.place_piece(sq, piece)` -- used by make_move for promotions, castling
- `board.remove_piece(sq)` -- used by make_move for captures, EP captures
- `board.move_piece(from, to)` -- used by make_move for normal moves
- `board.set_castling_rights(u8)` -- updated during make_move
- `board.set_en_passant(Option<Square>)` -- set/clear during make_move
- `board.set_side_to_move(Player)` -- advanced during make_move

**MoveUndo -> Board (restore path):**
- `unmake_move` uses the undo struct to restore exact prior state including Zobrist hash

## Contract

MoveGen **never reads board.squares[] directly** -- always uses the public API methods. This is per ADR-001: the attack query API is the board boundary.

Board's mutation methods automatically maintain Zobrist hash, piece lists, and king tracking. MoveGen relies on this -- it does not manually XOR Zobrist keys.

## Evolution

- Stage 9 adds move ordering on top of `generate_pseudo_legal`
- Stage 14+ NNUE accumulator updates will hook into the make/unmake path
- Stage 19 may optimize Board internals (bitboards) but the public API stays stable

## Related

- [[Component-Board]]
- [[Component-MoveGen]]
- [[downstream_log_stage_01]] -- Board API contracts
- [[downstream_log_stage_02]] -- MoveGen API contracts
