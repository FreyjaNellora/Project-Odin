# Audit Log — Stage 06: BootstrapEval

## Pre-Audit
**Date:** 2026-02-21
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build` and `cargo build --features huginn`)
- Tests pass: Yes (229 total: 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04)
- Previous downstream flags reviewed: Stages 0-3 (full dependency chain: Stage 6 → 3 → 2 → 1 → 0)

### Findings

**From Stage 0 downstream log:**
- Huginn macro and buffer available. `Phase::Eval` variant exists. No gates wired yet (deferred per [[Issue-Huginn-Gates-Unwired]]). Stage 6 will define `eval_call` and `eval_comparison` gate schemas but not wire them (following established pattern).

**From Stage 1 downstream log:**
- `Board::piece_list(player)` returns `&[(PieceType, Square)]` — primary iteration target for material/positional scoring. Does NOT include piece status; must use `board.piece_at(sq)` to check alive/dead/terrain.
- `Board::king_square(player)` returns king location — needed for king safety.
- PromotedQueen is distinct type (FEN char 'W'). Worth 1 point on capture in FFA but evaluates at 900cp in search. **Critical dual-value split.**

**From Stage 2 downstream log:**
- WARNING ([[Issue-Perft-Values-Unverified]]): Perft values not independently verified. No direct impact on eval.
- Attack query API is the board boundary (ADR-001). Eval must use `is_square_attacked_by(sq, attacker, &board)` for attack queries. Never read `board.squares[]` directly.
- `is_square_attacked_by` returns bool (no allocation). `attackers_of` returns `Vec` (allocates). Prefer `is_square_attacked_by` in eval hot path.
- Pawn directions: Red +rank, Blue +file, Yellow -rank, Green -file. Relevant for PST advancement bonuses and king safety pawn shield.

**From Stage 3 downstream log:**
- `GameState` provides `board()`, `score(player)`, `scores()`, `player_status(player)`, `is_player_active(player)`.
- `PlayerStatus`: Active, DeadKingWalking, Eliminated. Eval must handle all three states.
- DKW pieces have status Dead — should contribute 0cp to material eval.
- FFA scoring constants in `gamestate::scoring` are CAPTURE/RULES values (Pawn=1pt, Queen=9pt), NOT eval values. Stage 6 defines separate eval constants (Pawn=100cp, Queen=900cp).

### Risks for This Stage

1. **PST rotation correctness (Section 2.17).** 14x14 board with 4 rotational perspectives. Rotation bug silently produces wrong positional bonuses for 1-3 players. Mitigated by extensive unit tests with known square mappings.

2. **i16 overflow in eval summation (Section 2.6).** Multiple eval components summed into i16 could overflow. Mitigated by saturating arithmetic and clamping to [-30000, 30000].

3. **PromotedQueen dual-value confusion (Section 2.9).** FFA capture value (1pt) vs eval value (900cp) must be clearly separated. Mitigated by distinct constants and naming (`CAPTURE_PROMOTED_QUEEN` vs `PROMOTED_QUEEN_EVAL_VALUE`).

4. **Dead/Terrain piece handling (Section 2.6).** Piece list contains all pieces regardless of status. Material/positional scoring must check `board.piece_at(sq).status` to exclude dead/terrain pieces.

5. **API boundary compliance (Section 2.21).** Eval must not read `board.squares[]` directly. Must use `piece_at()`, `piece_list()`, `king_square()`, and attack query API.


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

- Stage spec: [[stage_06_bootstrap_eval]]
- Downstream log: [[downstream_log_stage_06]]
