---
type: moc
tags:
  - type/moc
last_updated: 2026-02-27
---

# Active Issues

Open problems, workarounds, and tech debt. Updated as issues are created and resolved.

## Blocking

_None._

## Warning

- [[Issue-Pawn-Push-Preference-King-Walk]] -- MITIGATED — eval-side fixes applied (dev bonuses increased, pawn advance gate, king displacement penalty). Full fix requires MCTS (Stage 10). (2026-02-27)
- ~~[[Issue-GameLog-Player-Label-React-Batching]]~~ -- Game log player labels fixed. User verified 2026-02-27. Scores display correctly, no info duplication.
- [[Issue-Perft-Values-Unverified]] -- Stage 2 perft values self-consistent but no external reference exists to cross-check

## Notes

- [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]] -- Lead-penalty heuristic causes BRS to prefer checks over captures in some positions; Stage 8 to fix
- [[Issue-DKW-Halfmove-Clock]] -- DKW instant moves increment halfmove_clock via make_move; may cause premature 50-move rule triggers in DKW games
- [[Issue-DKW-Invisible-Moves-UI]] -- DKW king instant moves not visible in UI rendering cache; visual desync after DKW events
- [[Issue-UI-React-Hooks-Queue-Error]] -- React hooks queue error on UI load; likely Vite HMR artifact, refresh fixes it

## Recently Resolved

- [[Issue-Vec-Clone-Cost-Pre-MCTS]] -- RESOLVED — piece_lists to fixed-size arrays, position_history to Arc<Vec<u64>>. Zero heap alloc on Board/GameState clone. (2026-02-27)
- [[Issue-BRS-Paranoid-Opponent-Modeling]] -- Hybrid scoring 80/20 paranoid blend too aggressive; tuned likelihood constants to 50/50 (0.7→0.5 base, 0.3→0.5 exposed penalty, 0.2→0.3 non-root). (2026-02-27)
- [[Issue-TT-Not-Player-Aware]] -- TT hash missing root_player; added root_player Zobrist keys XOR'd into tt_hash for TT probe/store. (2026-02-27)
- [[Issue-TT-Fresh-Per-Search]] -- TT discarded between moves; BrsSearcher now persisted in OdinEngine state. (2026-02-27)
- [[Issue-Hanging-Piece-Eval-Double-Count]] -- Eval-side hanging piece penalty double-counted capture threats handled by search; caused Nf3→e1 retreat regression. Reverted immediately; correct approach is search-side narrowing protection. (2026-02-26)
- [[Issue-PostElim-BRS-Crash]] -- Engine panicked after any player elimination: BRS alphabeta/quiescence called generate_legal on a kingless board. Four-layer fix (alphabeta skip, quiescence skip, board scanner Active filter, king square sentinel). User verified 2026-02-25.
- [[Issue-Huginn-Gates-Unwired]] -- Huginn telemetry system retired in Stage 8; replaced with `tracing` crate (ADR-015). Gates no longer needed. (2026-02-23)
- [[Issue-Promotion-Wrong-Ranks-No-UI]] -- UI used wrong promotion ranks (board edges instead of midline), no piece selection dialog, and wrong suffix (`q` instead of `w` for PromotedQueen). Fixed: correct ranks, PromotionDialog component, `w` suffix. (2026-02-22)
- [[Issue-SemiAuto-HumanPlayer-Guard]] -- Semi-auto engine took over human's turn when no player selected. Fixed: null guard in shouldEnginePlay, removed disabled from player selector. (2026-02-21)
- [[Issue-Checkmate-Detection-DKW-Ordering]] -- Checkmate not detected due to DKW ordering bug + protocol early return + UI parser dropping reason-suffixed events. All three bugs fixed. (2026-02-21)
- [[Issue-UI-EP-False-Positive]] -- En passant false positive for Blue/Green pawns in UI display cache. Fixed: require both file AND rank to change. (2026-02-20)
- [[Issue-UI-Castling-Blue-Green]] -- Castling display broken for Blue/Green in UI. Fixed: orientation-aware detection. (2026-02-20)
- [[Issue-UI-AdvancePlayer-React-Batching]] -- advancePlayer returned wrong player due to React 18 batching. Fixed: use ref instead of state updater. (2026-02-20)
- [[Issue-EP-Representation-4PC]] -- En passant stored file index, insufficient for 4PC. Fixed Stage 2: now stores full square index. (2026-02-20)
