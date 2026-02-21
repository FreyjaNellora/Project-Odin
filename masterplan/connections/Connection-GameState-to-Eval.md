---
type: connection
tags:
  - stage/06
  - area/eval
  - area/gamestate
last_updated: 2026-02-21
---

# Connection: GameState to Eval

## What Connects

- [[Component-GameState]] (provider)
- [[Component-Eval]] (consumer)

## How They Communicate

Eval receives `&GameState` as the primary input to both trait methods. It reads:

| API | Returns | Used For |
|---|---|---|
| `position.board()` | `&Board` | Material counting, PST lookup, king safety, attack queries |
| `position.score(player)` | `i32` | FFA points integration in eval |
| `position.scores()` | `&[i32; 4]` | Lead penalty calculation |
| `position.player_status(player)` | `PlayerStatus` | Skip eliminated players, return -30000 floor |

Eval also uses Board APIs indirectly:
- `board.piece_list(player)` -- iterate pieces for material/PST
- `board.piece_at(sq)` -- check piece status (alive/dead/terrain)
- `board.king_square(player)` -- king safety and threat penalty

And MoveGen API:
- `is_square_attacked_by(sq, attacker, board)` -- king safety and threat penalty

## Contract

1. **Eval never mutates GameState.** All methods take `&GameState`, not `&mut`.
2. **Eval never calls `legal_moves()` or `apply_move()`.** It's a static evaluator -- no lookahead.
3. **Eval handles all PlayerStatus variants.** Active = normal eval. Eliminated = -30000. DeadKingWalking = treated as active (DKW pieces are Dead status, so they contribute 0cp material automatically).
4. **Eval uses the attack query API (ADR-001).** Never reads `board.squares[]` directly.

## Evolution

| Stage | Change |
|---|---|
| 6 (current) | GameState provides read-only context to bootstrap eval |
| 7 | Search creates GameState clones, calls eval on leaf positions |
| 10 | MCTS uses eval_4vec for rollout value estimates |
| 16 | NnueEvaluator replaces BootstrapEvaluator; same trait, same GameState input |
