# Downstream Log — Stage 13: Time Management

## Notes for Future Stages

### Must-Know

- **W14:** `TimeManager::allocate()` uses `score_cp < 2000` for near-elimination detection. If NNUE eval (Stages 14-16) uses a different score scale, this threshold must be recalibrated.
- **W15:** `PositionType::Endgame` triggers at `piece_count() <= 16` (from 64 starting). This threshold may need tuning after NNUE makes positional evaluation more nuanced.
- **W16:** `limits_to_budget()` now takes `current_player: Option<Player>`. If called from contexts without a known player, pass `None` for the fallback `.or()` chain behavior.

### API Contracts

- `TimeContext` is set via `HybridController::set_time_context()` before `search()`. Consumed via `.take()` — one-shot per search. If not set, `search()` uses the raw budget unchanged.
- `SearchLimits::time_for_player(player)` returns `(Option<u64>, Option<u64>)` = (remaining_ms, increment_ms).
- `HybridController::apply_options(&EngineOptions)` threads tunable params. Call before `search()`.
- `setoption` names (lowercase): `tactical_margin`, `brs_fraction_tactical`, `brs_fraction_quiet`, `mcts_default_sims`, `brs_max_depth`.

### Known Limitations

- **W13 (carried):** MCTS score 9999 (max) in some positions — unchanged.
- **Pondering not implemented:** Stage 13 prompt listed it as optional. Deferred.
- **tune.mjs uses same binary for both engines:** It differentiates via `setoption` at runtime. Cannot test parameters that require recompilation.

### Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| TimeManager::allocate() | <1us per call | Pure arithmetic, no allocation |
| Enriched classify_position() | +1 is_in_check call | ~2us overhead per search |
| Time alloc for 60s clock | ~960ms (quiet), ~1560ms (tactical) | At ply 0, 50 moves estimated |
| Forced move return | <1ms | Bypasses search entirely |

### Open Questions

- **Near-elimination threshold (2000cp):** Is this the right cutoff for "lost half material"? Starting material is ~4300cp per player, so 2000cp = 47% remaining. May need empirical tuning.
- **Endgame piece count threshold (16):** Is 16 pieces (4 kings + 12 others) the right boundary? Could be too aggressive in 4-player chess where 4 players means more material overall.

### Reasoning

- Two-layer time allocation separates protocol concerns (clock extraction) from search concerns (position-aware allocation). This is cleaner than pushing GameState into the protocol layer.
- Override fields on HybridController are `Option<T>` so they cost nothing when unused (common case). Only tuning runs set them.



---

## Related

- Stage spec: [[stage_13_time_management]]
- Audit log: [[audit_log_stage_13]]
