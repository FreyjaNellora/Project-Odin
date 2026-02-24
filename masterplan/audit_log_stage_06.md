# Audit Log — Stage 06: BootstrapEval

## Pre-Audit
**Date:** 2026-02-21
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build`)
- Tests pass: Yes (229 total: 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04)
- Previous downstream flags reviewed: Stages 0-3 (full dependency chain: Stage 6 → 3 → 2 → 1 → 0)

### Findings

**From Stage 0 downstream log:**
- [Historical] Huginn macro and buffer available. `Phase::Eval` variant exists. No gates wired yet (deferred per [Resolved - Huginn retired in Stage 8, replaced by `tracing` crate]). Stage 6 will define `eval_call` and `eval_comparison` gate schemas but not wire them (following established pattern).

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
**Date:** 2026-02-21
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| Evaluator trait (eval_scalar + eval_4vec) | Done | `pub trait Evaluator` in `eval/mod.rs`. Takes `&GameState` + `Player`, returns i16 or [f64; 4]. Permanent contract. |
| BootstrapEvaluator struct | Done | Zero-size, stateless. Implements `Evaluator` + `Default`. |
| Material counting (alive pieces only) | Done | `eval/material.rs`. Iterates piece list, checks `piece_at(sq).status`. Dead/Terrain = 0cp. |
| Eval piece value constants | Done | `eval/values.rs`. Separate from `gamestate::scoring` capture values. PromotedQueen = 900cp (search), 1pt (FFA). |
| Piece-square tables with 4-player rotation | Done | `eval/pst.rs`. Const rotation tables (784 bytes). 7 PST grids from Red's perspective. |
| King safety heuristic | Done | `eval/king_safety.rs`. Pawn shield (max +45cp) + attacker pressure. Uses `is_square_attacked_by` (allocation-free). |
| Multi-player relative eval | Done | `eval/multi_player.rs`. Lead penalty (cap -150cp), threat penalty (30cp/opponent), FFA points integration (50cp/pt). |
| Sigmoid normalization for eval_4vec | Done | `normalize_4vec()` with K=400. Independent sigmoids per player (not softmax). |
| Eliminated player floor | Done | Returns -30000 immediately for eliminated players. |
| Saturating arithmetic throughout | Done | All component additions use `saturating_add`/`saturating_sub`, final result clamped to [-30000, 30000]. |
| Integration tests (11 tests) | Done | All 5 acceptance criteria + 6 additional (symmetry, bounds, random games, perft regression, sanity). |
| pub mod eval in lib.rs | Done | Evaluator trait exposed publicly for Stage 7+. |

### Code Quality
#### Uniformity

Consistent patterns throughout. All eval components follow the same structure: `pub(crate) fn component_name(board, player, ...) -> i16`. Constants use SCREAMING_SNAKE. Each submodule has its own `#[cfg(test)] mod tests`. Function signatures align with upstream API contracts from Stages 1-3.

#### Bloat

No unnecessary abstractions. BootstrapEvaluator is zero-size (no runtime state). PST rotation tables are computed at compile time. No allocations in the eval hot path. The 7 PST grids (each 196 entries) are intentionally simple arrays — no fancy data structures needed for bootstrap eval.

#### Efficiency

- Material scoring: 16 iterations per player (piece list), no allocation.
- PST: 16 table lookups per player via pre-computed rotation.
- King safety: max 27 `is_square_attacked_by` calls (9 squares x 3 opponents). No `attackers_of` (which allocates Vec).
- Threat penalty: 3 `is_square_attacked_by` calls (1 per opponent).
- Lead penalty: simple arithmetic over 4-element arrays.
- Performance: <10us per eval in release, <50us in debug. Well under target.

#### Dead Code

None identified. `cargo clippy` clean. All `pub(crate)` functions used by `eval_for_player`. All public exports (`PIECE_EVAL_VALUES`, trait, struct) needed by integration tests and Stage 7.

#### Broken Code

No known issues. 100 random games with eval at every position — no panics, all values in bounds. PST rotation verified with known square mappings for all 4 players. Material counting verified against expected starting value (4300cp).

