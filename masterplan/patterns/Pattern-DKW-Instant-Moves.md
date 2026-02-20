---
type: pattern
tags:
  - stage/03
  - area/gamestate
last_updated: 2026-02-20
---

# Pattern: DKW (Dead King Walking) Instant Moves

## When to Use

When a player is eliminated in DKW game mode and their king remains on the board. The eliminated player's king continues making random legal moves between active players' turns.

## The Problem

In standard 4PC, when a player is eliminated (checkmated, stalemated, resigned, timed out), their pieces are either removed, converted to terrain, or left inert. In DKW mode, the eliminated player's king gets special treatment: it stays on the board and moves randomly on its own, between the active players' turns. This creates a chaotic element -- the dead king can block squares, interfere with plans, and even get stuck.

DKW moves are **instant** -- they happen between turns, not as a full turn. The DKW player does not get a proper turn in the rotation. This requires careful manipulation of `side_to_move` to generate legal moves for a player who is not actually active.

## The Pattern

The DKW move sequence within GameState:

```rust
// 1. Save the current side_to_move
let saved_stm = board.side_to_move();

// 2. Temporarily set side_to_move to the DKW player
board.set_side_to_move(dkw_player);

// 3. Generate king-only legal moves for the DKW player
let king_moves = generate_legal(&mut board)
    .into_iter()
    .filter(|mv| mv.piece_type() == PieceType::King)
    .collect::<Vec<_>>();

// 4. Pick a random move (if any exist)
if let Some(chosen) = king_moves.choose(&mut rng) {
    // 5. Execute the move permanently (no unmake)
    make_move(&mut board, *chosen);
} else {
    // 6. No legal king moves -> eliminate the DKW player entirely (DkwKingStuck)
    eliminate_player(dkw_player, EliminationReason::DkwKingStuck);
}

// 7. Restore the original side_to_move
board.set_side_to_move(saved_stm);
```

### Key properties

1. **DKW moves are permanent.** `make_move` is called but `unmake_move` is never called. The move is part of the game history.
2. **Only king moves.** The DKW player's other pieces (if they still exist and are not terrain) do not move. Only the king gets DKW movement.
3. **Random selection.** The move is chosen randomly from the set of legal king moves. This is intentional -- DKW is a chaos mechanic, not a strategic one.
4. **DkwKingStuck elimination.** If the DKW king has no legal moves (completely surrounded by terrain, board edge, or other pieces), the DKW player is fully eliminated and the king is removed or converted.
5. **Side-to-move manipulation.** The `set_side_to_move` / restore pattern is necessary because `generate_legal` generates moves for `board.side_to_move()`. The temporary swap lets MoveGen work correctly without knowing about DKW.

## Timing

DKW moves happen in the elimination chain within `apply_move`:

```
apply_move(active_player_move)
  -> make_move
  -> score capture
  -> score check bonus
  -> push history
  -> advance turn
  -> check elimination chain
     -> player eliminated!
        -> if DKW mode: process DKW instant move  <-- HERE
  -> check game-over
```

Multiple DKW moves can happen in a single `apply_move` call if multiple players are eliminated simultaneously.

## Gotchas

- **Halfmove clock increment.** `make_move` increments `halfmove_clock`. DKW moves therefore count toward the 50-move rule. See [[Issue-DKW-Halfmove-Clock]].
- **Zobrist hash changes.** The DKW move changes the board's Zobrist hash. This affects position history and repetition detection. The DKW position is pushed to history as part of the game record.
- **Cascading DKW.** A DKW king move could theoretically expose another player to checkmate, triggering another elimination, which could trigger another DKW move. The elimination chain must handle this recursively.

## Examples

- `gamestate/mod.rs` -- DKW processing within the elimination chain
- Any integration test that plays a full game in DKW mode

## Anti-Patterns

- **Giving DKW players a full turn** -- DKW moves are instant, not turn-based. The DKW player is never in the turn rotation.
- **Using unmake_move for DKW** -- DKW moves are permanent game state changes, not speculative search moves.
- **Strategic DKW move selection** -- the spec says random. Do not add evaluation or search for DKW moves.
- **Skipping set_side_to_move** -- MoveGen generates moves for `board.side_to_move()`. Without the temporary swap, it would generate moves for the wrong player.

## Related

- [[Component-GameState]] -- where DKW is implemented
- [[Connection-MoveGen-to-GameState]] -- how DKW uses MoveGen
- [[Issue-DKW-Halfmove-Clock]] -- halfmove clock concern
- [[stage_03_gamestate]] -- spec for DKW mode
- [[stage_17_variants]] -- future DKW tuning
- [[4PC_RULES_REFERENCE]] -- DKW rules source
