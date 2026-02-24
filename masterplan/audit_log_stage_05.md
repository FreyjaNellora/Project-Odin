# Audit Log — Stage 05: BasicUI

## Pre-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build`)
- Tests pass: Yes (229 total: 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04)
- Previous downstream flags reviewed: Stages 0-4 (full dependency chain)

### Findings

**From Stage 0 downstream log:**
- [Historical] Huginn macro and buffer available. No gates wired (deferred). No impact on Stage 5.

**From Stage 1 downstream log:**
- Board is `[Option<Piece>; 196]`, index = `rank * 14 + file`. 36 corners invalid. UI must mirror square geometry.
- FEN4 format documented: `<ranks> <side> <castling> <ep> <halfmove> <fullmove>`.
- Piece lists: per-player `Vec<(PieceType, Square)>`. UI tracks pieces via local rendering cache, not engine internals.

**From Stage 2 downstream log:**
- WARNING (Issue-Perft-Values-Unverified): Perft values not independently verified. No impact on UI.
- Attack query API is the board boundary. UI does not need attack queries (no game logic).
- En passant stores target square (not file). UI display-cache handles EP removal heuristically.

**From Stage 3 downstream log:**
- DKW instant moves happen inside `apply_move()`. Protocol does not expose them separately. Accepted limitation for Stage 5.
- `position_history` grows unbounded. Not a UI concern.
- DKW increments halfmove_clock (grey area in rules). Not a UI concern.

**From Stage 4 downstream log:**
- Protocol is blocking/synchronous. `go` returns instantly (random move). Acceptable for Stage 5.
- Engine does NOT apply `bestmove` to its own state. UI must re-send full `position startpos moves <all moves>` before every `go`.
- On invalid move in `position` command, engine clears game state (`game_state = None`). UI must re-send valid position for error recovery.
- Move strings matched against legal moves (no `Move::from_algebraic()`). UI relies on engine for validation.
- `setoption` recognizes `Debug` and `Terrain`. All other options silently accepted.

### Risks for This Stage

1. **Board state desync (Section 2.6):** UI maintains a local rendering cache. If display-side move handling (castling rook, EP capture, promotion) is wrong, the board will visually desync from the engine's actual state. Mitigation: engine is the authority; UI can always reset via "New Game".
2. **DKW invisible moves:** DKW kings make instant random moves not reported through the protocol. The UI's rendering cache will not know about them. Accepted limitation — DKW only occurs after resignation/checkmate, rare in manual play.
3. **Tauri v2 Windows compatibility (Section 2.23):** Requires WebView2 on Windows 10. Target OS (10.0.19045) should support it. Verify during setup.
4. **Engine path resolution:** During development, engine binary at `../odin-engine/target/debug/odin-engine.exe`. Must ensure the path is resolved correctly from the Tauri app's working directory.
5. **No game logic in UI (permanent invariant):** Must be vigilant that display-side helpers (castling rook movement, EP capture removal) remain display logic only and never validate legality.


---

## Post-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| SVG board with 160 valid squares | Done | BoardDisplay.tsx renders 14x14 grid with corner exclusion |
| Starting position with 64 pieces | Done | startingPosition() mirrors board_struct.rs exactly, verified by 29 unit tests |
| Engine subprocess spawning | Done | EngineManager spawns odin-engine with piped stdio |
| Protocol communication (IPC) | Done | Tauri commands: spawn_engine, send_command, kill_engine |
| Protocol parser | Done | parseEngineOutput() handles all Odin Protocol formats, 16 unit tests |
| Click-to-move with engine validation | Done | Two-click input, sends position command, engine validates |
| Debug console | Done | Scrollable log with color coding, parsed info summary, manual input |
| Game controls (new game, terrain) | Done | New Game, New Game (Terrain), Engine Move buttons |
| Status bar | Done | Engine name, connection status |
| No game logic in UI | Verified | UI never generates legal moves, never checks legality, never evaluates |

### Code Quality
#### Uniformity

