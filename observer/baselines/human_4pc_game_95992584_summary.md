# Baseline: Human 4PC FFA — Game 95992584

**Source:** chess.com 4-Player Chess (FFA)
**Players:** SeadraCheeseChess (3293) | neoserbian (3054) | Hamlet (3434) | sakthi09 (3289)
**Result:** Red 73, Blue 20, Yellow 73, Green 47 | Termination: Yellow +20
**Total Moves:** 319 (100 rounds, game went to completion)
**Date:** 2026-01-30

## Opening Patterns (First 20 Rounds)

| Player | Pawn Moves | Piece Moves | Pawn Ratio | Queen Active | Castled |
|--------|-----------|-------------|------------|-------------|---------|
| Red    | 6         | 14          | 30%        | Round 7     | Round 17 |
| Blue   | 7         | 13          | 35%        | Round 7     | No       |
| Yellow | 6         | 14          | 30%        | Round 2     | No       |
| Green  | 6         | 14          | 30%        | Round 4     | Round 18 |

## Key Human Behaviors to Compare Against

1. **Pawn ratio ~30-35%** in opening (not 50-65% like the engine)
2. **Queen activated early** (rounds 2-7) — humans use the queen aggressively in 4PC
3. **Both knights developed** within first 10-12 rounds for all players
4. **Castling** when available (2/4 castled, by round 18)
5. **No knight undevelopment** — zero instances of retreating a developed knight to its starting square
6. **Captures happen** — Red captured on round 6 (Ne5xRc4), round 20 (Qf6xNc6). Humans take material when available.
7. **Rook activity** — Yellow had rook active by round 8 (Rd14-d12, Rd12-e12). Engine tends to leave rooks dormant.
8. **Bishop fianchetto** — Yellow fianchettoed both bishops (Bi14-h13, Bf14-h12). Green developed both bishops (Bn6-k9, Bn9-m10).

## Benchmark Targets for Engine

| Metric | Human Average | Engine Baseline (v0.4.3) | Target |
|--------|--------------|-------------------------|--------|
| Pawn ratio (first 20) | 31% | 53% | <=35% |
| Queen activation round | 5 | Never (in 10 rounds) | <=10 |
| Pieces developed by R10 | 4+ | 2 | >=3 |
| Knight undevelopment | 0 | 1 | 0 |
| Captures in first 20 | 1-2 per player | 0 | >=1 |
