---
type: issue
date_opened: 2026-02-26
last_updated: 2026-02-26
date_resolved: 2026-02-26
stage: post-9
severity: note
status: resolved
tags: [area/eval, severity/note]
---

# Issue: Eval-Side Hanging Piece Penalty Double-Counts Search Threats

## Description

An eval-side `hanging_piece_penalty` was implemented in `multi_player.rs` that penalized undefended pieces under attack at half their piece value (capped at 500cp). The penalty was integrated into `eval_for_player` as a subtraction.

**Result:** The engine regressed — Red's Nf3 retreated to e1, and the king walked Kh1→g2. Red's eval dropped from ~4400 to ~3889 while opponents stayed ~4400.

**Root cause:** The search tree already handles capture threats via alpha-beta pruning, quiescence search, and SEE-based move ordering. The eval penalty double-counted these threats:
1. Search sees the capture threat and explores the line → already accounted for
2. Eval ALSO penalizes the static position → makes forward deployment look worse than it is
3. Combined effect: the engine sees development as too risky and retreats

## Affected Components

- [[Component-Eval]] — `eval/mod.rs`, `eval/multi_player.rs`
- [[Component-Search]] — quiescence search, SEE, move ordering already handle captures

## Workaround

N/A — issue was immediately reverted.

## Resolution

**Fully reverted** in the same session. The hanging piece penalty function was removed from `multi_player.rs` and the integration removed from `eval_for_player`. A comment was added to `eval/mod.rs` explaining why:

```rust
// NOTE: hanging_piece_penalty was removed here — it double-counted capture
// threats already handled by the search tree, causing the engine to retreat
// developed pieces (Nf3→e1 regression in v0.4.3). The narrowing fix
// (root-capture protection) addresses hanging pieces through search instead.
```

The correct approach — addressing hanging pieces through search-side narrowing protection (root-capture exemption in progressive narrowing) — was kept and verified working in v0.4.3-narrowing.

**Lesson learned:** Tactical threats (captures, checks, hanging pieces) belong in search (move ordering, narrowing protection, quiescence), NOT in static eval. Eval should reflect positional and strategic factors that search cannot discover within its depth horizon.

## Related

- [[Session-2026-02-26-BRS-Architecture-Investigation]] — session where this was implemented and reverted
- [[Component-BoardScanner]] — root-capture protection (the correct fix)
- [[Component-Eval]] — where the incorrect penalty was attempted
