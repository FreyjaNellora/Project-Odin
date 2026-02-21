---
type: connection
tags:
  - stage/05
  - stage/04
last_updated: 2026-02-20
---

# Connection: Protocol ↔ UI

## What Connects

- [[Component-Protocol]] (engine-side, Stage 4)
- [[Component-BasicUI]] (UI-side, Stage 5)

## How They Communicate

The Tauri Rust backend spawns the engine as a child process and bridges stdin/stdout to the React frontend via IPC.

```
React Frontend  ←→  Tauri IPC  ←→  Rust Backend  ←→  Engine stdin/stdout
```

**Frontend → Engine:** `invoke('send_command', { cmd })` → Rust writes line to engine stdin.
**Engine → Frontend:** Engine writes to stdout → Rust reader thread emits `engine-output` event → React listener receives line → `parseEngineOutput()` converts to typed `EngineMessage`.

## Contract

1. **Protocol format:** All communication uses the Odin Protocol text format defined in [[downstream_log_stage_04]]. One command/response per line.
2. **Position re-sending:** UI must send `position startpos moves <all>` before each `go` because the engine does not apply `bestmove` to its own state.
3. **Error recovery:** On `info string Error:`, engine clears its `game_state`. UI must re-send valid position to restore state.
4. **Event-driven responses:** Engine responses arrive asynchronously via `engine-output` events. The `useEngine` hook routes messages to registered callbacks (e.g., `useGameState.handleEngineMessage`).
5. **Handshake:** On connect, UI sends `odin` then `isready`. Waits for `odinok` and `readyok` before enabling interaction.

## Evolution

- **Stage 7 (BRS Search):** `go` will take time instead of returning instantly. UI already handles async responses. May need to handle `stop` command if user cancels.
- **Stage 18 (Full UI):** More sophisticated response handling (analysis mode, move arrows, evaluation graphs). Same underlying IPC mechanism.
- **Production bundling:** Engine binary will become a Tauri sidecar instead of a hardcoded dev path.
