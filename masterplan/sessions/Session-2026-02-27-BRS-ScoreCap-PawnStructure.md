---
type: session
tags:
  - type/session
  - stage/post-9
date: 2026-02-27
---

# Session: BRS Score Cap + Pawn Structure + Depth 8

**Date:** 2026-02-27
**Scope:** False mate display fix, connected pawn bonus, depth 8 default, development bonus
**Version:** `v0.5.0-multi-perspective` (unchanged)

## What Was Done

### 1. Depth Default 7 -> 8 (Prior Sub-Session)

Changed `odin-engine/src/protocol/mod.rs` default `SearchBudget.max_depth` from 7 to 8. In 4PC, depth 7 = 1.75 rotations (R->B->Y->G->R->B->Y), depth 8 = 2 full rotations. Incomplete rotations produce false mate scenarios where the last opponent has no reply.

### 2. Development Bonus Module (Prior Sub-Session)

**File:** `odin-engine/src/eval/development.rs` (NEW)

Per piece off back rank: Queen +35cp, Knight +25cp, Rook +15cp, Bishop +15cp. Back rank detection per player orientation (Red=rank 0, Blue=file 0, Yellow=rank 13, Green=file 13).

### 3. False Mate Early-Termination Fix

**File:** `odin-engine/src/search/brs.rs` line ~361

BRS iterative deepening had an early-break on mate detection:
```rust
// OLD: breaks immediately, even at depth 7 (false mate)
if score.abs() >= MATE_SCORE - MAX_DEPTH as i16 { break; }

// NEW: requires 2 full rotations before trusting mate
if score.abs() >= MATE_SCORE - MAX_DEPTH as i16 && depth >= 8 { break; }
```

This fixed premature termination at depth 7, but false mates still appear at depth 8 because BRS's single-reply model inherently produces phantom mates (one opponent move chosen, opponent can't explore alternatives).

### 4. BRS Score Cap (Display Only)

**File:** `odin-engine/src/search/brs.rs`

Added `BRS_SCORE_CAP = 9,999` constant. Capped scores in two places:
- Info line emission: `display_score = score.clamp(-BRS_SCORE_CAP, BRS_SCORE_CAP)`
- `SearchResult.score`: `self.best_score.clamp(-BRS_SCORE_CAP, BRS_SCORE_CAP)`

Internal alpha-beta uses unclamped scores for correctness (aspiration windows, mate-break detection, TT storage). The cap only affects what the UI sees. Confirmed working: Green move 3 `Bn9i4` shows 9999cp instead of 19995cp.

### 5. Connected Pawn Bonus

**File:** `odin-engine/src/eval/pawn_structure.rs` (NEW)

+8cp per pawn defended by a friendly pawn (diagonally behind in advance direction). Handles all 4 orientations:
- Red (fwd +rank): defenders at (f-1, r-1), (f+1, r-1)
- Blue (fwd +file): defenders at (f-1, r-1), (f-1, r+1)
- Yellow (fwd -rank): defenders at (f-1, r+1), (f+1, r+1)
- Green (fwd -file): defenders at (f+1, r-1), (f+1, r+1)

Wired into `eval_for_player()` in `eval/mod.rs`. 8 unit tests (defender squares per player, edge cases, symmetry, starting position = 0).

**Concern noted:** Connected pawn bonus may reinforce pawn-push tendency. After f2f3, the pawn is instantly "connected" (+8cp) because e2/g2 neighbors defend it. This is a small effect vs +25cp knight dev, but pulls in the same direction.

## Remaining Issue: Pawn Push Preference + King Walk

[[Issue-Pawn-Push-Preference-King-Walk]]

Red consistently plays 4-5 pawn pushes before developing any pieces, and walks its king (Ki2 on move 4). Full game sample:
```
1. k2k4 (fine)   2. f2f3 (blocks knight)   3. i2i3 (passive)
4. Ki2 (king walk!!)   5. d2d3 (slow)   6. j2j4 (more pawns)
```

Root causes under investigation (to be explored by next session):
1. BRS single-reply model may penalize knight development — opponent's "best reply" is to capture the exposed knight, making the move look bad
2. King safety may not adequately penalize the king leaving its starting square
3. Connected pawn bonus may marginally reinforce pawn pushes

## Test Results

405 engine tests (264 unit + 141 integration, 3 ignored). All passing. 0 clippy warnings.

## Files Modified

### Engine
- `odin-engine/src/search/brs.rs` — depth >= 8 mate-break gate, `BRS_SCORE_CAP` constant, display score clamping, SearchResult score clamping
- `odin-engine/src/eval/pawn_structure.rs` — NEW: connected pawn bonus (+8cp)
- `odin-engine/src/eval/mod.rs` — added `pawn_structure` module, wired into eval formula
- `odin-engine/src/eval/development.rs` — NEW: piece development bonus (prior sub-session)
- `odin-engine/src/protocol/mod.rs` — depth 7 -> 8 default (prior sub-session)

### Documentation
- `masterplan/sessions/Session-2026-02-27-BRS-ScoreCap-PawnStructure.md` — this file
- `masterplan/issues/Issue-Pawn-Push-Preference-King-Walk.md` — NEW
- `masterplan/HANDOFF.md` — updated
- `masterplan/STATUS.md` — updated

## What's Next

1. **Investigate pawn-push preference + king walk** — root cause analysis (king safety, BRS opponent reply bias, eval balance)
2. Resolve [[Issue-Vec-Clone-Cost-Pre-MCTS]]
3. User manual gameplay testing (GATE for Stage 10)
4. Stage 10 (MCTS)
