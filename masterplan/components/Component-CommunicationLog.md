---
type: component
tags:
  - type/component
  - scope/ui
created: 2026-02-23
---

# Component: CommunicationLog

Raw engine protocol message log with manual command input.

## Location

`odin-ui/src/components/CommunicationLog.tsx` + `odin-ui/src/styles/CommunicationLog.css`

## Purpose

Shows the raw engine stdout lines (the protocol-level communication) and provides a text input for sending manual commands to the engine. Split out from the original `DebugConsole` component.

## Data Source

- `lines: string[]` — raw log from `useEngine.ts` (`engine.rawLog`)
- `onSendCommand: (cmd: string) => void` — sends command to engine via Tauri IPC

## Line Classification (CSS colors)

| Pattern | Class | Color |
|---|---|---|
| `info string Error:` | `log-error` | `#ff5555` (red) |
| `bestmove` | `log-bestmove` | `#50fa7b` (green) |
| `readyok` / `odinok` | `log-routine` | `#666` (dim gray) |
| `info` | `log-info` | `#ccc` (light gray) |
| `id ` | `log-routine` | `#666` (dim gray) |
| default | `log-default` | `#999` (gray) |

## Collapsible

Header toggles visibility. Default: expanded. Useful for hiding the noisy raw log when not debugging.

## Auto-Scroll

Uses a `scrollRef` to auto-scroll to the bottom on new lines.

## Tracing Integration Opportunity

Engine-side `tracing` spans/events could emit additional debug data visible here. Not yet implemented — noted for future sessions.

## Related

- [[Component-BasicUI]] — parent UI shell
- [[Component-Protocol-Parser]] — parses the same lines into structured messages
- [[Session-UI-QoL-2026-02-23]] — creation session
