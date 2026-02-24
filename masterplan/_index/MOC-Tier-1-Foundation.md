---
type: moc
tags:
  - type/moc
  - tier/foundation
last_updated: 2026-02-19
---

# Tier 1: Foundation (Stages 0-5)

The base layer everything else is built on.

## Stage Specs (in [[MASTERPLAN]] Section 4)

| Stage | Spec | Audit Log | Downstream Log |
|---|---|---|---|
| 0 -- Project Skeleton | [[stage_00_skeleton]] | [[audit_log_stage_00]] | [[downstream_log_stage_00]] |
| 1 -- Board Representation | [[stage_01_board]] | [[audit_log_stage_01]] | [[downstream_log_stage_01]] |
| 2 -- Move Generation | [[stage_02_movegen]] | [[audit_log_stage_02]] | [[downstream_log_stage_02]] |
| 3 -- Game State & Rules | [[stage_03_gamestate]] | [[audit_log_stage_03]] | [[downstream_log_stage_03]] |
| 4 -- Odin Protocol | [[stage_04_protocol]] | [[audit_log_stage_04]] | [[downstream_log_stage_04]] |
| 5 -- Basic UI Shell | [[stage_05_basic_ui]] | [[audit_log_stage_05]] | [[downstream_log_stage_05]] |

## Key Decisions

- [[DECISIONS]] ADR-001: Array-first board with clean abstraction boundary
- [[DECISIONS]] ADR-007: Huginn (superseded by ADR-015 -- replaced with `tracing` crate)

## Invariants Established

| Invariant | Stage | Detail in [[MASTERPLAN]] Section 4.1 |
|---|---|---|
| Prior-stage tests never deleted | 0 | Tests from earlier stages never removed |
| Board tests pass, FEN4 round-trips | 1 | Board representation correctness |
| Perft values are forever | 2 | Once established, never change |
| Zobrist make/unmake round-trip | 2 | Exact hash restoration |
| Attack query API is the board boundary | 2 | Nothing above Stage 2 reads board.squares[] |
| Game playouts complete without crashes | 3 | 1000+ random games complete |
| Protocol round-trip works | 4 | position + go -> legal bestmove |
| UI owns zero game logic | 5 | UI never validates moves or evaluates |

## Post-Stage UI Components (added outside stage pipeline)

- [[Component-GameLog]] — enriched move history with per-move search info
- [[Component-EngineInternals]] — collapsible engine data panel (phase, BRS/MCTS, per-player values)
- [[Component-CommunicationLog]] — raw protocol log + command input (split from DebugConsole)
- See [[Session-UI-QoL-2026-02-23]]

## Dependency Chain

```
Stage 0 -> 1 -> 2 -> 3 -> 4 -> 5
                          \-> Stage 6 (can run parallel with 5)
```
