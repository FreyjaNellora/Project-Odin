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
- `useGameState`: Hook managing board rendering cache, move list, turn rotation, click-to-move flow.
- `BoardDisplay/BoardSquare/PieceIcon`: SVG board renderer, 46px squares, file/rank labels.
- `DebugConsole`: Scrollable log with color coding, parsed info summary, manual command input.
- `GameControls`: Turn indicator, scores, New Game / Engine Move buttons.
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

## Performance Notes

SVG board renders 160 rect + text elements. Trivial for modern browsers. No performance concerns at this stage.

## Known Issues

- [[Issue-DKW-Invisible-Moves-UI]] — DKW king positions not reflected in UI
- `tauri dev` not tested end-to-end (awaits graphical testing)

## Build History

- [[Session-2026-02-20-Stage05]] — Initial implementation
