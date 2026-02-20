# Downstream Log — Stage 02: MoveGen

## Notes for Future Stages

### Must-Know

1. **Attack query API is the board boundary (ADR-001).** Use `is_square_attacked_by(sq, player, board)` and `attackers_of(sq, player, board)` for all attack/check queries. Never read `board.squares[]` directly from above Stage 2. These are pub via `odin_engine::movegen`.

2. **En passant stores the target square (not a file).** `board.en_passant() -> Option<Square>`. Changed from `Option<u8>` (file) in Stage 1. Zobrist EP keys are indexed by square (196 keys, not 14).

3. **En passant lasts exactly one turn.** EP is cleared at the start of every `make_move`. Only the immediately-next player can capture EP. The captured pawn's location is computed using `prev_player(capturing_player)` to determine the pushing player's direction.

4. **Pawn directions:** Red +rank, Blue +file, Yellow -rank, Green -file. Capture diagonals are relative to forward direction. Promotion ranks: Red at rank 8, Blue at file 8, Yellow at rank 5, Green at file 5 (all 0-indexed).

5. **Board derives Clone.** Added in this stage to support legal filtering (make/check/unmake).

6. **Move encoding:** `Move(u32)` with bits 0-7 from, 8-15 to, 16-19 piece_type, 20-23 captured (7=none), 24-27 promotion (7=none), 28-30 flags. Flags: 0=normal, 1=double_push, 2=en_passant, 3=castle_king, 4=castle_queen.

7. **MoveUndo contains:** captured_piece (full `Piece` with owner), castling_rights, en_passant, halfmove_clock, zobrist_before. The `Move` + `MoveUndo` together provide full "what changed" information for delta refresh.

### API Contracts

| Function | Signature | Notes |
|---|---|---|
| `is_square_attacked_by` | `(sq: Square, attacker: Player, board: &Board) -> bool` | O(N) in piece count along rays |
| `attackers_of` | `(sq: Square, attacker: Player, board: &Board) -> Vec<(PieceType, Square)>` | Returns all attackers |
| `is_in_check` | `(player: Player, board: &Board) -> bool` | Checks vs all 3 opponents |
| `generate_pseudo_legal` | `(board: &Board) -> Vec<Move>` | Ignores check legality |
| `generate_legal` | `(board: &mut Board) -> Vec<Move>` | Requires `&mut` for make/unmake |
| `make_move` | `(board: &mut Board, mv: Move) -> MoveUndo` | Advances side_to_move |
| `unmake_move` | `(board: &mut Board, mv: Move, undo: MoveUndo)` | Restores exact prior state |
| `perft` | `(board: &mut Board, depth: u32) -> u64` | Recursive legal move count |
| `perft_divide` | `(board: &mut Board, depth: u32) -> Vec<(Move, u64)>` | Per-move breakdown |

### Known Limitations

1. **No move ordering.** Moves are generated in piece-list order. Stage 9 adds ordering.
2. **Legal filtering is expensive.** Every pseudo-legal move is made/checked/unmade. Pin detection optimization deferred to later.
3. **Attack tables use heap-allocated Vecs.** Could be replaced with fixed-size arrays for performance. Not a priority.
4. **No game-level rulings.** No checkmate, stalemate, draw, or elimination detection. Stage 3 builds on top of this.
5. **Huginn gates not wired.** The spec lists 4 Huginn gates for this stage; none are implemented yet. Will add when telemetry infrastructure is needed.
6. **Perft values not independently verified.** No reference 4PC engine exists. Values are self-consistent but unverified externally.

### Performance Baselines

| Metric | Value | Build |
|---|---|---|
| perft(1) | 20 nodes | debug |
| perft(2) | 395 nodes | debug |
| perft(3) | 7,800 nodes | debug |
| perft(4) | 152,050 nodes / ~0.56s | debug |
| 1000 random games @ 100 ply | ~15s total | debug |

### Open Questions

1. **Castling "kingside" definition:** In 4PC, each player's back rank has RNBQKBNR (or equivalent). "Kingside" = toward the rook on the same side as the king. Current implementation uses this convention. If 4PC rules define kingside differently, castling configs need updating.
2. **Promotion to full Queen vs PromotedQueen:** Current implementation promotes to PromotedQueen (1-pt queen per FFA rules). If game mode changes (e.g., teams mode), promotion type may need to be configurable.

### Reasoning

1. **Why `(player + 2) % 4` for reverse pawn lookup:** Pawn attacks are directional. To find which pawns attack a square, we need the reverse capture direction. In 4PC with 4 orthogonal directions, the reverse of player P's captures equals the opposite-facing player's captures. Red↔Yellow, Blue↔Green, which is `(P + 2) % 4`.

2. **Why `prev_player` for EP captured square:** EP target is the midpoint of a double push. The captured pawn is at `target + pusher's forward`. Since EP lasts one turn, the pusher is always the player who moved just before the capturer: `prev_player(capturer)`.



---

## Related

- Stage spec: [[stage_02_movegen]]
- Audit log: [[audit_log_stage_02]]
