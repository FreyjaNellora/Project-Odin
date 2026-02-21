# Session: 2026-02-21 Bugfix Session

**Date:** 2026-02-21
**Stage context:** Stage 7 complete; this session resolves post-completion regressions
**Engineer:** Claude (Sonnet 4.6)

## Work Done

### Semi-auto regression fix (completed early in session)

After Stage 7's improved protocol event chain, the engine started taking over the human player's turn in semi-auto mode.

**Root cause:** `humanPlayerRef.current === null` caused `shouldEnginePlay` to return `true` for all players.
**Fix:** Added null guard, removed `disabled={gameInProgress}` from player selector, added chain-stop on player change.
**Files:** `odin-ui/src/hooks/useGameState.ts`, `odin-ui/src/components/GameControls.tsx`
**Tests:** 45/45 npm tests passing.

### Checkmate detection fix (completed this session)

Red's king was in checkmate after Green played n7i2 but the engine did not detect it — emitting `nextturn Red` instead of eliminating Red and advancing to Blue.

**Root cause (A):** `check_elimination_chain` ran before `process_dkw_moves` in `apply_move`. A DKW piece that Red could capture gave a false "HasMoves" result; the DKW piece then wandered away, leaving Red truly mated.
**Root cause (B):** `handle_go` early-returned an error when `legal_moves().is_empty()`, never reaching elimination logic.
**Fix:** Swapped step order in `apply_move` (DKW first, then checkmate detection); added `handle_no_legal_moves()` to `GameState`; restructured `handle_go` to handle no-legal-moves with proper elimination + recursive bestmove search.
**Files:** `odin-engine/src/gamestate/mod.rs`, `odin-engine/src/protocol/mod.rs`
**Tests:** 199/199 cargo lib tests passing (3 new tests added).

## Issues Created

- [[Issue-SemiAuto-HumanPlayer-Guard]]
- [[Issue-Checkmate-Detection-DKW-Ordering]]

## Next

Stage 8 — BRS hybrid scoring and move classification. See MASTERPLAN.md Stage 8 spec.
