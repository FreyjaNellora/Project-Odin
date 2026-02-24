---
type: component
stage: 1
tags:
  - stage/01
  - area/board
last_updated: 2026-02-20
---

# Component: Board Representation

The foundational data structure everything else builds on. Represents the 14x14 four-player chess board with 160 valid squares and 36 invalid corners.

## Purpose

Stores the position: which pieces are where, whose turn it is, castling rights, en passant state, and the Zobrist hash. Every module from move generation to evaluation reads from this structure.

## Key Types

- **Board** -- the main struct. `[Option<Piece>; 196]` array + per-player piece lists + king tracking + Zobrist hash + game state fields.
- **Piece** -- `(PieceType, Player, PieceStatus)`. Seven piece types including PromotedQueen.
- **Player** -- `Red=0, Blue=1, Yellow=2, Green=3`. Clockwise turn order.
- **Square** -- `u8`, index via `rank * 14 + file`. 0-195 range, 160 valid.

## Public API

Core query: `piece_at(sq)`, `king_square(player)`, `piece_list(player)`, `zobrist()`, `side_to_move()`, `castling_rights()`, `en_passant()`.

Core mutation: `place_piece(sq, piece)`, `remove_piece(sq)`, `move_piece(from, to)`. All mutations automatically update the piece list, king tracker, and Zobrist hash.

State mutation: `set_castling_rights(u8)`, `set_en_passant(Option<Square>)`, `set_side_to_move(Player)`. All update Zobrist.

Serialization: `Board::from_fen4(str)`, `board.to_fen4()`.

Verification: `verify_zobrist()`, `verify_piece_lists()`.

## Internal Design

- **Square indexing:** Raw 196-element array indexed by `rank * 14 + file`. Invalid corners hold `None`. Validity checked via compile-time `VALID_SQUARES` table.
- **Piece lists:** `[Vec<(PieceType, Square)>; 4]` -- one per player. Kept in sync by all mutation methods. Enables fast iteration over a player's pieces without scanning the full board.
- **Zobrist hashing:** Global keys via `OnceLock`. Fixed PRNG seed `0x3243F6A8885A308D` for deterministic hashes. Keys for piece-square (5,488), castling (256), en passant (196), side-to-move (4).
- **FEN4 format:** Custom notation for 4PC. Player-prefixed pieces (RK, BQ), corner markers (x), castling A/a B/b C/c D/d.

## Connections

- [[Component-MoveGen]] consumes Board for move generation and make/unmake
- [[stage_03_gamestate]] will wrap Board in a GameState struct
- [[stage_04_protocol]] will parse FEN4 from external input
- [[stage_06_bootstrap_eval]] will read piece positions for evaluation

## Tracing Points

Potential `tracing` spans/events (Huginn was retired in Stage 8; see ADR-015):
- `board_mutation` -- track piece placement/removal
- `zobrist_update` -- trace hash XOR operations
- `fen4_roundtrip` -- verify parse/serialize consistency
- `piece_list_sync` -- catch array vs list desync

## Gotchas

1. **Always check `is_valid_square(sq)` before placing pieces.** Corner squares accept `None` but must never hold pieces.
2. **En passant stores a square index, not a file.** Changed in Stage 2 (see [[Issue-EP-Representation-4PC]]).
3. **King/Queen placement is asymmetric.** Blue and Yellow have K/Q swapped compared to Red and Green. See [[4PC_RULES_REFERENCE]].
4. **Zobrist keys include invalid squares.** 196 piece-square keys per type-player combo, not 160. The 36 unused keys waste ~288 bytes but avoid validity checks on every hash lookup.

## Related

- [[stage_01_board]] -- spec
- [[audit_log_stage_01]] -- audit findings
- [[downstream_log_stage_01]] -- API contracts
