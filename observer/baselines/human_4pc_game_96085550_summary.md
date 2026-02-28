# Baseline: Human 4PC FFA — Game 96085550

**Source:** chess.com 4-Player Chess (FFA)
**Players:** Hamlet (3438) | Froggychess2 (2732) | MillzGambit (3083) | Yunikra (2541)
**Result:** Red 32, Blue 87, Yellow 33, Green 65 | Termination: Checkmate
**Total Moves:** 286 (84 rounds) | Checkmate finish
**Date:** 2026-01-31

## Notable: Lower-rated player wins

Froggychess2 (2732 Elo, lowest in lobby) wins with 87 points and checkmate. Demonstrates that in 4PC FFA, positioning and timing matter more than raw Elo. The lower-rated player can exploit multi-player dynamics.

## Opening Patterns (First 20 Rounds)

| Player | Pawn Moves | Piece Moves | Pawn Ratio | Queen Active | Castled | Captures |
|--------|-----------|-------------|------------|-------------|---------|----------|
| Red    | 6         | 14          | 30%        | Round 3     | Round 13 (O-O-O) | 4 |
| Blue   | 5         | 15          | 25%        | Round 2     | Round 17 (O-O-O) | 2 |
| Yellow | 6         | 14          | 30%        | Round 3     | Round 8 (O-O-O) | 3 |
| Green  | 10        | 10          | 50%        | Round 7     | No       | 3 |

## Key Human Behaviors

1. **O-O-O popular** — 3 of 4 players castled queenside. Engine should support and evaluate O-O-O.
2. **Queen active by round 3** for top players. Even Green (2541 Elo) activates queen by round 7.
3. **Aggressive captures** — avg 3 captures per player in first 20 moves. Material exchange is normal.
4. **Bishop pair activation** — All players developed both bishops within 15 rounds.
5. **Rook activation** — Red had rook on j9 by round 18, Yellow had rook e12 by round 9. Active rooks early.
6. **Pawn structure** — Even the weakest player (Green, 50% pawn ratio) still developed both knights and queen.
7. **No knight undevelopment** — zero instances across all 4 players.

## Interesting Tactical Sequences

- Round 14: Blue captures Red's queen (Qd7xQg4), Yellow captures Blue's knight (Qg10xNe8) — multi-player tactics where one capture enables another.
- Round 17-18: Chain of captures — Red Bh2xBm7+, Yellow Qh8xBc8, Green Qm7xRg1+ — 3-way exchange sequence.
- Round 23: Blue checkmates Green (Qg9-b9#) — early checkmate eliminates a player, reshaping the game.

## Comparison with Engine Baseline

| Metric | This Game (Human Avg) | Engine v0.4.3 | Engine Current (d4) |
|--------|----------------------|---------------|---------------------|
| Pawn ratio (first 20) | 34% | 53% | TBD at depth 6+ |
| Queen activation | Round 4 avg | Never in 10 rounds | TBD |
| Castling | 3/4 by round 17 | 0/4 | TBD |
| Captures in first 20 | 3 avg | 0 | TBD |
| Knight undevelopment | 0 | 1 | TBD |
