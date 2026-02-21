---
type: moc
tags:
  - type/moc
last_updated: 2026-02-21---

# Active Issues

Open problems, workarounds, and tech debt. Updated as issues are created and resolved.

## Blocking

_None._

## Warning

- [[Issue-Perft-Values-Unverified]] -- Stage 2 perft values self-consistent but no external reference exists to cross-check
- [[Issue-Vec-Clone-Cost-Pre-MCTS]] -- Board.piece_lists (Vec) and GameState.position_history (Vec) cause heap allocation on every clone; must retrofit to fixed-size/Arc before Stage 10 (MCTS)

## Notes

- [[Issue-Huginn-Gates-Unwired]] -- Stages 1-7 Huginn observation gates not wired; deferred until buffer plumbing exists
- [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]] -- Lead-penalty heuristic causes BRS to prefer checks over captures in some positions; Stage 8 to fix
- [[Issue-DKW-Halfmove-Clock]] -- DKW instant moves increment halfmove_clock via make_move; may cause premature 50-move rule triggers in DKW games
- [[Issue-DKW-Invisible-Moves-UI]] -- DKW king instant moves not visible in UI rendering cache; visual desync after DKW events

## Recently Resolved

- [[Issue-UI-EP-False-Positive]] -- En passant false positive for Blue/Green pawns in UI display cache. Fixed: require both file AND rank to change. (2026-02-20)
- [[Issue-UI-Castling-Blue-Green]] -- Castling display broken for Blue/Green in UI. Fixed: orientation-aware detection. (2026-02-20)
- [[Issue-UI-AdvancePlayer-React-Batching]] -- advancePlayer returned wrong player due to React 18 batching. Fixed: use ref instead of state updater. (2026-02-20)
- [[Issue-EP-Representation-4PC]] -- En passant stored file index, insufficient for 4PC. Fixed Stage 2: now stores full square index. (2026-02-20)
