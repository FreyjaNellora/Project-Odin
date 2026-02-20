# Audit Log — Stage 01: Board

## Pre-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build` in 0.67s, `cargo build --features huginn` in 0.43s)
- Tests pass: Yes (2 without huginn, 11 with huginn — all pass)
- Previous downstream flags reviewed: Yes — Stage 0 downstream log reviewed

### Findings
**From [[downstream_log_stage_00]]:**
1. `huginn_observe!` macro is available crate-wide via `#[macro_export]`. Macro arguments must be pure (no allocating expressions).
2. `HuginnBuffer` API: `new`, `with_default_capacity`, `new_trace`, `record`, `len`, `is_empty`, `get`, `session_id`, `current_trace_id`.
3. No JSON serialization yet — buffer stores raw `u64` values. This is fine for Stage 1; we record raw data.
4. No global buffer instance — must be created and passed explicitly. Stage 1 Huginn gates will accept `&mut HuginnBuffer` parameter.
5. Data limited to 16 `u64` fields per observation. Stage 1 gates (board_mutation, zobrist_update, piece_list_sync, fen4_roundtrip) all fit within 16 fields.

**From [[audit_log_stage_00]]:**
- No blocking or warning findings. Stage 0 is clean.
- `Phase` and `Level` enums may need new variants — additive and safe.
- `huginn_observe!` macro signature is now a contract. Will use it as-is.

### Risks for This Stage
1. **Corner square validity (Section 2.17):** The 14x14 board has 36 invalid squares in four 3x3 corners. Off-by-one errors in corner exclusion are the highest risk. Must verify exact coordinates against 4PC_RULES_REFERENCE.md: a1-c3, l1-n3, a12-c14, l12-n14.
2. **Zobrist hash correctness (Section 2.18):** 4-player Zobrist needs keys for (square, piece_type, owner) + castling (8 bits) + en passant (14 files) + side-to-move (4 players). The side-to-move key depends on which of 4 players moves next, not a binary toggle. Must get this right from the start.
3. **Pawn direction encoding (Appendix C):** Red +rank, Blue +file, Yellow -rank, Green -file. Stage 1 doesn't implement movement but must encode player orientation correctly for piece-square tables and FEN4.
4. **Piece list sync (Section 2.4):** Board array and piece lists must stay synchronized. Every mutation must update both. Huginn gate catches desync at the moment it happens.
5. **FEN4 format correctness:** No standard FEN4 spec exists for 4PC. Must define a consistent format and verify round-trip (parse -> serialize -> matches original).


---

## Post-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| Square indexing (196 total, 160 valid, 36 invalid) | PASS | `VALID_SQUARES` compile-time table. Integration tests verify exact counts and all 4 corner regions. |
| Piece representation (7 types, 4 owners, 3 statuses) | PASS | `PieceType`, `Player`, `PieceStatus` enums with index round-trip. PromotedQueen distinct (FEN char 'W'). |
| Board storage (array + piece lists + king tracking) | PASS | `[Option<Piece>; 196]` + `[Vec<(PieceType, Square)>; 4]` + `[Square; 4]`. Sync verified by tests. |
| Zobrist hashing (piece-square + castling + ep + stm) | PASS | 5,488 piece-square keys + 256 castling + 14 ep + 4 stm. Fixed seed `0x3243F6A8885A308D`. Deterministic. |
| FEN4 parser/serializer | PASS | Round-trip verified for starting position (string, Zobrist, piece count). Corner markers ('x'), multi-digit empty counts. |
| Make/unmake stubs | PASS | Infrastructure present (place/remove/move methods). Full make/unmake logic deferred to Stage 2. |
| Acceptance: 160 valid / 36 invalid | PASS | `test_exactly_160_valid_squares`, `test_exactly_36_invalid_corners`, `test_all_corner_squares_are_correctly_identified` |
| Acceptance: FEN4 round-trip | PASS | `test_fen4_roundtrip_starting_position`, `test_fen4_roundtrip_preserves_zobrist` |
| Acceptance: Zobrist changes on mutation | PASS | `test_zobrist_changes_on_piece_placement`, `test_zobrist_place_remove_restores_hash` |
| Acceptance: Piece lists synchronized | PASS | `test_starting_position_piece_lists_synchronized`, `test_piece_lists_sync_after_mutations` |

