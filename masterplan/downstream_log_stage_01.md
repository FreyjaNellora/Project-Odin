# Downstream Log — Stage 01: Board Representation

## Notes for Future Stages

### Must-Know

1. **Board is a 196-element `Option<Piece>` array.** Index = `rank * 14 + file`. 36 corner squares are `None` and marked invalid in `VALID_SQUARES` table. Always check `is_valid_square(sq)` before accessing.

2. **Piece lists are per-player `Vec<(PieceType, Square)>`.** Kept in sync with the array by `place_piece`, `remove_piece`, and `move_piece`. Stage 2 must use these methods (not raw array access) to maintain sync.

3. **King squares tracked in `king_squares: [Square; 4]`.** Indexed by `Player::index()`. Updated automatically by `place_piece`/`move_piece` when the piece is a King.

4. **Zobrist hash is incrementally maintained.** Every `place_piece`, `remove_piece`, `move_piece`, `set_castling_rights`, `set_en_passant`, and `set_side_to_move` call XORs the appropriate key. The hash is always current after any Board method call.

5. **Zobrist keys are global via `OnceLock`.** Access through `Board::empty()` or `Board::starting_position()` — both initialize the keys on first call. The `&'static ZobristKeys` reference is stored in every Board instance.

6. **FEN4 format is custom.** No external standard. Format: `<ranks> <side> <castling> <ep> <halfmove> <fullmove>`. Ranks top-to-bottom, player prefix + piece letter (e.g., `RK` = Red King), corners as `x`, castling as A/a B/b C/c D/d.

### API Contracts

**`Board`** public methods:
- `Board::empty() -> Board` — Empty board, Red to move, no castling, no ep.
- `Board::starting_position() -> Board` — Full 4PC starting position, 64 pieces, all castling, Red to move.
- `board.piece_at(sq: u8) -> Option<Piece>` — What's on this square?
- `board.piece_list(player: Player) -> &[(PieceType, Square)]` — All pieces for a player.
- `board.king_square(player: Player) -> Square` — King location.
- `board.zobrist() -> u64` — Current Zobrist hash.
- `board.side_to_move() -> Player` — Whose turn.
- `board.castling_rights() -> u8` — 8-bit castling flags.
- `board.en_passant() -> Option<u8>` — En passant file (0-13) or None.
- `board.halfmove_clock() -> u16` / `board.fullmove_number() -> u16`
- `board.place_piece(sq, piece)` — Place piece, update array + piece list + Zobrist + king tracking.
- `board.remove_piece(sq) -> Option<Piece>` — Remove piece, update everything.
- `board.move_piece(from, to)` — Move piece between squares, update everything.
- `board.set_castling_rights(rights: u8)` — Update castling with Zobrist delta.
- `board.set_en_passant(file: Option<u8>)` — Update ep with Zobrist delta.
- `board.set_side_to_move(player: Player)` — Update side with Zobrist delta.
- `board.compute_full_hash() -> u64` — Recompute hash from scratch (for verification).
- `board.verify_zobrist() -> bool` — Check incremental hash matches full recompute.
- `board.verify_piece_lists() -> bool` — Check piece lists match array contents.
- `board.piece_count() -> usize` — Total pieces on board.
- `Board::from_fen4(fen: &str) -> Result<Board, Fen4Error>` — Parse FEN4 string.
- `board.to_fen4() -> String` — Serialize to FEN4.

**Castling bit constants** (exported from `board_struct`):
- `CASTLE_RED_KING = 0x01`, `CASTLE_RED_QUEEN = 0x02`
- `CASTLE_BLUE_KING = 0x04`, `CASTLE_BLUE_QUEEN = 0x08`
- `CASTLE_YELLOW_KING = 0x10`, `CASTLE_YELLOW_QUEEN = 0x20`
- `CASTLE_GREEN_KING = 0x40`, `CASTLE_GREEN_QUEEN = 0x80`

**Square utilities** (exported from `square`):
- `square_from(file, rank) -> Option<Square>` — Bounds-checked square creation.
- `file_of(sq) -> u8`, `rank_of(sq) -> u8` — Extract coordinates.
- `is_valid_square(sq) -> bool` — Corner check.
- `parse_file(c: char) -> Option<u8>` — 'a'-'n' → 0-13.
- `parse_square(s: &str) -> Option<Square>` — "e4" → square index.
- `square_name(sq) -> String` — Square index → "e4".

**Player** enum: `Red=0, Blue=1, Yellow=2, Green=3`. `player.index()` for array indexing. `player.next()` for turn rotation (clockwise: R→B→Y→G→R).

**PieceType** enum: `Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4, King=5, PromotedQueen=6`. FEN chars: `P N B R Q K W`.

### Known Limitations

1. **No move generation.** Board can place/remove/move pieces but has no concept of legal moves. Stage 2 provides this.

2. **No game rules.** No check detection, no checkmate, no stalemate, no draw rules. Stage 3 provides this.

3. **Piece lists use `Vec`, not fixed-size arrays.** This means heap allocation per player. If profiling shows this is a bottleneck in make/unmake (Stage 2), consider switching to fixed-capacity `ArrayVec<(PieceType, Square), 16>`.

4. **~~Huginn gates not yet wired.~~** *(Historical — Huginn was retired in Stage 8 and replaced with the `tracing` crate; see ADR-015.)* Verification methods exist (`verify_zobrist`, `verify_piece_lists`) for debug use.

5. **No `Clone` or `Copy` on Board.** Board contains `Vec` fields (piece lists) and a static reference. If make/unmake in Stage 2 needs board copying, derive or implement `Clone`.

### Performance Baselines

| Metric | Value | Notes |
|---|---|---|
| `cargo build` (incremental) | ~0.18s | Dev profile |
| `cargo build --release` | ~0.33s | Binary: 129,024 bytes |
| Test count | 64 | 44 unit + 2 stage-00 + 18 stage-01 |
| Clippy warnings | 5 dead_code | Utility functions awaiting Stage 2+ consumers |

### Open Questions

1. **Should Stage 2 add `Clone` to Board?** Make/unmake can either clone the board (simpler but slower) or use an undo stack (faster, more complex). ADR-001 and MASTERPLAN favor undo stack, but `Clone` may be useful for testing.

### Reasoning

- **Raw square index (0-195) over valid-only index (0-159):** Avoids remapping overhead on every square access. The 36 wasted `Option<Piece>` slots (36 bytes) are negligible. Validity check is a single array lookup.
- **Per-player piece lists as `Vec`:** The spec requires fast iteration over a player's pieces for evaluation and move generation. A flat `Vec` is cache-friendly and supports O(1) push. Removal is O(n) where n ≤ 16, which is fast enough.
- **`OnceLock` for global Zobrist keys:** Avoids threading a `&ZobristKeys` through every function signature. The keys are truly global (same for all boards), initialized once, never modified.
- **FEN4 with player prefixes:** Unambiguous piece ownership. Standard FEN uses case (upper=white, lower=black) which doesn't extend to 4 players. Player prefix (R/B/Y/G) followed by piece letter is explicit and parseable.

---

## Related

- Stage spec: [[stage_01_board]]
- Audit log: [[audit_log_stage_01]]
