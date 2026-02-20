# Audit Log — Stage 02: MoveGen

## Pre-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build` in 1.00s, `cargo build --features huginn` in 0.46s)
- Tests pass: Yes (64 without huginn — 44 unit + 2 stage-00 + 18 stage-01; all pass)
- Previous downstream flags reviewed: Yes — Stage 0 and Stage 1 downstream logs reviewed

### Findings

**From [[downstream_log_stage_01]]:**
1. Board uses `[Option<Piece>; 196]` array, index = `rank * 14 + file`. Must check `is_valid_square(sq)` before accessing.
2. Piece lists are `Vec<(PieceType, Square)>` per player. Must use `place_piece`/`remove_piece`/`move_piece` to maintain sync.
3. King squares tracked in `king_squares: [Square; 4]`. Updated automatically by Board methods.
4. Zobrist hash incrementally maintained by all Board mutation methods.
5. **REQUIRES CHANGE:** `en_passant: Option<u8>` stores a file index (0-13). This is insufficient for 4PC — Blue/Green pawns move along files, not ranks. En passant target squares for Blue/Green are on specific ranks, not identifiable by file alone. Must change to store full target square (`Option<Square>`). Zobrist ep keys need expansion from 14 (one per file) to 196 (one per square index).
6. Piece lists use `Vec`, not fixed-size arrays. Acceptable for correctness-first approach.
7. Huginn gates not yet wired — will add when make/unmake becomes active per Stage 1 downstream log recommendation.

**From [[downstream_log_stage_00]]:**
1. `huginn_observe!` macro available crate-wide. Arguments must be pure.
2. `HuginnBuffer` API: `new`, `record`, etc. No global buffer instance.

**From [[audit_log_stage_01]]:**
1. No blocking or warning findings.
2. 5 dead_code warnings — utility functions awaiting Stage 2 consumption. Several will be consumed this stage.
3. Zobrist key count uses 196 squares (includes invalid corners) for simpler indexing — no conflict.
4. Castling encoding (A/a B/b C/c D/d) is custom. Must use same encoding.

**From [[audit_log_stage_00]]:**
- No blocking or warning findings. Stage 0 is clean.

### Risks for This Stage

1. **Board geometry errors (Section 2.17):** Rays for sliding pieces must stop at board edges AND corner boundaries. A bishop sliding toward a corner zone must stop before entering it. Knight destinations near corners may land on invalid squares. This is the highest-risk area.
2. **Pawn direction bugs (Appendix C):** Red +rank, Blue +file, Yellow -rank, Green -file. Each player has unique forward/capture directions. A sign error means one player's pawns move backward.
3. **En passant for side players (Appendix C):** Current en_passant representation stores file only, which doesn't identify Blue/Green ep targets. Must fix as a prerequisite.
4. **Castling for 4 players (Appendix C):** 8 castling rights (2 per player). Each player has different king/rook positions and different castling paths. King must not pass through or into check from ANY of the other 3 players.
5. **Three-way check (Appendix C):** Legal filtering must check if the moving player's king is attacked by ANY of the other 3 opponents.
6. **Zobrist make/unmake round-trip (Section 2.18):** Make/unmake must perfectly restore the hash. This is a Stage 2 permanent invariant.
7. **Perft correctness (Section 4.1):** Perft values become permanent invariants once established.
8. **Performance not a goal:** Spec explicitly says "Correctness is the only goal. If perft is slow, that's fine."


---

## Post-Audit
**Date:**
**Auditor:**

### Deliverables Check


### Code Quality
#### Uniformity

#### Bloat

#### Efficiency

#### Dead Code

#### Broken Code

#### Temporary Code


### Search/Eval Integrity


### Future Conflict Analysis


### Unaccounted Concerns


### Reasoning & Methods


---

## Related

- Stage spec: [[stage_02_movegen]]
- Downstream log: [[downstream_log_stage_02]]