Consistent patterns throughout. React components use functional style with hooks. TypeScript types in `types/`, CSS in `styles/`, hooks in `hooks/`, utilities in `lib/`. Rust backend follows standard Tauri v2 patterns.

#### Bloat

No unnecessary dependencies. Vitest is the only new dev dependency. No state management library (useState/useReducer suffices). No unnecessary abstractions.

#### Efficiency

SVG board renders 160 rectangles — trivial for modern browsers. Protocol parser uses string matching. Log buffer capped at 1000 lines.

#### Dead Code

None identified. All components wired into App.tsx. All exports used.

#### Broken Code

WARNING: `tauri dev` not tested end-to-end in this session due to environment constraints. All individual compilations verified (Rust backend, TypeScript frontend, engine tests).

#### Temporary Code

Engine path resolution in engine.rs uses hardcoded dev paths. Accepted for Stage 5 — sidecar bundling is a production concern.

### Search/Eval Integrity

N/A — Stage 5 adds no search or evaluation logic. All engine tests (229) still pass.

### Future Conflict Analysis

1. **Stage 7 (BRS Search):** When actual search replaces random move, `go` will take time. The UI's useEngine hook handles async responses via event listeners — no structural change needed.
2. **Stage 18 (Full UI):** All UI components are minimal scaffolding. Stage 18 will replace/extend them. Component boundaries are clean.
3. **Tauri sidecar:** Production bundling requires `tauri.conf.json` `bundle.externalBin` config. Not needed until packaging.

### Unaccounted Concerns

1. **DKW invisible moves:** DKW king instant moves not visible through protocol. UI rendering cache will not reflect these. Accepted limitation.
2. **Turn tracking simplified:** UI tracks turns as simple rotation. Does not account for eliminated players being skipped. Engine handles real turn order.

### Reasoning & Methods

1. **Tauri v2 over Electron/WebSocket:** Rust backend spawns engine directly. No separate server. Smaller binary.
2. **SVG over Canvas:** React component model, DOM event handling, trivial performance for 196 elements.
3. **No state management library:** useState/useReducer sufficient for current scope.
4. **Display-side move application:** Board array is rendering cache only. Engine is the authority.


---

## Post-Audit Addendum — Bugfix Session
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

This addendum documents bugs found during live `tauri dev` testing and subsequent fixes. All issues were discovered during interactive play, not during the original audit (which noted `tauri dev` was not tested end-to-end).

### Bugs Found and Fixed

#### BLOCKING: En Passant False Positive for Blue/Green Pawns

**Severity:** BLOCKING (caused piece to vanish on every Blue/Green pawn forward move)
**Root cause (Section 2.6 — Broken Code):** In `applyMoveToBoard` (useGameState.ts), en passant detection checked only `fileOf(from) !== fileOf(to)`. For Blue and Green pawns, whose forward direction changes the file (not rank), every forward step triggered en passant detection. The captured-square formula `squareFrom(fileOf(to), rankOf(from))` then equaled the destination square itself, removing the just-placed piece.

**Fix:** Changed condition to require both file AND rank to change (a true diagonal):
```typescript
// Before (broken):
if (piece.pieceType === 'Pawn' && fileOf(from) !== fileOf(to) && prev[to] === null)

// After (correct for all 4 orientations):
const isDiagonal = fileOf(from) !== fileOf(to) && rankOf(from) !== rankOf(to);
if (piece.pieceType === 'Pawn' && isDiagonal && prev[to] === null)
```

**Impact:** Display-only (rendering cache). Engine state was never affected. Pattern documented in [[Pattern-EP-Captured-Square-4PC]].

#### WARNING: Castling Display Broken for Blue/Green

**Severity:** WARNING (visual-only — rook not moved on display when Blue/Green castle)
**Root cause (Section 2.6 — Broken Code):** Castling detection in `applyMoveToBoard` checked `Math.abs(fileOf(to) - fileOf(from)) >= 2`, but Blue and Green kings castle by changing rank (not file). The detection never triggered for these players.

