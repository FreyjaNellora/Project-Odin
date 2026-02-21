# Downstream Log — Stage 06: BootstrapEval

## Notes for Future Stages

### Must-Know

1. **Evaluator trait is the eval boundary (permanent invariant).** All search code calls `evaluator.eval_scalar(position, player)` or `evaluator.eval_4vec(position)` through the `Evaluator` trait. Never call `eval_for_player` directly (it's `pub(crate)`). Never reach into eval submodules.
2. **eval_scalar returns i16 centipawns.** Range: -30000 to +30000. Eliminated players always return -30000. Positive = good for the given player.
3. **eval_4vec returns [f64; 4] via independent sigmoid.** Each value in [0, 1]. Index by `Player::index()`. NOT softmax -- values do NOT sum to 1. MCTS may need softmax normalization on top.
4. **Eval takes &GameState, not &Board.** It needs FFA scores (`position.score(player)`) and player statuses (`position.player_status(player)`) beyond just the board position.
5. **BootstrapEvaluator is stateless (zero-size).** `BootstrapEvaluator::new()` or `::default()`. No incremental update. No accumulated state. NNUE will have state (accumulator) managed by search.
6. **Dual piece values are strictly separated.** Eval uses `eval::PIECE_EVAL_VALUES` (Pawn=100cp, PromotedQueen=900cp). Scoring uses `gamestate::scoring::capture_points` (Pawn=1pt, PromotedQueen=1pt). Never use one where the other belongs.

### API Contracts

**Evaluator Trait (`eval::Evaluator`):**

```rust
pub trait Evaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16;
    fn eval_4vec(&self, position: &GameState) -> [f64; 4];
}
```

- `eval_scalar`: centipawn score from one player's perspective
- `eval_4vec`: all 4 players normalized to [0,1], indexed by `Player::index()`

**BootstrapEvaluator (`eval::BootstrapEvaluator`):**

| Method | Signature | Notes |
|---|---|---|
| `new()` | `-> Self` | Zero-size struct, no arguments |
| `default()` | `-> Self` | Same as `new()` |

**Public Constants (`eval::values` re-exported via `eval::*`):**

| Constant | Value | Type |
|---|---|---|
| `PAWN_EVAL_VALUE` | 100 | i16 |
| `KNIGHT_EVAL_VALUE` | 300 | i16 |
| `BISHOP_EVAL_VALUE` | 500 | i16 |
| `ROOK_EVAL_VALUE` | 500 | i16 |
| `QUEEN_EVAL_VALUE` | 900 | i16 |
| `KING_EVAL_VALUE` | 0 | i16 |
| `PROMOTED_QUEEN_EVAL_VALUE` | 900 | i16 |
| `PIECE_EVAL_VALUES` | [i16; 7] | Indexed by `PieceType::index()` |

### Known Limitations

1. **Bootstrap eval is rough.** PST values are simple advancement/center bonuses. King safety uses basic pawn shield + attack counting. This is intentionally "good enough for BRS to find captures and avoid blunders" -- not a competitive eval.
2. **Bishop == Rook (500cp each).** Non-standard but defensible on 14x14 board. May need tuning.
3. **eval_4vec uses sigmoid, not softmax.** Values are independent -- they don't sum to 1. MCTS (Stage 10) may need to add softmax normalization.
4. **material_scores computed twice per eval.** Once in `material_score(board, player)`, once passed to `lead_penalty`. Not a performance concern at current levels.
5. **No mobility term.** Bootstrap eval has no piece mobility component. Could improve playing strength if added later.
6. **Lead penalty is a heuristic.** Cap of -150cp. No empirical basis yet. Self-play (Stage 12) will validate.

### Performance Baselines

| Metric | Value | Notes |
|---|---|---|
| eval_scalar per call | <10us | Release build, starting position |
| eval_scalar per call (debug) | <50us | Debug build, starting position |
| 10,000 evals | <100ms | Release build |
| Engine test count | 275 | 191 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 + 11 stage-06 |
| Starting position material per player | 4300cp | 8P + 2N + 2B + 2R + Q + K = 800+600+1000+1000+900+0 |

### Open Questions

1. **Softmax vs sigmoid for eval_4vec:** Should Stage 10 (MCTS) expect softmax-normalized values, or should it normalize itself? Currently sigmoid -- each player independent.
2. **PST tuning:** Should PST values be tuned by self-play in Stage 12, or replaced wholesale by NNUE in Stage 16?
3. **King safety evolution:** Stage 8 (BRS Hybrid) may want more sophisticated king safety (open file penalties, king pawn storm detection). Current component structure supports incremental improvement.

### Reasoning

1. **&GameState over &Board:** Eval needs FFA scores and player statuses. Passing &Board would require separate arguments for these.
2. **i16 for centipawns:** Sufficient range (-32768 to 32767). Smaller than i32 -- fits in search node structures. Standard in chess engines.
3. **Separate eval/values.rs from gamestate/scoring.rs:** Prevents the most likely bug in 4PC eval -- confusing capture points with centipawn values.
4. **Const rotation tables:** Zero runtime cost for PST rotation. 784 bytes is negligible.
5. **Saturating arithmetic:** Prevents panic on overflow. At score extremes, saturation is the desired behavior.


---

## Related

- Stage spec: [[stage_06_bootstrap_eval]]
- Audit log: [[audit_log_stage_06]]
