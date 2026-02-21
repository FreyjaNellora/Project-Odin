---
type: pattern
tags:
  - stage/02
  - area/movegen
last_updated: 2026-02-20
---

# Pattern: Pawn Reverse Lookup in 4PC

## When to Use

Whenever you need to answer "is this square attacked by player P's pawns?" -- used in check detection, legal filtering, and later in evaluation and SEE.

## The Problem

In standard 2-player chess, pawn attacks are simple: white attacks diagonally up, black attacks diagonally down. To check if a square is attacked by a white pawn, look diagonally down from the target.

In 4PC, four players face four different directions:
- Red: +rank (captures diag: +rank +/-file)
- Blue: +file (captures diag: +file +/-rank)
- Yellow: -rank (captures diag: -rank +/-file)
- Green: -file (captures diag: -file +/-rank)

To check "does player P's pawn attack square S?", you need to look at squares from which P's pawns **could** capture to S. This is the **reverse** of P's capture direction.

## The Pattern

The reverse of player P's capture direction equals the **opposite-facing player's** capture direction. In 4PC with clockwise turn order:

```
opposite(P) = (P + 2) % 4
```

- Red (0) <-> Yellow (2) -- face opposite on rank axis
- Blue (1) <-> Green (3) -- face opposite on file axis

So to find squares that attack target S from player P's perspective:

```rust
let reverse_player = Player::from_index((attacker.index() + 2) % 4);
let attack_sources = pawn_attacks[reverse_player.index()][target_square];
```

Then check if any of those source squares contain player P's pawn.

## Why It Works

Pawn captures are directional. The set of squares that player P can capture **to** from square A is the same set of squares that the opposite-facing player can capture **to** from square A. Reversing the lookup is equivalent to using the opposite player's attack table.

## Anti-Patterns

- **Using the attacker's own pawn table for reverse lookup** -- this was the original bug in Stage 2. It works in 2-player chess (where reverse = same table reflected) but breaks in 4PC where 4 orthogonal directions exist.
- **Hard-coding direction offsets** -- fragile and duplicates the table data. Use the pre-computed tables.

## Related

- [[Component-MoveGen]] -- where this pattern is implemented
- [[audit_log_stage_02]] -- documents the bug this pattern fixed
- [[downstream_log_stage_02]] -- reasoning section #1
