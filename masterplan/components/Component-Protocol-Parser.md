---
type: component
stage_introduced: 5
tags:
  - stage/05
  - tier/foundation
status: implemented
last_updated: 2026-02-21
---

# Component: Protocol Parser (UI-side)

## Purpose

TypeScript module that parses raw engine stdout lines into structured UI events. Lives in the UI codebase (`odin-ui/src/protocol-parser.ts`). Translates Odin Protocol text format into typed objects the React frontend can consume. Critical bridge between engine output and UI state — if parsing fails silently, the UI loses sync with the engine.

## Key Types

- `EngineMessage`: Union type of all parsed message variants
- `InfoData`: Structured search info (depth, score, nodes, NPS, time, PV, v1-v4, phase)
- `Player`: `'Red' | 'Blue' | 'Yellow' | 'Green'` — validated color strings

## Public API

- `parseEngineOutput(line: string) -> EngineMessage | null` — Main entry point. Returns null for unrecognized lines.

## Internal Design

Line-prefix matching: checks `startsWith` for known prefixes (`bestmove`, `info string eliminated`, `info string nextturn`, `info string gameover`, `info depth`, `readyok`, etc.).

**Critical parsing rule (post-Stage 7 bugfix):** The `eliminated` message comes in two formats:
1. `info string eliminated <Color>` — normal path (inline detection)
2. `info string eliminated <Color> <reason>` — safety-net path (handle_no_legal_moves)

The parser must extract only the **first word** after `"eliminated"` as the color. Trailing words (reason) are ignored for color validation. Failing to do this causes the elimination event to be silently dropped — the UI never learns the player was eliminated, creating a desync.

## Connections
- Depends on: [[Component-Protocol]] (defines the message format the engine emits)
- Depended on by: [[Component-BasicUI]] (useGameState consumes parsed messages)
- Communicates via: [[Connection-Protocol-to-UI]]

## Huginn Gates

None. UI-side component; Huginn is engine-side only.

## Gotchas

1. **First-token extraction for `eliminated`.** Parser must split on whitespace and take only the first token as the color. See two-format rule above. This was Bug C in the Stage 7 checkmate bugfix.
2. **Silent null return.** Unrecognized lines return null and are ignored. If a new protocol message type is added to the engine but the parser isn't updated, the message vanishes silently. No error, no warning.
3. **`nextturn` must be idempotent in the UI.** Both the normal bestmove path and the handle_no_legal_moves path emit nextturn. The UI may receive duplicate nextturn events for the same color — this must not break state.

## Performance Notes

String parsing. Trivial cost. No concerns.

## Known Issues

None currently open.

## Build History

- [[Session-2026-02-20-Stage05]] — Initial implementation as part of Basic UI
- [[Session-2026-02-21-Stage07-Bugfix2]] — Bug C fix: first-token extraction for eliminated messages
