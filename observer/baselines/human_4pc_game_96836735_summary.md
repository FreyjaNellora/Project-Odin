# Baseline: Human 4PC FFA — Game 96836735 (Weak Lobby, Player-C Dominates)

**Source:** chess.com 4-Player Chess (FFA)
**Players:** Player-H (2266) | Player-K (2104) | Player-L (2252) | Player-C (3077)
**Result:** Red 15, Blue 34, Yellow 36, Green 77 | Termination: insuff. material
**Total Moves:** 270 (85 rounds) | Red checkmated round 17
**Date:** 2026-02-12

## Why This Game Matters

Player-C as Green shows **maximum exploitation of weak players**: 20% pawn ratio, 5 captures in 17 moves, checkmate delivered by round 17. The weak players (all ~2100-2250) demonstrate how NOT to play. Red gets checkmated after wandering the king across the board.

## Opening Patterns (First 17-20 Rounds)

| Player | Elo | Pawn Ratio | Queen Active | Castled | Captures | Fate |
|--------|-----|-----------|-------------|---------|----------|------|
| Red    | 2266 | 29%* | Round 9 | **Never** | 3 | **Checkmated R17** |
| Blue   | 2104 | 35% | Round 5 | **Never** | 4 | Survived (34 pts) |
| Yellow | 2252 | 40% | Round 5 | Round 20 | 3 | Survived (36 pts) |
| Green  | 3077 | **20%** | Round 2 | Round 18 | **5** | **Won (77 pts)** |

*Red's pawn ratio looks decent but 4/5 pawns were pushed in the first 4 rounds before any piece development.

## Critical Weak Patterns

### Red (2266) — "King Walk to Checkmate"
The most instructive failure in our dataset:
1. Pushes h3, k4, f3, g3 — four pawns in first four moves, zero pieces
2. Develops knights but allows both to be captured
3. King forced to walk: h1→g1→g2→f1 — never castles
4. **Checkmated round 17** with 15 points

This is almost exactly what the Odin engine does: push pawns, delay development, leave king exposed. If the engine does this, it's playing like a 2266 who gets destroyed.

### Yellow (2252) — "Queen Shuffle"
1. Four pawn pushes in first four rounds
2. Queen shuffles: Qf12→f6→f12→f10 — four moves, net gain zero
3. Second knight not developed until round 14
4. Castles round 20 — dangerously late

The queen shuffle is a new weak pattern to watch for: the engine might move the queen actively but without purpose, wasting tempo.

### Green/Player-C (3077) — "Clinical Exploitation"
1. **20% pawn ratio** — lowest in any baseline game (only 4 pawn moves in 20)
2. Both knights developed by round 4
3. Bishop pair aggressively captures material: Bm5→j2→Rk1 (wins rook), Bl7→h3+ (wins piece)
4. Delivers checkmate round 17 — fastest elimination in dataset
5. Castles after the damage is done (round 18)

## New Patterns Identified

### "Front-loaded pawn spam"
Red and Yellow both push 4 pawns in their first 4 moves. Even though Red's overall pawn ratio (29%) looks acceptable, the SEQUENCE matters — all pawns first, then pieces. Strong players interleave pawn and piece moves from move 1.

### "Queen shuffle"
Yellow's queen moves Qf12→f6→f12→f10 — four tempo spent on one piece with no material gain, no threats created, no territory gained. The engine should be checked for this pattern.

### "King march = death"
Red's king walked h1→g1→g2→f1 across 4 moves. In 4PC FFA, an exposed king is a target for ALL three opponents. This is the most dangerous weak pattern.

## Updated Elo Tier Table

| Metric | Strong (3000+) | Mid (~2500) | Weak (~2200) | Engine v0.4.3 |
|--------|---------------|-------------|-------------|---------------|
| Pawn ratio | **20-30%** | 35-40% | 29-40%* | **53%** |
| Queen activation | **Round 2-5** | Round 3-7 | Round 5-9 | **Never** |
| Captures in first 20 | **4-5** | 2-3 | 1-4 | **0** |
| King safety | Castle + shelter | Castle late | **King march** | **King walk** |
| Piece interleaving | From move 1 | From move 3 | **Pawns first** | **Pawns first** |

*Weak players' pawn ratios can look deceptively decent because they front-load all pawns then switch to pieces out of desperation.
