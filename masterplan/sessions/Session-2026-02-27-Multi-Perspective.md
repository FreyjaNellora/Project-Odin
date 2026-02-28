---
type: session
tags:
  - type/session
  - stage/post-9
date: 2026-02-27
---

# Session: Multi-Perspective Opponent Modeling

**Date:** 2026-02-27
**Scope:** 3-term blend scoring (paranoid + BRS + anti-leader)
**Version:** `v0.5.0-multi-perspective`

## What Was Done

Implemented the full 10-step plan from `engine_multi_perspective_instructions.md`.

### Core Changes

Replaced the 2-term likelihood formula in `board_scanner.rs`:

```
OLD: score = harm_to_root * likelihood + objective_strength * (1 - likelihood)
NEW: score = w_paranoid * harm_to_root + w_brs * objective_strength + w_anti_leader * harm_to_leader
```

### Implementation Steps

1. **`find_leader()`** — finds strongest active player by material + FFA score (50cp per point)
2. **`compute_harm_to_player()`** — generalized from `compute_harm_to_root()` to accept any target player
3. **`BoardContext` extended** — `leader_player` and `material` fields added
4. **`BlendWeights` + `compute_blend_weights()`** — dynamic per-opponent weight computation
5. **Deleted `LIKELIHOOD_*` constants** — 5 constants removed
6. **`ScoredReply` updated** — `likelihood` → `harm_to_leader`
7. **`score_reply()` rewritten** — 3-term formula with dynamic blend
8. **`select_hybrid_reply()` verified** — no changes needed
9. **7 new unit tests** added
10. **Version string** → `v0.5.0-multi-perspective`

### Weight Behavior

| Scenario | w_paranoid | w_brs | w_anti_leader |
|---|---|---|---|
| Opponent targets root, root not leader | ~0.47 | ~0.33 | ~0.20 |
| Opponent targets root, root IS leader | ~0.60 | ~0.40 | 0.00 |
| Opponent doesn't target root, big leader gap | ~0.20 | ~0.33 | ~0.47 |
| Opponent doesn't target root, no leader gap | ~0.38 | ~0.62 | 0.00 |
| Exposed opponent (high vulnerability) | lower | higher | lower |

## What Was NOT Changed

- `select_hybrid_reply()` logic (narrowing, classification, fallback) — unchanged
- `scan_board()` — only `leader_player` and `material` fields added
- `brs.rs` — no changes; search calls are unchanged
- Vulture (`harm_to_weakest`) and convergent (`harm_to_opp_target`) terms — deferred

## Test Results

396 engine tests (253 unit + 143 integration, 3 ignored). All passing. No existing test thresholds needed adjustment.

## Files Modified

- `odin-engine/src/search/board_scanner.rs` — all scoring changes + 7 new tests
- `odin-engine/src/protocol/emitter.rs` — version string

## What's Next

1. Resolve [[Issue-Vec-Clone-Cost-Pre-MCTS]]
2. User manual gameplay testing (GATE)
3. Stage 10 (MCTS)
4. Future: multi-perspective phase 2 (vulture + convergent terms)
