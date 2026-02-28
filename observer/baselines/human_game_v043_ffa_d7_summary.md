# Baseline: Human-Observed Engine Game (v0.4.3-narrowing)

**Source:** User-observed gameplay, manually logged with full search traces
**Engine:** v0.4.3-narrowing (pre-multi-perspective, pre-pawn-structure, pre-king-displacement)
**Settings:** FFA | Standard | Depth 7 | 42 ply (not played to completion)

## Per-Player Summary

| Player | Moves | Avg Eval | Avg Nodes | Eval Range | Pawn Moves | Piece Moves |
|--------|-------|----------|-----------|------------|------------|-------------|
| Red    | 11    | 4876     | 26,820    | 4456–5212  | 4 (36%)    | 7 (64%)     |
| Blue   | 11    | 4386     | 33,149    | 4198–4612  | 7 (64%)    | 4 (36%)     |
| Yellow | 10    | 4099     | 35,254    | 3811–4443  | 6 (60%)    | 4 (40%)     |
| Green  | 10    | 3961     | 29,374    | 3801–4506  | 5 (50%)    | 5 (50%)     |

## Known Behavioral Bugs

### Bug 1: Green Rook Exposure (MAJOR)
Green pushes pawns aggressively (m5k5, m10l10, m6k6, l10k10, m8k8) while Red's bishop snipes from e2→n11→h5. Green's rook becomes exposed and captured.

### Bug 2: Undefended Pawn Pushes (MAJOR)
Blue makes 7/11 moves as pawn pushes, many undefended (d6e6, e9f9). Engine doesn't penalize undefended forward pawns.

### Bug 3: Knight Undevelopment (MAJOR)
Red moves Ni3→j1 at ply 40 (eval 5142cp — engine confident). Knight retreats to starting square. BRS paranoid modeling likely causes engine to "see" opponents capturing the knight, making retreat look safe.

## Key Metrics for Comparison

These are the numbers future observer runs should compare against:

- **Pawn-to-piece move ratio:** Blue 64% pawn, Yellow 60% pawn, Green 50% pawn. Healthy would be ~30-40% pawn in opening.
- **Eval spread at ply 42:** Red 5146, Green 3897 = 1249cp gap. Large asymmetry.
- **Node growth:** Moves 1-4 avg ~8K nodes, moves 8-11 avg ~50K nodes. Healthy exponential growth.
- **Knight undevelopment:** Should be 0 instances. This baseline has 1 (Red i3→j1).
- **Eval monotonicity:** Red's eval climbs from 4456→5212 (+756cp in 11 moves) — suspiciously high for no captures. Suggests eval inflation.

## What "Better" Looks Like

After fixes (dev bonuses, pawn gate, king displacement, multi-perspective):
- Pawn move ratio should drop to ~30-40% in opening
- No knight undevelopment
- More balanced eval spread across players
- Green should develop pieces before pushing 5 pawns
- Blue should interleave piece development with pawn pushes
