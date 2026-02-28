---
type: issue
date_opened: 2026-02-26
last_updated: 2026-02-26
date_resolved:
stage: 8
severity: warning
status: open
tags: [area/search, area/board-scanner, severity/warning]
---

# Issue: BRS Hybrid Scoring Too Paranoid for FFA

## Description

The hybrid reply scoring formula in `board_scanner.rs` (lines 608-656) blends paranoid and realistic opponent modeling:

```
hybrid_score = harm_to_root * likelihood + objective_strength * (1 - likelihood)
```

The current likelihood constants produce an approximately **80% paranoid / 20% realistic blend**:

| Constant | Value | Effect |
|---|---|---|
| `LIKELIHOOD_BASE_TARGETS_ROOT` | 0.7 | Every opponent with ANY root-targeting move gets 70% likelihood |
| `LIKELIHOOD_BEST_TARGET_BONUS` | 0.2 | If root is the best target, likelihood reaches 90% |
| `LIKELIHOOD_SUPPORTING_BONUS` | 0.1 | Compound attacks get 80% |
| `LIKELIHOOD_EXPOSED_PENALTY` | 0.3 | Opponents with exposed positions only drop to ~40% |
| `LIKELIHOOD_BASE_NON_ROOT` | 0.2 | Opponents not targeting root still get 20% paranoid weight |

**Symptom:** In user testing (v0.4.3-narrowing), Blue moved b6d6 exposing its king. Red's queen had a clear attack path (f2f4 + Qd1-h5 or similar), but the engine didn't pursue it. The paranoid model assumed Blue would "defend" by attacking Red (70% likelihood) rather than modeling Blue's actual situation (exposed king = Blue is weak = Blue's best move is likely defensive, not aggressive against Red).

**Root cause:** The `LIKELIHOOD_BASE_TARGETS_ROOT = 0.7` is too high for FFA. In FFA with 3 opponents, the rational expectation is that each opponent attacks whoever is weakest or most profitable — not necessarily the root player. A 70% base makes the engine play as if all opponents are coordinating against it.

## Affected Components

- [[Component-BoardScanner]] — `score_reply()`, `compute_harm_to_root()`, likelihood constants
- [[Component-Search]] — BRS search quality depends on accurate opponent modeling
- [[stage_08_brs_hybrid]] — hybrid scoring is the Stage 8 innovation

## Proposed Fix

**Priority 1 — Likelihood Tuning (do now, before Stage 10):**

1. Lower `LIKELIHOOD_BASE_TARGETS_ROOT` from **0.7 to 0.5**
   - 50% base means "even odds" the opponent targets root vs anyone else
   - With 3 opponents, 50% is already generous (random = 33%)
2. Increase `LIKELIHOOD_EXPOSED_PENALTY` from **0.3 to 0.5**
   - An opponent with an exposed king should drop to near-zero root targeting (they'll play defensively)
3. Keep `LIKELIHOOD_BEST_TARGET_BONUS` at 0.2 (so max = 0.7 when root truly IS the best target)
4. Increase `LIKELIHOOD_BASE_NON_ROOT` from **0.2 to 0.3**
   - Give more weight to objective-best moves for opponents not targeting root

**Expected effect:** Opponents with exposed kings (Blue in the test game) would be modeled more realistically — they'd play defensive/selfish moves rather than "attacking Red." Red would then see that Blue's weakness persists and can be exploited.

**Priority 2 — Target Attractiveness Model (Stage 12+ with self-play data):**

Replace the binary "targets root / doesn't target root" with a continuous model that considers ALL opponents' relative weakness:
- Score each potential target (including root) by: material deficit, king exposure, piece coordination
- Weight likelihood toward the weakest target, not just toward root
- This requires self-play calibration (Stage 12)

## Workaround

Currently: engine plays conservatively, missing exploitation opportunities against weakened opponents. No crash or correctness issue — just suboptimal play quality.

## Resolution

<!-- When fix is implemented, describe what was done and set status to "pending-verification". -->

## Related

- [[Session-2026-02-26-BRS-Architecture-Investigation]] — analysis session that identified this issue
- [[Component-BoardScanner]] — implementation location
- [[stage_08_brs_hybrid]] — stage spec
- [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]] — related eval issue (lead penalty also affects opponent modeling)
