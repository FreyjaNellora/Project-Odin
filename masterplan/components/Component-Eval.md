---
type: component
stage_introduced: 6
tags:
  - stage/06
  - area/eval
status: active
last_updated: 2026-02-21
---

# Component: Eval (Bootstrap Evaluator)

The evaluation system behind the `Evaluator` trait. Bootstrap handcrafted eval scores positions using material, piece-square tables, king safety, and multi-player adjustments. Replaced by NNUE in Stage 16, but the trait persists.

## Purpose

Provides position evaluation for search. Without eval, search cannot compare positions or choose moves. The `Evaluator` trait is the permanent boundary -- all search code calls through it, never a specific implementation. The bootstrap eval is "good enough" for BRS to find captures and avoid blunders.

## Files

- `eval/mod.rs` -- Evaluator trait, BootstrapEvaluator struct, `eval_for_player`, sigmoid normalization
- `eval/values.rs` -- Centipawn piece value constants (separate from FFA capture scoring)
- `eval/material.rs` -- Material counting (sum alive pieces per player)
- `eval/pst.rs` -- Piece-square tables with compile-time 4-player rotation
- `eval/king_safety.rs` -- Pawn shield bonus + opponent attacker pressure
- `eval/multi_player.rs` -- Lead penalty, threat penalty, FFA points integration

## Key Types

- **Evaluator** (trait) -- Permanent eval boundary. `eval_scalar(&GameState, Player) -> i16` and `eval_4vec(&GameState) -> [f64; 4]`. All search goes through this.
- **BootstrapEvaluator** -- Zero-size, stateless implementation. Temporary (Stage 16 replaces with NnueEvaluator).
- **PIECE_EVAL_VALUES** -- `[i16; 7]` indexed by `PieceType::index()`. Centipawn values for search, NOT FFA capture points.

## Public API

| Item | Signature | Notes |
|---|---|---|
| `Evaluator` trait | `eval_scalar`, `eval_4vec` | Permanent contract |
| `BootstrapEvaluator::new()` | `-> Self` | Stateless, zero-size |
| `BootstrapEvaluator::default()` | `-> Self` | Same as `new()` |
| `PAWN_EVAL_VALUE` | `i16 = 100` | Re-exported from values |
| `KNIGHT_EVAL_VALUE` | `i16 = 300` | Re-exported |
| `BISHOP_EVAL_VALUE` | `i16 = 500` | Re-exported |
| `ROOK_EVAL_VALUE` | `i16 = 500` | Re-exported |
| `QUEEN_EVAL_VALUE` | `i16 = 900` | Re-exported |
| `KING_EVAL_VALUE` | `i16 = 0` | Re-exported |
| `PROMOTED_QUEEN_EVAL_VALUE` | `i16 = 900` | Re-exported |
| `PIECE_EVAL_VALUES` | `[i16; 7]` | Re-exported, indexed by PieceType |

## Internal Flow: eval_for_player

1. **Check elimination:** If `player_status == Eliminated`, return -30000 immediately.
2. **Material:** Sum `PIECE_EVAL_VALUES[pt.index()]` for all alive pieces. (eval/material.rs)
3. **Positional:** Sum PST values for all alive pieces using rotated lookup. (eval/pst.rs)
4. **King safety:** Pawn shield bonus + attacker pressure penalty. (eval/king_safety.rs)
5. **Threat penalty:** Count opponents attacking king square, 30cp each. (eval/multi_player.rs)
6. **Lead penalty:** If leading in combined strength, penalty up to -150cp. (eval/multi_player.rs)
7. **FFA points:** Game score * 50cp. (eval/multi_player.rs)
8. **Combine:** `material + positional + king_safety - threat + lead + ffa`, all saturating, clamped to [-30000, 30000].

## Connections

- Depends on: [[Component-Board]] (piece_list, piece_at, king_square), [[Component-MoveGen]] (is_square_attacked_by), [[Component-GameState]] (player_status, score, scores)
- Depended on by: [[stage_07_plain_brs]], [[stage_08_brs_hybrid]], [[stage_10_mcts]], [[stage_16_nnue_integration]]
- Communicates via: [[Connection-GameState-to-Eval]], [[Connection-Eval-to-Search]]

## Huginn Gates

Specified in [[MASTERPLAN]] Stage 6:
- `eval_call` (Verbose) -- position hash, player, component scores (material, positional, king_safety, threat, lead, ffa), final score
- `eval_comparison` (Normal) -- two positions compared, scores, which is better

Not wired (see [[Issue-Huginn-Gates-Unwired]]).

## Gotchas

1. **PIECE_EVAL_VALUES vs capture_points:** These are different value systems. Eval uses centipawns (Pawn=100, PromotedQueen=900). FFA scoring uses capture points (Pawn=1, PromotedQueen=1). Using the wrong one silently produces wrong behavior.
2. **Piece list includes dead/terrain pieces.** Always check `board.piece_at(sq).status == Alive` before counting material or PST.
3. **PST rotation is not obvious.** Red=identity, Blue=(rank,file), Yellow=(13-file,13-rank), Green=(13-rank,13-file). The ROTATION const tables handle this, but if adding new PST grids, test rotation correctness for all 4 players.
4. **eval_4vec uses sigmoid, NOT softmax.** Values don't sum to 1. Each player's estimate is independent.
5. **Eliminated players get -30000, not 0.** This is intentional -- search must strongly prefer keeping a player alive.

## Performance Notes

- <10us per eval (release), <50us (debug)
- No allocations in hot path
- 784 bytes of const rotation tables
- Starting material: 4300cp per player

## Known Issues

None.

## Build History

- [[Session-2026-02-21-Stage06]] -- initial implementation

## Related

- [[stage_06_bootstrap_eval]] -- spec
- [[audit_log_stage_06]] -- audit findings
- [[downstream_log_stage_06]] -- API contracts
