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
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| Pre-computed attack tables (rays, knight, king) | DONE | `tables.rs` — 196-square indexed, rays stop at corners. Pawn attack tables per player. Lazy global init via `OnceLock`. |
| Attack query API | DONE | `attacks.rs` — `is_square_attacked_by`, `attackers_of`, `is_in_check`. Uses reverse pawn lookup `(player + 2) % 4`. |
| Pseudo-legal generation | DONE | `generate.rs` — all piece types, 4-direction pawns, double step, en passant, promotion (4 options), castling (4 players). |
| Legal move filtering | DONE | `generate.rs` — make/check-king/unmake. Checks all 3 opponents. |
| Move encoding | DONE | `moves.rs` — compact u32 with from/to/piece/captured/promotion/flags. Debug/Display via algebraic notation. |
| Make/unmake with undo | DONE | `moves.rs` — `MoveUndo` stores captured piece, castling rights, en passant, halfmove, zobrist_before. Zobrist round-trip verified. |
| Perft validation depths 1-4 | DONE | Pinned: depth 1=20, 2=395, 3=7800, 4=152050. Integration tests assert exact values. |

**Acceptance criteria:**
- [x] Perft at depths 1-4 matches established values (permanent invariants)
- [x] All special moves tested: castling for 4 players, en passant, promotions
- [x] Make -> unmake returns board to identical state (Zobrist matches)
- [x] Stress test: 1000 random game playouts at 100 ply each without crashes

### Code Quality
#### Uniformity
- Consistent naming: `generate_*` for move gen, `compute_*` for table building, `is_*`/`has_*` for queries
- All movegen files follow same structure: module doc comment, imports, constants, functions, tests
- Square/piece/player types used consistently from `board` module

#### Bloat
- No unnecessary abstractions. Each file has a single clear responsibility.
- `get_castling_config` returns a tuple — slightly unwieldy but avoids exposing internal `CastlingConfig` struct. Acceptable for now.

#### Efficiency
- Not a goal per spec. Legal move gen uses make/check/unmake (expensive but correct).
- Attack tables use `Vec<Vec<Square>>` — heap-allocated but simple. Performance optimization deferred to later stages if needed.
- Perft(4) = 152,050 nodes in ~0.56s (debug build). Acceptable.

#### Dead Code
- **NOTE:** Huginn gates listed in spec (move_generation, make_unmake, legality_filter, perft) are not yet wired. Deferred per pre-audit finding #7 — will add when Huginn gates become relevant.
- `prev_player` in `moves.rs` is an internal utility. No dead public API.

#### Broken Code
- No known broken code. All 125 tests pass.

#### Temporary Code
- None. All code is production-intent for this stage.

### Search/Eval Integrity
- N/A for Stage 2. Search and eval not yet implemented.
- Attack query API (`is_square_attacked_by`, `attackers_of`) is the formal boundary per ADR-001. Future stages must use this, not read `board.squares[]` directly.

### Future Conflict Analysis
1. **Stage 3 (Game State):** Will consume `is_in_check`, `generate_legal`, `make_move`/`unmake_move`. All are pub and stable. No conflicts expected.
2. **Stage 8 (Delta Refresh):** Will need to know "what changed" from a move. `Move` + `MoveUndo` expose from/to/piece_type/captured/promotion/flags. The spec says "make it easily derivable" — this is sufficient.
3. **Stage 14 (NNUE Accumulator):** Will need piece placement/removal deltas. `Move` encoding has from/to/captured, which is sufficient for feature diff computation.
4. **En passant semantics:** The `en_passant_captured_sq` function uses `prev_player(capturing_player)` to determine the pushing player. This assumes EP only lasts one turn (cleared at start of every make_move). If game rules change to allow multi-turn EP persistence, this breaks. Current FFA rules confirm one-turn EP.
5. **Castling configurations are hardcoded** in `castling_config()`. If the starting position layout changes, these need updating. The FEN4 starting position and castling config must stay in sync.

### Unaccounted Concerns
1. **WARNING:** Perft values have not been independently verified against another 4PC engine. No reference implementation exists for FFA 4-player chess perft. Values are self-consistent (board restored, zobrist round-trips, 1000 random playouts pass) but could contain a systematic error that preserves consistency while producing wrong move counts. The most likely source would be incorrect pawn promotion ranks or castling configurations.
2. **NOTE:** The `en_passant` unit test in `generate.rs::test_en_passant_move_generated` uses a manually-set EP state that wouldn't arise in normal play (Red capturing Yellow's EP when prev_player(Red) = Green). This test only verifies pseudo-legal generation (not make/unmake), so it passes, but the game state is technically invalid. The integration test `test_en_passant_capture` uses a valid state.

### Reasoning & Methods
- **Pawn attack reverse lookup:** The attack query needs to find pawns that attack a target square. Pawn attacks are asymmetric (forward only). To check "does player P's pawn attack square S?", we look at squares from which player P could capture to S. This is the reverse of P's capture direction, which equals the opposite-facing player's capture direction: `(P + 2) % 4`.
- **En passant captured square:** In 4PC, the capturing player may face a different direction than the pushing player. The captured pawn is always at `ep_target + pushing_player's forward`. The pushing player is `prev_player(capturing_player)` since EP lasts exactly one turn.
- **Perft verification:** Board state restoration verified via Zobrist hash comparison at every depth. Piece list integrity verified. 1000 random playouts (deterministic PRNG) exercise diverse game lines. No reference values exist for 4PC FFA, so values are established as permanent invariants from this implementation.


---

## Related

- Stage spec: [[stage_02_movegen]]
- Downstream log: [[downstream_log_stage_02]]