**Fix:** Added orientation-aware detection:
```typescript
const isVertical = piece.owner === 'Red' || piece.owner === 'Yellow';
const moveDist = isVertical
  ? fileOf(to) - fileOf(from)
  : rankOf(to) - rankOf(from);
if (Math.abs(moveDist) >= 2) {
  // Orientation-specific rook placement
}
```

#### WARNING: Board Clipped at Bottom (Red's Side Cut Off)

**Severity:** WARNING (usability)
**Root cause:** SVG had fixed `width={684} height={684}` attributes. With CSS `overflow: hidden` on parent containers, shorter viewports clipped the bottom of the board.

**Fix:** Removed fixed SVG dimensions. Added CSS-based responsive sizing with `max-height: calc(100vh - 70px)`, `aspect-ratio: 1`, and `width: 100%; height: 100%` on the SVG.

#### BLOCKING: advancePlayer Returns Wrong Player (React 18 Batching)

**Severity:** BLOCKING (caused semi-auto mode to completely fail)
**Root cause (Section 2.6 — Broken Code):** `advancePlayer` computed the next player inside a `setCurrentPlayer` updater function. With React 18 automatic batching, when multiple setState calls are pending in the same handler, the updater may not run synchronously. `nextPlayer` stayed at its default value `'Red'`, causing every downstream auto-play decision to use the wrong player.

**Fix:** Compute next player directly from `currentPlayerRef.current` (the ref, not the React state):
```typescript
// Before (broken due to React 18 batching):
const advancePlayer = useCallback((): Player => {
  let nextPlayer: Player = 'Red';
  setCurrentPlayer((prev) => {
    const idx = PLAYERS.indexOf(prev);
    nextPlayer = PLAYERS[(idx + 1) % 4];
    return nextPlayer;
  });
  return nextPlayer;
}, []);

// After (ref-based, no React timing dependency):
const advancePlayer = useCallback((): Player => {
  const idx = PLAYERS.indexOf(currentPlayerRef.current);
  const nextPlayer = PLAYERS[(idx + 1) % 4];
  currentPlayerRef.current = nextPlayer;
  setCurrentPlayer(nextPlayer);
  return nextPlayer;
}, []);
```

**Impact:** Semi-auto and full-auto play modes were completely non-functional without this fix. Pattern documented in [[Pattern-React-Ref-Async-State]].

### New Features Added

| Feature | Description | Files |
|---|---|---|
| Three play modes | Manual (click-to-move), Semi-Auto (user picks color, engine plays rest), Full Auto (engine plays all) | useGameState.ts, GameControls.tsx |
| Speed control slider | Adjustable engine move delay 100ms-2000ms | GameControls.tsx |
| Pause/Resume button | Pause auto-play in semi-auto and full-auto modes | useGameState.ts, GameControls.tsx |
| Player color selection | In semi-auto, user picks which color to play as. Locked during active game. | GameControls.tsx |
| Responsive board sizing | Board scales to viewport with aspect-ratio preservation | BoardDisplay.tsx, App.css |

### Deliverables Re-Check

All original deliverables remain intact. New features are additive. The permanent invariant "UI owns ZERO game logic" is still respected — all move validation happens engine-side. The play mode logic (which player to auto-play) is UI scheduling logic, not game logic.

### Issue Disposition

| Issue | Disposition |
|---|---|
| [[Issue-UI-EP-False-Positive]] | Created and resolved this session |
| [[Issue-UI-Castling-Blue-Green]] | Created and resolved this session |
| [[Issue-UI-AdvancePlayer-React-Batching]] | Created and resolved this session |
| [[Issue-DKW-Invisible-Moves-UI]] | Still open (accepted limitation) |

---

## Related

- Stage spec: [[stage_05_basic_ui]]
- Downstream log: [[downstream_log_stage_05]]
- Bugfix session: [[Session-2026-02-20-Stage05-Bugfix]]
