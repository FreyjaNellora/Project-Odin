---
type: component
stage_introduced: 5
tags:
  - stage/05
  - tier/foundation
status: implemented
last_updated: 2026-02-20
---

# Component: Basic UI Shell

## Purpose

Desktop application shell for visualizing the 4-player chess board, making moves, and debugging engine communication. Built with Tauri v2 (Rust backend, React frontend). This is functional scaffolding — visual polish comes in Stage 18.

## Key Types

- `Player`: `'Red' | 'Blue' | 'Yellow' | 'Green'` — player identifiers
- `PieceType`: 7 piece types including PromotedQueen
- `Piece`: `{ pieceType, owner }` — piece on board
- `EngineMessage`: Union type for all parsed engine output
- `InfoData`: Parsed search info fields (depth, score, nodes, etc.)

## Public API

**Tauri IPC Commands:**
- `spawn_engine()` → starts engine subprocess
- `send_command(cmd: String)` → writes to engine stdin
- `kill_engine()` → kills engine process

**Events (backend → frontend):**
- `engine-output` — single engine stdout line
- `engine-exit` — engine process exit

**Utility Functions:**
- `isValidSquare(index)` — square validity check
- `squareName(sq)` / `parseSquare(name)` — coordinate helpers
- `startingPosition()` — 196-element board array
- `parseEngineOutput(line)` — protocol parser

## Internal Design

**Rust Backend (src-tauri/):**
- `engine.rs`: EngineManager struct spawns child process with piped stdio. Stdout reader thread emits events. Drop impl cleans up.
- `lib.rs`: Tauri app builder with IPC command handlers and AppState (Mutex<EngineManager>).

**React Frontend (src/):**
- `useEngine`: Hook managing engine lifecycle, IPC, event listeners, message routing.
- `useGameState`: Hook managing board rendering cache, move list, turn rotation, click-to-move flow, play modes (manual/semi-auto/full-auto), auto-play chaining with ref-based state.
- `BoardDisplay/BoardSquare/PieceIcon`: SVG board renderer, 46px squares, file/rank labels. Responsive sizing via CSS (no fixed dimensions).
- `DebugConsole`: Scrollable log with color coding, parsed info summary, manual command input.
- `GameControls`: Turn indicator, scores, play mode selector (Manual/Semi-Auto/Full Auto), player color picker (semi-auto), speed slider (100-2000ms), pause/resume button, New Game / Engine Move buttons.
- `StatusBar`: Engine name and connection status.

## Connections
- Depends on: [[Component-Protocol]] (stdout/stdin communication format)
- Depended on by: [[stage_18_full_ui]] (will extend/replace these scaffolding components)
- Communicates via: [[Connection-Protocol-to-UI]]

## Huginn Gates

None. Stage 5 is UI-only; Huginn is engine-side.

## Gotchas

1. **Engine does NOT apply bestmove to its own state.** UI must re-send full `position startpos moves <all>` before every `go`.
2. **Error in `position` command clears engine state.** UI must re-send previous valid position for error recovery.
3. **DKW invisible moves.** UI rendering cache does not reflect DKW king instant moves.
4. **Turn tracking is simple rotation.** Does not account for eliminated players being skipped.
5. **En passant detection must check BOTH file AND rank change.** Blue/Green forward moves change file, not rank. Only a true diagonal (both change) is en passant.
6. **Castling detection is orientation-aware.** Red/Yellow castle horizontally (file ≥ 2). Blue/Green castle vertically (rank ≥ 2).
7. **advancePlayer uses ref, not React state updater.** React 18 batching can delay state updater execution. The ref `currentPlayerRef` is the source of truth for immediate reads. See [[Pattern-React-Ref-Async-State]].
8. **Player color locked during active game.** In semi-auto, player selector is disabled once moveList is non-empty. Prevents mid-game switching bugs.

## Performance Notes

SVG board renders 160 rect + text elements. Trivial for modern browsers. No performance concerns at this stage.

## Known Issues

- [[Issue-DKW-Invisible-Moves-UI]] — DKW king positions not reflected in UI

## Build History

- [[Session-2026-02-20-Stage05]] — Initial implementation
- [[Session-2026-02-20-Stage05-Bugfix]] — Fixed en passant/castling for Blue/Green, responsive board, play modes, semi-auto advancePlayer fix