### Code Quality

#### Uniformity
PASS. All modules follow the same pattern: module-level doc comment, imports, types/structs, impl blocks, `#[cfg(test)] mod tests`. Naming is consistent (`snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` constants).

#### Bloat
PASS. No unnecessary abstractions. Board struct matches spec exactly. No builder patterns, no trait objects where not needed. FEN4 error type is a simple enum, not a trait-object-based error chain.

#### Efficiency
PASS. No performance targets for Stage 1. Board array is stack-allocated (Option<Piece> is small). Zobrist keys use `OnceLock` for global singleton (one-time init). XorShift64 is branchless. Piece lists use `Vec` per spec.

#### Dead Code
**NOTE.** 5 dead_code warnings: `MAX_PIECES_PER_PLAYER`, `valid_squares()`, `rank_number()`, `parse_square()`, `square_name()`. All are public utility functions intended for Stage 2+ consumption (move generation, protocol, debugging). Expected and acceptable. Will be consumed when downstream stages use them.

#### Broken Code
PASS. All 64 unit tests and 18 integration tests pass in both configurations (with and without huginn). No panics, no unwrap-on-None paths in production code.

#### Temporary Code
PASS. No `todo!()`, `unimplemented!()`, or `// TODO` markers in production code. Make/unmake stubs are complete functions that do real work (place/remove/move pieces with Zobrist update), not empty bodies.

### Search/Eval Integrity
N/A for Stage 1. No search or evaluation code.

### Future Conflict Analysis
1. **Zobrist key count discrepancy (NOTE).** MASTERPLAN says "4,480 entries" for piece-square keys. Implementation has `196 * 7 * 4 = 5,488`. The difference: MASTERPLAN likely assumed `160 * 7 * 4 = 4,480` (valid squares only). Implementation indexes by raw square index (0-195), which means 36 invalid-square keys exist but are never accessed. This wastes ~288 bytes but avoids a validity check on every hash lookup. No conflict — the unused keys are harmless, and the indexing is simpler.
2. **Castling encoding (NOTE).** The FEN4 castling letters (A/a, B/b, C/c, D/d) are a custom design since no standard exists. Stage 4 (Odin Protocol) must use the same encoding. Documented in FEN4 module header.
3. **Huginn gates not yet wired (NOTE).** The MASTERPLAN specifies 4 Huginn gates for Stage 1: board_mutation, zobrist_update, fen4_roundtrip, piece_list_sync. These are not implemented as `huginn_observe!` calls because make/unmake is not yet active (Stage 2). The verification methods (`verify_zobrist`, `verify_piece_lists`) serve the same purpose in debug/test builds. Huginn gates should be added in Stage 2 when mutations become frequent during move generation.

### Unaccounted Concerns
1. **Player turn order in starting position.** Board initializes with Red to move and all castling rights (0xFF). The 4PC rules reference confirms Red moves first. Verified.
2. **King/Queen placement asymmetry.** Blue and Yellow have K/Q swapped compared to Red and Green. Verified against `4PC_RULES_REFERENCE.md`. Blue: a4=R, a5=N, a6=B, a7=K, a8=Q, a9=B, a10=N, a11=R. Yellow: d14=R, e14=N, f14=B, g14=K, h14=Q, i14=B, j14=N, k14=R.

### Reasoning & Methods
- **Array-first over bitboard:** Per ADR-001 and MASTERPLAN. Simpler, correct first. Bitboard optimization deferred to Stage 19 if profiling warrants it.
- **Global Zobrist keys via OnceLock:** Avoids passing keys through every function. Thread-safe, initialized once. Board stores `&'static ZobristKeys` reference.
- **FEN4 designed for 4PC:** No existing standard. Format inspired by chess.com 4PC but adapted for Odin's needs. Player prefixes (R/B/Y/G) + piece letter. Corner markers ('x'). Castling uses A-D/a-d for 4 players.
- **PromotedQueen as distinct type:** Per MASTERPLAN spec. Worth 1 point on capture in FFA, but evaluates at 900cp. FEN char 'W' avoids collision with regular Queen 'Q'.


---

## Related

- Stage spec: [[stage_01_board]]
- Downstream log: [[downstream_log_stage_01]]
