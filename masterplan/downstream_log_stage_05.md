# Downstream Log — Stage 05: BasicUI

## Notes for Future Stages

### Must-Know

1. **UI owns ZERO game logic.** No legal move generation, no check detection, no move validation. All moves sent to engine for validation via `position startpos moves <all>` command. This is a permanent invariant.
2. **The UI maintains a local rendering cache.** The `board` array in `useGameState.ts` is a display-side copy, NOT the engine's internal state. It can desync from the engine (especially during DKW). "New Game" resets it.
3. **Engine subprocess is managed by Tauri Rust backend.** `engine.rs` handles spawn/kill/send. Frontend communicates via Tauri IPC (`invoke`) and events (`engine-output`, `engine-exit`).
4. **Move flow requires re-sending full position.** Before every `go`, UI sends `position startpos moves <complete move list>`. This is because the engine does NOT apply `bestmove` to its own state (see [[downstream_log_stage_04]]).
5. **Error recovery re-sends previous valid position.** If engine returns `info string Error:`, UI re-sends `position startpos moves <moveList without failed move>` to restore engine state (since error clears `game_state = None`).

### API Contracts

**Tauri IPC Commands (Rust → Frontend):**

| Command | Args | Returns | Notes |
|---|---|---|---|
| `spawn_engine` | none | `Result<String, String>` | Starts engine process, begins stdout reader |
| `send_command` | `cmd: String` | `Result<(), String>` | Writes line to engine stdin |
| `kill_engine` | none | `Result<(), String>` | Kills engine process |

**Tauri Events (Backend → Frontend):**

| Event | Payload | Notes |
|---|---|---|
| `engine-output` | `String` (single stdout line) | Emitted for each engine stdout line |
| `engine-exit` | `String` (exit info) | Emitted when engine process exits |

**TypeScript Types:**

| Type | Location | Purpose |
|---|---|---|
| `Player` | `types/board.ts` | `'Red' \| 'Blue' \| 'Yellow' \| 'Green'` |
| `PieceType` | `types/board.ts` | All piece types including PromotedQueen |
| `Piece` | `types/board.ts` | `{ pieceType, owner }` |
| `EngineMessage` | `types/protocol.ts` | Union of all parsed engine output types |
| `InfoData` | `types/protocol.ts` | Parsed info fields: depth, scoreCp, values, nodes, nps, pv, etc. |

**Utility Functions:**

| Function | Location | Notes |
|---|---|---|
| `isValidSquare(index)` | `lib/board-constants.ts` | Mirrors engine's square.rs |
| `squareName(sq)` / `parseSquare(name)` | `lib/board-constants.ts` | Coordinate conversion |
| `startingPosition()` | `lib/board-constants.ts` | 196-element array matching engine |
| `parseEngineOutput(line)` | `lib/protocol-parser.ts` | Parse single engine stdout line |

### Known Limitations

1. **DKW invisible moves:** DKW king instant random moves not visible through protocol. UI rendering cache will not update.
2. **Turn tracking is simple rotation:** Red→Blue→Yellow→Green regardless of eliminations.
3. **No undo/takeback.** "New Game" is the only reset.
4. **No move history display.** Move list stored but not rendered.
5. **No legal move highlighting.** Would require game logic in UI.
6. **Auto-promote to queen.** No promotion dialog. Stage 18 concern.
7. **`tauri dev` not verified end-to-end.** Individual compilations verified.
8. **Engine path hardcoded to dev build.** Sidecar bundling is a production concern.

### Performance Baselines

| Metric | Value | Notes |
|---|---|---|
| Engine test count (no huginn) | 229 | Unchanged — Stage 5 adds no engine tests |
| Vitest test count | 45 | 29 board-constants + 16 protocol-parser |
| Tauri backend compile (fresh) | ~11s | Debug profile |
| TypeScript compile | <1s | `tsc --noEmit` |

### Open Questions

1. **Sidecar bundling:** When should the engine binary be bundled as a Tauri sidecar? Not needed until packaging for distribution.
2. **DKW rendering:** Should Stage 18 add a mechanism to query engine state after moves to detect DKW king relocations?
3. **Turn tracking accuracy:** Should the UI query the engine for the current side-to-move rather than tracking locally?

### Reasoning

1. **Tauri v2 over Electron:** Rust-native, spawns engine directly, smaller binary, no Node.js runtime.
2. **SVG over Canvas:** React component model, DOM events, trivial performance for 160 elements.
3. **No state management library:** useState/useReducer sufficient. Redux/Zustand over-engineering.
4. **Display-side move application:** Rendering cache only. Engine is the authority.


---

## Related

- Stage spec: [[stage_05_basic_ui]]
- Audit log: [[audit_log_stage_05]]
