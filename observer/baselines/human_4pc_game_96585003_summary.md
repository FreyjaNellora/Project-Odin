# Baseline: Human 4PC FFA — Game 96585003 (Mixed Elo — Weak Lobby)

**Source:** chess.com 4-Player Chess (FFA)
**Players:** Chess_Taifun (2254) | Hamlet (3068) | Bidakterakhir (2072) | visor_sarge (2594)
**Result:** Red 9, Blue 64, Yellow 18, Green 47 | Termination: Blue +20
**Total Moves:** 173 (52 rounds)
**Date:** 2026-02-09

## Why This Game Matters

This is the **counter-example** baseline. Hamlet (3068) dominates weaker players (2072-2594). The weak players exhibit the SAME behavioral bugs we see in the Odin engine. If the engine plays like Red or Yellow here, it's playing at ~2100-2250 Elo.

## Opening Patterns (First 20 Rounds)

| Player | Elo | Pawn Ratio | Queen Active | Castled | Captures | Grade |
|--------|-----|-----------|-------------|---------|----------|-------|
| Red    | 2254 | **50%** | Round 4 | Round 8 | 1 | WEAK |
| Blue   | 3068 | **30%** | Round 2 | Round 9 | 2 | STRONG |
| Yellow | 2072 | **50%** | Round 12 | Round 13 | 1 | WEAK |
| Green  | 2594 | **40%** | Round 3 | Round 11 | 3 | MID |

## Weak Player Patterns (What the Engine Should NOT Do)

### Red (2254 Elo) — "Scattered Pawn Pusher"
- **50% pawn ratio** — half of all moves are pawn pushes
- Pushes h3, k4, f3, j3, g4, d4, h4, h5 — pawns everywhere, no structure
- Knight takes 3 moves to reach a useful square (Ne1→g2→e3→f5)
- Castles early but then pushes ALL kingside pawns forward, destroying own king shelter
- Only 1 capture in 20 moves — plays passively while being attacked
- **Result: 9 points — crushed**

### Yellow (2072 Elo) — "Slow Developer"
- **50% pawn ratio** — matches the engine's v0.4.3 behavior exactly
- Queen doesn't activate until round 12 (strong players: round 2-5)
- **Knight undevelopment** — Nh13→j14 at round 14 (same bug as Odin engine!)
- Bishop shuffles waste 3 tempo: Bi14→h13→i12→h11
- Pushes e-pawn 3 times (e12→e11→e10) with no purpose
- **Result: 18 points — second worst**

### Green (2594 Elo) — "Getting There"
- **40% pawn ratio** — better than weak but worse than strong
- Queen active by round 3 — good
- Captures material (3 captures) — engages tactically
- Still too many pawn pushes (8/20 moves)
- **Result: 47 points — decent but not competitive**

## Strong vs Weak Comparison Table

| Metric | Strong (3000+) | Mid (~2500) | Weak (~2100) | Engine v0.4.3 |
|--------|---------------|-------------|-------------|---------------|
| Pawn ratio | 30% | 40% | 50% | **53%** |
| Queen activation | Round 2-5 | Round 3-7 | Round 8-12 | **Never (10 rounds)** |
| Knight undevelopment | 0 | 0 | 1 | **1** |
| Captures in first 20 | 2-3 | 3 | 1 | **0** |
| Bishop shuffling | No | Rare | Yes | **Yes** |

## The Verdict

**The Odin engine (v0.4.3) plays worse than a 2072-rated human.** It has higher pawn ratio, later queen activation, and zero captures. The engine's behavioral bugs are literally the hallmarks of the weakest human players in the dataset.

The current engine (post-fixes: dev bonuses, pawn gate, king displacement) should be retested to see where it now falls on this scale.
