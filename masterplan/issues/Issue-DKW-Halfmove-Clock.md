---
type: issue
date_opened: 2026-02-20
last_updated: 2026-02-20
date_resolved:
stage: 3
severity: note
status: open
tags:
  - stage/03
  - area/gamestate
  - severity/note
---

# DKW Instant Moves Increment Halfmove Clock

## Description

DKW (Dead King Walking) instant moves are executed via `make_move()`, which increments `Board::halfmove_clock` on every non-capture, non-pawn move. Since DKW kings only make king moves (non-pawn) and rarely capture (they move randomly), most DKW moves increment the halfmove clock.

This means that in DKW mode, the 50-move rule may trigger earlier than expected. A game with active DKW kings will accumulate halfmove clock ticks faster than a game without DKW, because the clock counts DKW moves in addition to active player moves.

## Impact

- In practice, this is unlikely to be a problem in most games. DKW kings typically get stuck or eliminated quickly.
- However, in edge cases where multiple DKW kings survive for many turns, the 50-move rule could trigger prematurely from the active players' perspective.
- The 4PC rules reference ([[4PC_RULES_REFERENCE]]) does not explicitly address whether DKW moves should count toward the 50-move rule.

## Affected Components

- [[Component-GameState]] -- the DKW processing loop calls `make_move`
- [[Component-MoveGen]] -- `make_move` increments `halfmove_clock`
- [[Connection-MoveGen-to-GameState]] -- the make_move usage path

## Workaround

None needed currently. The behavior is functional and does not cause crashes or incorrect game results. It is a rules-ambiguity concern.

Possible future fix if needed: save `halfmove_clock` before DKW move, call `make_move`, restore `halfmove_clock` to the saved value. This would make DKW moves invisible to the 50-move rule.

## Resolution

_Open. Not blocking. May need revisiting if the 50-move rule triggers unexpectedly in DKW games during testing (Stage 12+ self-play)._

## Related

- [[Pattern-DKW-Instant-Moves]] -- the DKW move pattern documentation
- [[stage_03_gamestate]] -- spec
- [[stage_17_variants]] -- future variant tuning may address this
- [[4PC_RULES_REFERENCE]] -- rules source (silent on this question)
