---
type: issue
date_opened: 2026-02-21
last_updated: 2026-02-21
date_resolved:
stage: 7
severity: info
status: open
tags:
  - stage/07
  - stage/08
  - area/eval
  - area/search
  - severity/info
---

# Bootstrap Eval Lead-Penalty Causes Tactical Mismatch

## Description

The bootstrap evaluator's lead-penalty heuristic penalizes Red's material advantage. When BRS finds a sequence that increases Red's material lead, the evaluator scores that sequence lower than a sequence that gives check (which doesn't immediately increase the lead). This causes the engine to prefer check-giving moves over immediate captures in some tactical positions.

**Observed example (Stage 7):** Position with a free Blue queen at g8. Red has a queen at h7.
- `h7g8` (queen capture) — increases Red's material. Penalized by lead heuristic. Lower score.
- `h7b7+` (check) — does not immediately increase material. Higher score (905cp).

BRS selects `h7b7+`. This is correct BRS+eval behavior — the check sequence has higher eval output — but is incorrect tactically (capturing a free queen is strictly better).

## Root Cause

`eval/multi_player.rs` lead penalty: if Red's material + PST significantly exceeds the average, apply a penalty up to -150cp. This creates a non-monotonic material function where capturing a major piece can make the eval score *decrease* if it pushes Red's lead above a threshold.

## Impact

- Stage 7 tactical test assertions were relaxed from specific-move assertions to `legal + positive score`.
- 5 mate positions in `tactical_suite.txt` marked `[unverified]` — engine may prefer checks over mates due to this heuristic.
- Engine is tactically suboptimal for the leading player: it may prefer checks over captures, sacrifices over consolidation.
- Engine is *strategically* correct for FFA: the lead penalty is intended to make the engine avoid building an obvious target, which is valid 4-player strategy.

## Not Affected

- BRS search logic (correct)
- Move legality (all moves legal)
- Score sign (always positive for winning positions)
- Non-leading positions (lead penalty only fires when material lead exceeds threshold)

## Resolution Plan

Stage 8 (Bootstrap Eval refinement):
1. Verify whether lead-penalty weight is tunable to allow tactical captures while retaining FFA-strategic discouragement of runaway leads.
2. Re-verify 5 `[unverified]` mate positions in tactical suite against the updated eval.
3. If tactical suite positions pass, remove `[unverified]` tags.
4. Consider alternative formulation: penalize lead only at material advantage > N pieces, not at marginal leads caused by a single capture.

## Related

- [[audit_log_stage_07]] -- tactical limitation section
- [[downstream_log_stage_07]] -- `Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch` entry
- [[Component-Eval]] -- lead penalty in eval/multi_player.rs
- [[Component-Search]] -- BRS uses eval_scalar at leaf nodes
- `tests/positions/tactical_suite.txt` -- positions with [unverified] tags
