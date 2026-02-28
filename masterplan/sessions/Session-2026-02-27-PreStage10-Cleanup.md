---
type: session
tags:
  - type/session
  - stage/pre-10
date: 2026-02-27
---

# Session: Pre-Stage-10 Final Cleanup

**Date:** 2026-02-27
**Focus:** Audit fixes, pawn-push/king-walk eval mitigations, Vec clone cost retrofit

## Summary

Executed the 3-task pre-Stage-10 cleanup plan (Claude.D + Claude Code collaboration):

1. **Quick audit fixes** — W1 (lead_penalty hardcoded weight), W2 (prev_player duplication), N1 (clippy).
2. **Pawn-push preference mitigations** — Increased development bonuses, gated connected pawn bonus to 2+ rank advances, added king displacement penalty.
3. **Vec clone cost retrofit** — `position_history` to `Arc<Vec<u64>>` (O(1) clone), `piece_lists` to fixed-size arrays (zero heap alloc on clone).

## What Changed

### Task 1: Audit Fixes

| Fix | File | Change |
|-----|------|--------|
| W1 | `eval/multi_player.rs` | `lead_penalty()` now accepts `ffa_point_weight: i16` parameter instead of using hardcoded default |
| W2 | `board/types.rs` | Added `Player::prev()` method |
| W2 | `movegen/moves.rs`, `gamestate/mod.rs` | Removed duplicate `fn prev_player()`, replaced 5 calls with `.prev()` |
| N1 | `eval/values.rs` | `const { assert!(...) }` style for const assertions |
| N1 | `eval/mod.rs` | `(0.0..=1.0).contains(&v)` for range check |

### Task 2: Pawn-Push Mitigations

| Mitigation | File | Change |
|------------|------|--------|
| Dev bonuses | `eval/development.rs` | Knight 25→45, Bishop 15→30, Queen 35→50, Rook 15→25 |
| Pawn gate | `eval/pawn_structure.rs` | `is_sufficiently_advanced()` — only 2+ ranks past start get connected bonus |
| King penalty | `eval/king_safety.rs` | -40cp if king not on home rank/file (`is_on_home_rank()`) |

### Task 3: Vec Clone Cost Retrofit

| Refinement | File | Change |
|------------|------|--------|
| Ref 2 | `gamestate/mod.rs` | `position_history: Vec<u64>` → `Arc<Vec<u64>>`, `Arc::make_mut` on push |
| Ref 1 | `board/board_struct.rs` | `piece_lists: [Vec; 4]` → `[[(PieceType, Square); 20]; 4]` + `piece_counts: [u8; 4]` |

## Test Results

- 408 engine tests (267 unit + 141 integration, 3 ignored) — all pass
- 0 clippy warnings
- Perft invariants: 20/395/7800/152050 — unchanged

## Issues

- `Issue-Vec-Clone-Cost-Pre-MCTS`: **RESOLVED** — both refinements applied
- `Issue-Pawn-Push-Preference-King-Walk`: **MITIGATED** — eval-side fixes applied, full fix requires MCTS

## What's Next

User gameplay testing gate, then Stage 10 (MCTS).
