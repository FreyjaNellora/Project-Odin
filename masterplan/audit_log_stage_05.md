# Audit Log — Stage 05: BasicUI

## Pre-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build` and `cargo build --features huginn`)
- Tests pass: Yes (229 total: 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04)
- Previous downstream flags reviewed: Stages 0-4 (full dependency chain)

### Findings

**From Stage 0 downstream log:**
- Huginn macro and buffer available. No gates wired (deferred). No impact on Stage 5.

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

## Related

- Stage spec: [[stage_05_basic_ui]]
- Downstream log: [[downstream_log_stage_05]]
