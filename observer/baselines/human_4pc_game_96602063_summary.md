# Baseline: Human 4PC FFA — Game 96602063 (Very Weak Lobby)

**Source:** chess.com 4-Player Chess (FFA)
**Players:** Player-C (2709) | Player-H (1954) | Player-M (2139) | Player-N (2069)
**Result:** Red 63, Blue 3, Yellow 30, Green 30 | Termination: Green +20
**Total Moves:** 160 (50 rounds) | Blue checkmated round 14
**Date:** 2026-02-09

## Why This Game Matters

Contains the **weakest player in the dataset** (Player-H at 1954 Elo, 3 points). Blue never activates queen, pushes 5 pawns in first 6 moves, gets checkmated round 14. This is the absolute floor — if the engine plays like this Blue, it's sub-2000.

Also shows Player-C adapting strategy: uses a pawn storm + deliberate king walk instead of his usual low-pawn-ratio piece play. Strong players adjust to weak opponents.

## Opening Patterns

| Player | Elo | Pawn Ratio | Queen Active | Castled | Captures | Fate |
|--------|-----|-----------|-------------|---------|----------|------|
| Red    | 2709 | 40%* | Round 6 | No (deliberate) | 3 | **Won (63 pts)** |
| Blue   | 1954 | **54%** | **Never** | Round 11 | 1 | **Checkmated R14, 3 pts** |
| Yellow | 2139 | 45% | Round 11 | Round 7 | 4 | Survived (30 pts) |
| Green  | 2069 | 40% | Round 6 | Round 16 | 3 | Survived (30 pts) |

*Player-C's 40% is deliberate pawn storm strategy, not weakness.

## Sub-2000 Play: The Absolute Floor

Blue (Player-H, 1954) shows what truly terrible play looks like:
- **54% pawn ratio** — highest in entire 6-game dataset
- **5 of first 6 moves are pawn pushes** (b7-c7, b4-d4, b9-d9, b11-c11, b8-c8)
- **Queen NEVER activated** — still sitting on a8 when checkmated
- **Only 1 capture** (forced recapture b6xNc5)
- **3 points** — worst result in the entire dataset

If the Odin engine's behavior matches this profile, it's playing below 2000 Elo.

## New Pattern: "Single-Pawn Storm"

Yellow pushes the h-pawn 5 times: h13→h11→h10→h9→h8→h7=D. Five tempo invested in one pawn promotion. While this eventually creates a queen, the cost is enormous — no other development during those 5 moves. The engine should be checked for similar single-file pawn obsession.

## New Pattern: "Deliberate King Walk" (Strong Player)

Player-C plays Kh1→i1→j1 (rounds 10-11) instead of castling. This is NOT a blunder — it's a deliberate repositioning to support the pawn storm. Key difference from weak-player king walks:
- Player-C's king moves TOWARD a safe square behind pawns
- Weak players' kings move INTO danger with no shelter
- Context matters when evaluating king displacement

## Updated Cross-Game Elo Scale

| Elo | Pawn Ratio | Queen | Captures/20 | Result Range |
|-----|-----------|-------|-------------|-------------|
| <2000 | **54%** | **Never** | 1 | 3 pts |
| 2000-2150 | 40-50% | Round 8-12 | 1-3 | 18-30 pts |
| 2200-2300 | 35-45% | Round 5-9 | 1-4 | 15-36 pts |
| 2500-2700 | 30-40% | Round 3-7 | 2-3 | 47-65 pts |
| 3000+ | **20-30%** | **Round 2-5** | **4-5** | **63-87 pts** |
