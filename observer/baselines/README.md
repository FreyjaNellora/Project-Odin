# Observer Baselines

Reference data for measuring engine improvement over time. Three tiers of comparison: strong human play (the target), weak human play (the floor), and engine self-play (where we actually are).

## Strong Human Games (Target Behavior)

Real 4PC FFA games from chess.com with 3000+ Elo players. The engine should aspire to match these patterns.

| File | Players | Elo Range | Result | Rounds | Key Feature |
|------|---------|-----------|--------|--------|-------------|
| `human_4pc_game_95992584` | SeadraCheeseChess, neoserbian, Hamlet, sakthi09 | 3054-3434 | Red 73, Yellow 73 | 100 | Full game, tight high-Elo spread |
| `human_4pc_game_96085550` | Hamlet, Froggychess2, MillzGambit, Yunikra | 2541-3438 | Blue 87 (checkmate) | 84 | Lower-rated player wins, O-O-O popular |

## Weak/Mixed Human Games (Counter-Examples)

Games with weaker players (2000-2600). These players make the SAME mistakes the engine makes. If the engine plays like them, it's playing at that Elo.

| File | Players | Elo Range | Result | Rounds | Key Feature |
|------|---------|-----------|--------|--------|-------------|
| `human_4pc_game_96585003` | Chess_Taifun, Hamlet, Bidakterakhir, visor_sarge | 2072-3068 | Blue 64 (Hamlet dominates) | 52 | Weak players show engine-like bugs |
| `human_4pc_game_96836735` | Chess_Taifun, lipeih1, martinaxo, Hamlet | 2104-3077 | Green 77 (Hamlet dominates) | 85 | Red checkmated R17, queen shuffle, king march |
| `human_4pc_game_96602063` | Hamlet, Chess_Taifun, AaryaBhatt0123, basenowsky | 1954-2709 | Red 63 (Hamlet dominates) | 50 | Sub-2000 floor: 54% pawn, queen never activated, 3 pts |

## Elo Tier Benchmarks (Opening, First 20 Rounds)

| Metric | Strong (3000+) | Mid (~2500) | Weak (~2100) | Engine v0.4.3 |
|--------|---------------|-------------|-------------|---------------|
| Pawn move ratio | **20-30%** | 35-40% | 29-50%* | **53%** |
| Queen activation | **Round 2-5** | Round 3-7 | Round 5-12 | **Never** |
| Knight undevelopment | **0** | 0 | 0-1 | **1** |
| Captures in first 20 | **4-5** | 2-3 | 1-4 | **0** |
| Queen shuffling | **No** | No | Yes | **Unknown** |
| King march | **Never** | Never | **Yes** | **Yes** |
| Castling | **By round 18** | By round 13 | Round 13-20+ | **Never** |
| Piece interleaving | **From move 1** | From move 3 | **Pawns first** | **Pawns first** |

*Weak players' pawn ratios can look deceptively decent — they often front-load 4+ pawns before developing any pieces, making the overall ratio misleading. Sequence matters.

**The engine v0.4.3 played worse than 2072 Elo humans on most metrics.**

## Engine Baselines

| File | Version | Settings | Depth | Ply | Known Bugs |
|------|---------|----------|-------|-----|------------|
| `human_game_v043_ffa_d7` | v0.4.3-narrowing | FFA/standard | 7 | 42 | Green rook exposure, Blue pawn spam, Red knight undevelop |

## How to Use

1. Run the observer: `cd observer && node observer.mjs`
2. Compare output in `observer/reports/` against these baselines
3. Key metrics to track across versions:
   - **Pawn ratio** (target <=35%, currently 53%)
   - **Queen activation** (target <=10 rounds)
   - **Piece development count** by round 10 (target >=3)
   - **Knight undevelopment instances** (target: 0)
   - **Captures in first 20** (target >=1)
   - **Eval spread** across players (smaller = more balanced)
   - **Node growth curve** (should be exponential, not flat)

## Rating Scale

Use this to estimate where the engine falls:

| Elo Range | Pawn Ratio | Queen | Captures/20 | Typical Result |
|-----------|-----------|-------|-------------|---------------|
| <2000 | **54%** | **Never** | 1 | 3 pts |
| 2000-2150 | 40-50% | Round 8-12 | 1-3 | 18-30 pts |
| 2200-2300 | 35-45% | Round 5-9 | 1-4 | 15-36 pts |
| 2500-2700 | 30-40% | Round 3-7 | 2-3 | 47-65 pts |
| 3000+ | **20-30%** | **Round 2-5** | **4-5** | **63-87 pts** |

## Dataset Summary

**6 human games total** from chess.com 4PC FFA:
- 2 strong lobbies (3000+ Elo, target behavior)
- 3 weak lobbies (Hamlet vs 1954-2266 opponents, counter-examples)
- 1 engine self-play (v0.4.3-narrowing, known bugs)
- **24 player-games** across the dataset (each game has 4 players)
- Elo range covered: **1954 to 3438**
