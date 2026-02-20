---
type: pattern
tags:
  - stage/03
  - area/movegen
  - area/gamestate
last_updated: 2026-02-20
---

# Pattern: Terrain-Aware Move Generation

## When to Use

Whenever move generation or attack detection encounters a piece with `PieceStatus::Terrain`. This applies to the terrain game mode where eliminated players' pieces become impassable obstacles on the board.

## The Problem

In terrain mode, when a player is eliminated, their pieces remain on the board but change behavior entirely. They become walls: they cannot be captured, they cannot move, they do not deliver check, and they block sliding piece rays. This behavior must be enforced consistently across all MoveGen functions without leaking game-rule knowledge into the MoveGen layer.

## The Pattern

Terrain awareness is implemented **at the MoveGen level**, not at the GameState level. GameState only triggers the conversion (calling `Board::set_piece_status()` on each eliminated player's piece). MoveGen enforces the restrictions by checking `is_terrain()` at three key points:

### 1. attacks.rs: Terrain pieces do not attack or give check

```rust
// Guard: skip terrain pieces when computing attacks
if board.piece_at(sq).map_or(false, |p| p.is_terrain()) {
    continue; // terrain piece does not attack
}
```

When checking if a square is attacked, terrain pieces are skipped entirely. They are invisible to attack queries. This means:
- A terrain rook on the same rank as a king does **not** deliver check
- A terrain knight adjacent to a king does **not** threaten it
- `is_in_check` returns false even if terrain pieces "point at" the king

### 2. attacks.rs: Terrain pieces block sliding rays

```rust
// When walking a ray for sliding pieces (bishop, rook, queen):
if board.piece_at(next_sq).map_or(false, |p| p.is_terrain()) {
    break; // terrain piece blocks the ray, just like a board edge
}
```

Sliding piece rays (bishop diagonals, rook ranks/files, queen both) terminate when hitting a terrain piece. The terrain piece itself is not capturable -- the ray simply stops. This is identical to hitting the board edge or an invalid corner square.

### 3. generate.rs: Terrain pieces are impassable

```rust
// When generating moves for a piece:
if board.piece_at(target_sq).map_or(false, |p| p.is_terrain()) {
    continue; // cannot move to or capture a terrain piece
}
```

No active piece can move to a square occupied by a terrain piece. This applies to all piece types:
- Pawns cannot capture terrain pieces (even diagonally)
- Knights cannot land on terrain squares
- Sliding pieces stop before terrain squares (per point 2 above)
- Kings cannot move to terrain-occupied squares

## Why This Design

Implementing terrain at the MoveGen level (rather than GameState level) means:
1. All MoveGen consumers automatically get terrain-correct behavior for free
2. No need to special-case terrain in check detection, legal filtering, or evaluation
3. The `generate_legal` function produces correct results without knowing about the terrain game mode
4. Search (Stage 7+) gets correct terrain behavior without any terrain-specific code

GameState only needs to call `set_piece_status(sq, PieceStatus::Terrain)` once per eliminated piece. Everything else flows naturally from MoveGen's guards.

## Examples

- `attacks.rs` -- `is_square_attacked_by()` skips terrain pieces
- `attacks.rs` -- ray walking breaks on terrain
- `generate.rs` -- move target validation rejects terrain squares
- `gamestate/mod.rs` -- `eliminate_player()` calls `set_piece_status` for terrain conversion

## Anti-Patterns

- **Filtering terrain at the GameState level** -- this would require GameState to post-process MoveGen output, removing moves that interact with terrain. Fragile and duplicates logic.
- **Removing terrain pieces from the board** -- terrain pieces must remain on the board as physical obstacles. Removing them changes the game geometry.
- **Treating terrain as capturable** -- terrain pieces award zero points and cannot be removed. They are permanent fixtures.

## Related

- [[Component-MoveGen]] -- where the terrain guards are implemented
- [[Component-GameState]] -- where terrain conversion is triggered
- [[Connection-MoveGen-to-GameState]] -- the interaction between GameState and MoveGen for terrain
- [[stage_03_gamestate]] -- spec for terrain mode
- [[stage_17_variants]] -- future variant tuning for terrain
