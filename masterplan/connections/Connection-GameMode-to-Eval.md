---
type: connection
stage_introduced: 8
tags:
  - stage/08
  - area/eval
  - area/gamestate
status: active
last_updated: 2026-02-23
---

# Connection: GameMode → EvalProfile → Eval

## Overview

Game rules (GameMode) and evaluation personality (EvalProfile) are independent axes that affect how the engine evaluates positions. This connection describes how they interact.

## Data Flow

```
GameMode (FFA/LKS)
    ↓
EngineOptions.resolved_eval_profile()
    ↓
EvalProfile (Standard/Aggressive)
    ↓
EvalWeights { ffa_point_weight, lead_penalty_enabled, ... }
    ↓
BootstrapEvaluator::new(profile) → stores weights
    ↓
eval_for_player() uses weights for lead_penalty + ffa_points_eval
```

## Resolution Rules

| GameMode | Explicit Profile | Resolved Profile |
|----------|-----------------|------------------|
| FFA      | None (Auto)     | Aggressive       |
| LKS      | None (Auto)     | Standard         |
| FFA      | Standard        | Standard         |
| LKS      | Aggressive      | Aggressive       |

Explicit profile always overrides the auto-resolution. Cross-mode testing (e.g., LKS + Aggressive) is intentional.

## Key Differences

| Parameter | Standard | Aggressive |
|-----------|----------|------------|
| `ffa_point_weight` | 50cp per point | 120cp per point |
| `lead_penalty_enabled` | true | false |
| `lead_penalty_divisor` | 4 | 4 (unused) |
| `max_lead_penalty` | 150cp | 0cp |

## Impact on Behavior

- **Standard (LKS default):** Penalizes material leads, making the engine more conservative. Good for survival-focused play where accumulating too much attention is dangerous.
- **Aggressive (FFA default):** No lead penalty, high FFA point weight. Engine freely captures material and pursues scoring opportunities. Required for correct tactical behavior in FFA.
- **W4 (lead penalty tactical mismatch):** Standard profile sometimes prefers check over free capture because the capture would increase Red's material lead (penalty). This is eval behavior, not a search bug.

## Touchpoints

- `protocol/types.rs` — `EngineOptions::resolved_eval_profile()`
- `protocol/mod.rs` — `handle_setoption` parses `gamemode` and `evalprofile`
- `eval/mod.rs` — `BootstrapEvaluator::new(profile)`, `eval_for_player(position, player, weights)`
- `eval/multi_player.rs` — `lead_penalty()` and `ffa_points_eval()` accept weight params
- `gamestate/mod.rs` — `GameMode` enum, `GameState::game_mode()` accessor

## Related

- [[Component-Eval]] — eval implementation
- [[Component-GameState]] — GameMode source
- [[Component-Protocol]] — setoption parsing
- [[Connection-GameState-to-Eval]] — broader GameState → Eval connection
