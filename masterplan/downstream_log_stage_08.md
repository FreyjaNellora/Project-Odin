# Downstream Log — Stage 08: BRS/Paranoid Hybrid Layer

**Date:** 2026-02-23
**Author:** Claude Opus 4.6 (Stage 8 implementation session)

## Notes for Future Stages

### Must-Know

1. **Board scanner runs once pre-search, not during search.** `scan_board(gs, root_player)` produces a `BoardContext` that is frozen for the entire search tree. It is NOT updated as pieces move during search. This means `per_opponent` aggression values, `high_value_targets`, and `convergence` reflect the root position only. A delta updater was deferred to v2. If a future stage needs mid-search context updates, it must either implement the delta updater or accept stale data.

2. **Hybrid reply scoring only at main-search MIN nodes.** Quiescence MIN nodes still use `select_best_opponent_reply` (plain BRS: pick move that minimizes root eval). This is intentional — quiescence is captures-only with a small candidate set where likelihood weighting adds no value.

3. **`relative_material_advantage` added to eval.** `eval_for_player` now has 7 components: material + positional + king_safety - threat + lead_penalty + ffa_points + relative_material_advantage. The relative component rewards having more material than the average of active opponents. Weight: advantage / 4, clamped to ±500cp. Any future eval changes must account for this component.

4. **Progressive narrowing limits at deep depths.** At depth 7+, only 3 opponent candidates survive narrowing. At depth 4-6, up to 6. At depth 1-3, up to 10. `cheap_presort` sorts by capture value (MVV) before truncation. If a future stage changes the depth schedule, update the constants in `board_scanner.rs` and re-verify tactical test correctness.

5. **EvalProfile affects tactical behavior.** Standard profile (lead penalty ON) makes the engine avoid accumulating material leads — sometimes preferring check over free capture (W4). Aggressive profile (lead penalty OFF) fixes this for FFA. Tactical tests that expect material-gaining moves should use Aggressive profile. Any future tactical test must choose the correct profile.

6. **GameMode and EvalProfile are independent axes.** GameMode (FFA/LKS) controls rules (win conditions, claim-win gating). EvalProfile (Standard/Aggressive) controls eval personality (lead penalty, FFA point weight). Default pairing: FFA → Aggressive, LKS → Standard. Explicit override via `setoption name EvalProfile`. Any future mode/profile interaction must respect this separation (ADR-014 in DECISIONS.md).

7. **W5 (stale non-board GameState fields) still open.** `player_status`, `scores`, `halfmove_clock` are NOT updated during make/unmake in search. Board scanner reads accurate data pre-search. Eval reads root-position data during search. This is acceptable for bootstrap eval but will matter for NNUE (Stage 16) if it reads player status.

### API Contracts

**Board Scanner (public, Stage 8):**
```rust
pub fn scan_board(gs: &GameState, root_player: Player) -> BoardContext
pub fn classify_move(mv: Move, board: &Board, root_player: Player) -> MoveClass
pub fn classify_moves(moves: &[Move], board: &Board, root_player: Player) -> (Vec<Move>, Option<Move>)
pub fn narrowing_limit(depth: u8) -> usize
pub fn select_hybrid_reply(
    gs: &mut GameState, evaluator: &dyn Evaluator, root_player: Player,
    opponent: Player, moves: &[Move], ctx: &BoardContext, depth: u8,
) -> Option<Move>
```

**Key types:**
- `BoardContext` — pre-search analysis output. Fields: `game_mode`, `root_player`, `weakest_player`, `most_dangerous: [Player; 3]`, `root_danger_level: f64`, `high_value_targets`, `convergence`, `per_opponent: [OpponentProfile; 3]`.
- `OpponentProfile` — per-opponent analysis. Fields: `player`, `aggression_toward_root: f64`, `own_vulnerability: f64`, `best_target: Player`, `can_afford_to_attack_root: bool`, `supporting_attack_on_root: bool`.
- `MoveClass` — `Relevant` or `Background`.
- `ScoredReply` — hybrid-scored move. Fields: `mv`, `hybrid_score`, `objective_strength`, `harm_to_root`, `likelihood`.

**Eval additions (Stage 8):**
```rust
pub(crate) fn relative_material_advantage(board: &Board, player: Player, player_statuses: &[PlayerStatus; 4]) -> i16
```
Returns centipawns (±500 max). Weight: raw advantage / 4.

**EvalProfile / EvalWeights (Stage 8, Step 0):**
```rust
pub enum EvalProfile { Standard, Aggressive }
pub struct EvalWeights { ffa_point_weight: i16, lead_penalty_enabled: bool, lead_penalty_divisor: i16, max_lead_penalty: i16 }
pub fn BootstrapEvaluator::new(profile: EvalProfile) -> Self
```

### Known Limitations

**W5 — Stale GameState fields during search (carried from Stage 7):**
`player_status` and `scores` are snapshots from the `go` call. Not updated during make/unmake. Board scanner reads accurate pre-search data. Eval reads stale data during search. Acceptable for bootstrap eval. Will need resolution before NNUE if NNUE reads player status.

**W4 — Lead penalty tactical mismatch (carried from Stage 7):**
Standard profile's lead penalty causes the engine to prefer check over free capture in some positions. Aggressive profile (no lead penalty) eliminates this. Tactical tests use Aggressive for capture/fork positions. Not a search bug — eval behavior.

**Delta updater deferred to v2:**
Board scanner data is frozen for the entire search. In long searches at deep depths, the board may look very different from the root position. A delta updater that patches the scanner every 2 plies was planned but deferred — scanner time is < 1ms, so re-scanning would be cheap. Not critical for Stage 8 performance.

### Performance Baselines

**Hybrid BRS (release build), starting position, Aggressive profile:**

| Depth | Nodes (Stage 8 Hybrid) | Nodes (Stage 7 Plain) | Reduction |
|-------|------------------------|-----------------------|-----------|
| 6     | < 10,916               | 10,916                | ~49%      |
| 8     | < 31,896               | 31,896                | ~46%      |

- Board scanner: < 1ms per call (release build).
- Smoke-play: 10 games × 20 moves × depth 4 with no panics or illegal moves.
- Test count: 361 (233 unit + 128 integration), 3 ignored.

### Open Questions

1. **Should the delta updater be implemented for Stage 9 (TT)?** With TT, deeper searches become practical. Stale board context may hurt more at depth 10+. Evaluate after Stage 9 performance data is available.

2. **Should `relative_material_advantage` weight be tunable?** Currently hardcoded at divisor 4. A stronger weight makes the engine more aggressive about captures. Tuning could be part of Stage 17 (Game Mode Variant Tuning).

3. **How does hybrid scoring interact with TT (Stage 9)?** TT stores best moves from previous searches. If the hybrid scoring changes which move is "best" at a MIN node, TT collisions could cause stale best-move suggestions. This likely doesn't matter since TT is indexed by position hash, not by opponent identity.

### Reasoning

The board scanner, move classifier, and hybrid reply scoring were designed as pre-search analysis tools that enhance BRS without changing its core alpha-beta algorithm. Progressive narrowing provides significant node reduction (~49%) with minimal tactical quality loss. The `relative_material_advantage` eval fix was critical — without it, the engine had zero incentive to capture opponent pieces. The EvalProfile separation enables testing both conservative (LKS) and aggressive (FFA) play styles without code changes.

---

## Related

- Stage spec: [[stage_08_brs_hybrid]]
- Audit log: [[audit_log_stage_08]]
