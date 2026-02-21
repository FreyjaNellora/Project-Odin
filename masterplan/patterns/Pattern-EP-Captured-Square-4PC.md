---
type: pattern
tags:
  - stage/02
  - area/movegen
last_updated: 2026-02-20
---

# Pattern: En Passant Captured Square in 4PC

## When to Use

During `make_move` when executing an en passant capture. You need to find the square of the pawn that will be removed (the one that double-stepped).

## The Problem

In standard chess, the captured pawn is trivially located: one rank behind the EP target square from the capturing player's perspective.

In 4PC, the capturing player and the pushing player may face **completely different directions**. A Blue pawn (moving +file) might capture a Red pawn (that moved +rank) via en passant. Using the capturing player's backward direction gives the wrong square.

## The Pattern

The captured pawn is always at:

```
ep_target + pushing_player's forward direction
```

Since EP lasts exactly one turn in 4PC, the pushing player is always the **previous player** in turn order:

```rust
let pushing_player = prev_player(capturing_player);
```

Then compute the captured pawn's square by stepping one square in the pushing player's forward direction from the EP target.

## Why It Works

The EP target square is the midpoint of the double push. The captured pawn ended its move one square beyond the target in the pushing player's forward direction. Since EP lasts exactly one turn, the pushing player is always `(current_player + 3) % 4` (i.e., the player who just moved).

## Anti-Patterns

- **Using the capturing player's backward direction** -- this was the original bug. It works in 2-player chess (both players face the same axis) but fails in 4PC when players face orthogonal directions.
- **Assuming EP only happens between opposite-facing players** -- any player can capture any other player's pawn EP, as long as their diagonal capture reaches the target square.

## Related

- [[Component-MoveGen]] -- where this pattern is implemented
- [[audit_log_stage_02]] -- documents the bug this pattern fixed
- [[downstream_log_stage_02]] -- reasoning section #2
