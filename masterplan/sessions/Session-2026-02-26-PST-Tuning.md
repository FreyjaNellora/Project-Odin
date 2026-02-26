---
type: session
date: 2026-02-26
stage: post-stage-9 (gameplay quality)
tags: [stage/09, area/eval, area/pst]
---

# Session: 2026-02-26 -- PST Tuning (Knight Gradient + Bishop Development)

## Goal

Fix "knight chess" — all four players opening with 3-4 knight moves each, bishops and rooks
rarely developing. Observed live in the running app after Stage 9 was complete.

## What Happened

### Root Cause Analysis

User identified that the per-move PST gain math was driving the behavior. The engine at depth 7
gives Red ~2 moves. The comparison that mattered:

| Strategy (2 Red moves) | Old gain |
|---|---|
| Two knight developments | +23 + +23 = **+46cp** ← engine always picked this |
| g-pawn + bishop fianchetto | +15 + +15 = **+30cp** |

Knight first hop (Ne1→f3) was gaining **+23cp in a single move** due to the steep gradient:
- rank 0 center: -20cp (huge back-rank penalty)
- rank 2 center: +10cp
- Net: **+30cp first hop** (old grid had even larger values; session predecessor was -15 to +8 = +23cp)

Bishop development was structurally cheaper per move: it required a pawn push prep (only +15cp)
before the bishop could move at all. Even with improved bishop destinations, the two-move
sequence averaged only ~+22cp/move vs knight's +23cp/move.

### First Attempt (Previous Context)

Increased bishop destination values (rank 2-4 center: +25 to +32cp). Created only a 2cp
advantage for the bishop plan (+48cp) vs knight plan (+46cp). Too small — swamped by search noise.
Engine still played 3/4 knight moves.

Strengthened bishop back-rank penalty (-8→-15cp) and rank1 reward (+8→+20cp). Bishop single step
became +35cp, creating a 4cp gap. Still insufficient — knight doesn't need a prep move.

### Root Fix (This Context)

User correctly diagnosed: **the knight gradient itself** is the domino. Even with better bishop
values, the knight wins because each individual knight move beats anything else when viewed
in isolation. The bishop plan requires TWO moves to show its value.

Fix: **flatten the knight gradient** so first hop = ~+10cp (competitive, not dominant).

**KNIGHT_GRID changes:**
- rank 0 center: -20 → **-3cp** (small penalty, not spring-loaded)
- rank 2 center: +10/+12 → **+8cp**
- rank 3/4 (peak): +20-30 → **+12cp**
- First hop gain: +23cp → **+10cp**

**BISHOP_GRID rank 1 adjustment:**
- Center: +20 → **+15cp** (pulled back slightly; the flatter knight makes bishop plans naturally competitive)

**New 2-move comparison:**

| Strategy (2 Red moves) | New gain |
|---|---|
| Two knight developments | +10 + +10 = **+20cp** |
| g-pawn + bishop fianchetto | +15 + +30 = **+45cp** ← wins clearly |
| e-pawn + bishop | +8 + +27 = **+35cp** |
| g-pawn + knight | +15 + +10 = **+25cp** |

Bishop paths now clearly dominate. Knights are still useful (good supporting role), but
no single knight move individually crushes all alternatives.

### Clippy Cleanup

12 pre-existing clippy warnings addressed (all in Stage 9 code, none from PST work):
- `board_scanner.rs`: `get(0)` → `first()`, needless range loop, collapsible if,
  match-for-equality, manual range contains, map_or simplification
- `brs.rs`: manual is_multiple_of (x2), map-over-inspect
- `tt.rs`: `is_empty()` added alongside `len()` (len_without_is_empty lint)
- `protocol/emitter.rs`, `protocol/mod.rs`, test files: formatting (cargo clippy --fix)

## Components Touched

- [[component-pst]] (`odin-engine/src/eval/pst.rs`) — KNIGHT_GRID, BISHOP_GRID, ROOK_GRID,
  QUEEN_GRID redesigned this session and in the previous context (KING_GRID was fixed in
  [[Session-2026-02-26-KingSafety-SEE-Hotfixes]])
- [[component-board-scanner]] (`odin-engine/src/search/board_scanner.rs`) — clippy fix
- [[component-tt]] (`odin-engine/src/search/tt.rs`) — `is_empty()` added

## Discoveries

**The gradient spring-load problem.** A large back-rank penalty + large destination value creates
a "spring-loaded" single-move gain that dominates search at shallow depth. In 4-player chess where
each player gets ~2 moves in a 7-ply search, the engine cannot see past the immediate gain to
prefer a 2-move setup plan.

**Rule of thumb:** First development hop gain must be ≤ pawn push gain (~+10-15cp), otherwise
the piece with the steeper gradient will be moved first every single game.

**Bishop math structure:** Bishop development is inherently a 2-move investment (pawn push +
bishop step). The bishop's total destination value only shows its advantage when the prep move is
included. If the knight gradient makes ANY single knight step worth more than the bishop's average
over its 2-move setup, the engine will always play knight-first.

## Issues Created/Resolved

No new issues created. No existing issues resolved.

PST tuning is ongoing — these are heuristic values subject to self-play calibration (Stage 12+).
The values chosen are principled first approximations: knight development ~+10cp/hop,
bishop single step ~+30cp (after pawn prep), pawn pushes +8-15cp by rank.
