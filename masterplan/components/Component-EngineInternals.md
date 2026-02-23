---
type: component
tags:
  - type/component
  - scope/ui
created: 2026-02-23
---

# Component: EngineInternals

Collapsible panel showing engine-specific search data.

## Location

`odin-ui/src/components/EngineInternals.tsx` + `odin-ui/src/styles/EngineInternals.css`

## Purpose

Displays engine internals that are distinct from the main analysis summary: search phase, BRS/MCTS-specific stats, and per-player evaluation values.

## Data Source

All data comes from `latestInfo: InfoData` (already parsed from engine `info` lines):

| Field | Source | Display |
|---|---|---|
| `phase` | `info ... phase brs` | Badge: "BRS" or "MCTS" |
| `brsSurviving` | `info ... brs_surviving 5` | "5 candidates" |
| `mctsSims` | `info ... mcts_sims 1000` | "1,000" |
| `seldepth` | `info ... seldepth 12` | "12" |
| `values` | `info ... v1 4300 v2 4300 v3 4300 v4 4300` | 4-column grid with player colors |

## Collapsible

Header toggles visibility. Default: expanded. Arrow indicator: `▶` (collapsed) / `▼` (expanded).

## Known Issue

Some data overlaps with `AnalysisPanel` (depth, seldepth). A future dedup pass should ensure AnalysisPanel owns the search summary and EngineInternals owns only engine-specific fields.

## Related

- [[Component-BasicUI]] — parent UI shell
- [[Session-UI-QoL-2026-02-23]] — creation session