#### Temporary Code

The entire BootstrapEvaluator is temporary — designed to be replaced by NnueEvaluator in Stage 16. The Evaluator trait and `eval/values.rs` constants persist. PST values are deliberately rough (bootstrap quality). King safety and multi-player components may evolve in Stage 8 (BRS Hybrid).

### Search/Eval Integrity

- **Dual value separation verified:** `eval/values.rs` defines PAWN_EVAL_VALUE=100cp while `gamestate/scoring.rs` defines CAPTURE_PAWN=1pt. No cross-contamination. Test `test_eval_values_sanity` verifies PROMOTED_QUEEN_EVAL_VALUE == QUEEN_EVAL_VALUE.
- **Dead piece handling verified:** `material_score` and `positional_score` both check `piece.status == PieceStatus::Alive` before counting. Test `test_material_after_piece_removal` confirms.
- **Perspective correctness verified:** Test `test_evaluation_is_perspective_dependent` confirms asymmetric positions produce higher scores for the materially advantaged player.
- **Symmetry verified:** Starting position: Red==Yellow, Blue==Green, all within 100cp. Test `test_starting_position_approximate_symmetry`.
- **Trait boundary established:** All eval access goes through `dyn Evaluator`. Test `test_evaluator_trait_compiles` verifies trait object dispatch works.

### Future Conflict Analysis

1. **Stage 7 (BRS Search):** Search will call `evaluator.eval_scalar(position, player)` at leaf nodes. The trait interface is ready. No structural changes needed in eval.
2. **Stage 8 (BRS Hybrid):** May tune king safety and multi-player parameters. The component structure (separate files) supports this cleanly.
3. **Stage 9 (TT & Move Ordering):** Move ordering may use `PIECE_EVAL_VALUES` for MVV-LVA. These are already public.
4. **Stage 16 (NNUE Integration):** `NnueEvaluator` implements `Evaluator`. Bootstrap eval becomes dead code but should be preserved as fallback/comparison until verified.
5. **eval_4vec usage:** MCTS (Stage 10) will use `eval_4vec` for 4-player value estimates. The sigmoid normalization approach (independent, not softmax) may need revision — softmax would enforce sum-to-1 constraint. This is a design decision for Stage 10.

### Unaccounted Concerns

1. **Bishop=500cp == Rook=500cp:** On a 14x14 board, bishops have more diagonal scope. The equal valuation is defensible but non-standard. May need tuning after self-play (Stage 12).
2. **Lead penalty heuristic untested in real games:** The lead penalty (cap -150cp) is a reasonable guess but has no empirical basis yet. Self-play will validate or invalidate.
3. **material_scores called twice per eval_for_player:** Once for `material_score(board, player)` and once for the `lead_penalty` call. Could cache. Not a concern at current performance levels (<10us) but worth noting for future optimization.

### Reasoning & Methods

1. **Separate values.rs from gamestate::scoring:** Eval centipawns and FFA capture points serve different purposes. Mixing them is the #1 bug risk identified in pre-audit. Separate files with clear naming (`*_EVAL_VALUE` vs `CAPTURE_*`) prevent confusion.
2. **Compile-time rotation tables:** 784 bytes of const data eliminates runtime rotation cost. The const fn approach avoids `lazy_static` or `once_cell` dependencies.
3. **PSTs from Red's perspective only:** Defining 4 separate PST sets would be error-prone and wasteful. Rotation at lookup time is clean and verifiable.
4. **Sigmoid over softmax for eval_4vec:** Softmax would enforce sum-to-1, which is semantically correct for "win probability" but harder to compute and unnecessary for the bootstrap eval. Sigmoid keeps each player's estimate independent. MCTS may prefer softmax — deferred to Stage 10.
5. **Saturating arithmetic over checked arithmetic:** Checked would panic on overflow. Saturating silently clamps. For eval, saturation at extremes is the desired behavior (a position +40000cp isn't meaningfully different from +30000cp).


---

## Related

- Stage spec: [[stage_06_bootstrap_eval]]
- Downstream log: [[downstream_log_stage_06]]
