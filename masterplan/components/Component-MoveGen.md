---
type: component
stage: 2
tags:
  - stage/02
  - area/movegen
last_updated: 2026-02-20
---

# Component: Move Generation + Attack Query API

The layer that answers "what moves are legal?" and "what squares are attacked?" Built on top of [[Component-Board]].

## Purpose

Generates legal moves for the side to move. Provides attack queries used by check detection, legal filtering, and later by evaluation and search. Implements make/unmake for efficient move execution and reversal.

## Key Types

- **Move** -- compact `u32` encoding. Bits: 0-7 from, 8-15 to, 16-19 piece_type, 20-23 captured (7=none), 24-27 promotion (7=none), 28-30 flags.
- **MoveFlags** -- Normal=0, DoublePush=1, EnPassant=2, CastleKing=3, CastleQueen=4.
- **MoveUndo** -- stores captured_piece, castling_rights, en_passant, halfmove_clock, zobrist_before. Enables perfect state restoration.

## Public API

Attack queries:
- `is_square_attacked_by(sq, attacker, board) -> bool`
- `attackers_of(sq, attacker, board) -> Vec<(PieceType, Square)>`
- `is_in_check(player, board) -> bool` -- checks all 3 opponents

Move generation:
- `generate_pseudo_legal(board) -> Vec<Move>` -- ignores check legality
- `generate_legal(board) -> Vec<Move>` -- requires `&mut Board` for make/unmake
- `make_move(board, mv) -> MoveUndo` -- executes move, advances side_to_move
- `unmake_move(board, mv, undo)` -- restores exact prior state
- `perft(board, depth) -> u64` -- recursive legal move count
- `perft_divide(board, depth) -> Vec<(Move, u64)>` -- per-move breakdown

## Internal Design

- **Pre-computed attack tables** (`tables.rs`): Ray tables (8 directions per square), knight moves, king moves, pawn attacks (per player -- 4 different directions). All 196-indexed, rays stop at board edges AND corner boundaries.
- **Attack queries** (`attacks.rs`): Walk rays for sliding pieces, lookup tables for knight/king/pawn. Pawn reverse lookup uses `(player + 2) % 4` (see [[Pattern-Pawn-Reverse-Lookup]]).
- **Move generation** (`generate.rs`): Iterates player's piece list, generates moves per piece type. Pawns handle forward, double step, captures, en passant, promotion (to Queen, Rook, Bishop, Knight -- all as PromotedQueen variants). Castling for all 4 players.
- **Legal filtering**: Make move, check if own king is in check by any of 3 opponents, unmake. Expensive but correct.
- **Make/unmake** (`moves.rs`): Uses Board's `place_piece`/`remove_piece`/`move_piece` primitives to maintain Zobrist consistency. Handles special moves: castling (two-piece move), en passant (capture on different square), promotion (piece type change).

## Connections

- [[Component-Board]] -- consumed for all board state access and mutation
- [[stage_03_gamestate]] -- will use `is_in_check`, `generate_legal`, `make_move`/`unmake_move`
- [[stage_07_plain_brs]] -- will use `generate_legal` and `make_move`/`unmake_move` for search
- [[stage_09_tt_ordering]] -- will add move ordering on top of `generate_pseudo_legal`

## Huginn Gates

Specified but not yet wired (see [[Issue-Huginn-Gates-Unwired]]):
- `move_generation` -- track move counts and lists
- `make_unmake` -- verify state restoration
- `legality_filter` -- why moves were rejected
- `perft` -- depth/count verification

## Gotchas

1. **Pawn directions are per-player.** Red +rank, Blue +file, Yellow -rank, Green -file. Capture diagonals are relative to forward direction. See [[Pattern-Pawn-Reverse-Lookup]].
2. **En passant captured square uses pushing player's direction.** Not the capturing player's. The pushing player is `prev_player(capturing_player)` since EP lasts exactly one turn.
3. **Castling paths differ per player.** Each player has unique king position, rook positions, empty square requirements, and king path check requirements. Hardcoded in `castling_config()`.
4. **generate_legal requires `&mut Board`** because it uses make/unmake internally.
5. **Perft values are permanent invariants.** See [[Issue-Perft-Values-Unverified]] for verification status.

## Performance Notes

Not optimized -- correctness only per spec. Legal filtering does full make/check/unmake per pseudo-legal move. Attack tables use heap-allocated Vecs. Perft(4) = 152,050 nodes in ~0.56s (debug build).

## Related

- [[stage_02_movegen]] -- spec
- [[audit_log_stage_02]] -- audit findings
- [[downstream_log_stage_02]] -- API contracts
