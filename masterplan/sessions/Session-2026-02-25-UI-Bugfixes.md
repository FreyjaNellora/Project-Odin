---
type: session
date: 2026-02-25
stage: "Post-Stage 8 (non-stage work)"
tags:
  - type/session
  - area/ui
  - area/search
---

# Session: 2026-02-25 — UI Bugfixes + In-Search Repetition

## What Was Done

### 1. In-Search Repetition Detection (`odin-engine/src/search/brs.rs`)

Engine was cycling moves and reaching threefold repetition draws because `make_move`/`unmake_move` don't update `position_history`. Fix: snapshot game history at search start (`game_history: Vec<u64>`), maintain a path-local `rep_stack` pushed/popped in `max_node`/`min_node` (not in `alphabeta` to avoid multi-return-path cleanup problems). Check `game_count + search_count >= 3` at each node (ply > 0).

- 233 unit + 128 integration = 361 tests pass, 3 ignored.
- Committed: `f50fc57`

### 2. Search Depth Default: 6 → 7 (`odin-engine/src/protocol/mod.rs`)

Bare `go` from UI falls into `limits_to_budget` fallback. Changed `max_depth: Some(6)` → `Some(7)`.

### 3. Piece-Prefix Notation in Game Log (`odin-ui/src/hooks/useGameState.ts`)

Moves now display as `Nj1i3` instead of bare `j1i3`. Added:
- `boardRef: useRef<(Piece | null)[]>` — mirror of board state for synchronous access in async callbacks
- `boardRef.current` kept in sync alongside every `setBoard` call (in `applyMoveToBoard`, `newGame`, `eliminated` handler)
- `pieceLetterPrefix(piece)` and `formatMoveForDisplay(moveStr, board)` helpers

### 4. Game Log Player Labels Shifted by One — Root Bug Fixed

**Bug:** Every move was labeled with the *next* player, not the player who actually moved.

**Root cause:** In both the `bestmove` and `readyok` handlers, `currentPlayerRef.current` was read inside a React functional updater passed to `setMoveHistory`. Due to React 18 automatic batching, the updater executes during the next render flush — by which time the ref has already been mutated to the next player (via `pendingNextTurnRef` assignment or `advancePlayer()`). Similarly `boardRef.current` was read inside the updater.

**Fix:** Capture both refs as local variables **before** any mutations:
```typescript
const movingPlayer = currentPlayerRef.current;
const movingBoard = boardRef.current.slice();
setMoveHistory((prev) => [
  ...prev,
  { move: formatMoveForDisplay(engineMove, movingBoard), player: movingPlayer, info: infoSnapshot },
]);
applyMoveToBoard(engineMove);
// ... then update refs
```
Applied in both `bestmove` and `readyok` handlers.

This same pattern was already used correctly in Stage 5 for `advancePlayer` (see [[Session-2026-02-20-Stage05-Bugfix]]), but not carried forward when `setMoveHistory` was added in the UI QoL session.

All changes committed as `b98c087`.

## What This Revealed

The "Red king exposing itself" observation from earlier testing was an artifact of the label bug — the moves attributed to "Red" in the game log were actually Green's moves. With correct labels, engine play appears reasonable. King safety evaluation may still be weak (see [[Component-KingSafety]]) but needs fresh playthroughs with correct labels to confirm.

## Files Modified

- `odin-engine/src/search/brs.rs` — repetition detection fields + push/pop
- `odin-engine/src/protocol/mod.rs` — depth default 6 → 7
- `odin-ui/src/hooks/useGameState.ts` — boardRef pattern, piece notation, player label fix

## Test Counts

- Engine: 361 (233 unit + 128 integration, 3 ignored) — unchanged
- UI Vitest: 54 — unchanged (no new tests added; TypeScript clean)

## Commits

- `f50fc57` — `[Search] Add in-search repetition detection to BRS`
- `b98c087` — `[UI] Fix game log player labels + add piece-prefix notation + depth 7 default`
