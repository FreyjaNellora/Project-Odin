---
type: moc
tags:
  - type/moc
last_updated: 2026-02-25
---

# Active Issues

Open problems, workarounds, and tech debt. Updated as issues are created and resolved.

## Blocking

_None._

## Warning

- [[Issue-GameLog-Player-Label-React-Batching]] -- Game log labeled every move with the *next* player (React 18 batching — deferred updater reads wrong ref). Fixed `b98c087`; pending user verification.
- [[Issue-Perft-Values-Unverified]] -- Stage 2 perft values self-consistent but no external reference exists to cross-check
- [[Issue-Vec-Clone-Cost-Pre-MCTS]] -- Board.piece_lists (Vec) and GameState.position_history (Vec) cause heap allocation on every clone; must retrofit to fixed-size/Arc before Stage 10 (MCTS)

## Notes

- [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]] -- Lead-penalty heuristic causes BRS to prefer checks over captures in some positions; Stage 8 to fix
- [[Issue-DKW-Halfmove-Clock]] -- DKW instant moves increment halfmove_clock via make_move; may cause premature 50-move rule triggers in DKW games
- [[Issue-DKW-Invisible-Moves-UI]] -- DKW king instant moves not visible in UI rendering cache; visual desync after DKW events

## Recently Resolved

- [[Issue-Huginn-Gates-Unwired]] -- Huginn telemetry system retired in Stage 8; replaced with `tracing` crate (ADR-015). Gates no longer needed. (2026-02-23)
- [[Issue-Promotion-Wrong-Ranks-No-UI]] -- UI used wrong promotion ranks (board edges instead of midline), no piece selection dialog, and wrong suffix (`q` instead of `w` for PromotedQueen). Fixed: correct ranks, PromotionDialog component, `w` suffix. (2026-02-22)
- [[Issue-SemiAuto-HumanPlayer-Guard]] -- Semi-auto engine took over human's turn when no player selected. Fixed: null guard in shouldEnginePlay, removed disabled from player selector. (2026-02-21)
- [[Issue-Checkmate-Detection-DKW-Ordering]] -- Checkmate not detected due to DKW ordering bug + protocol early return + UI parser dropping reason-suffixed events. All three bugs fixed. (2026-02-21)
- [[Issue-UI-EP-False-Positive]] -- En passant false positive for Blue/Green pawns in UI display cache. Fixed: require both file AND rank to change. (2026-02-20)
- [[Issue-UI-Castling-Blue-Green]] -- Castling display broken for Blue/Green in UI. Fixed: orientation-aware detection. (2026-02-20)
- [[Issue-UI-AdvancePlayer-React-Batching]] -- advancePlayer returned wrong player due to React 18 batching. Fixed: use ref instead of state updater. (2026-02-20)
- [[Issue-EP-Representation-4PC]] -- En passant stored file index, insufficient for 4PC. Fixed Stage 2: now stores full square index. (2026-02-20)
