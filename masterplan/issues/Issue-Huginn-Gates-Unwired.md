---
type: issue
date_opened: 2026-02-20
last_updated: 2026-02-21
date_resolved:
stage: 1
severity: note
status: open
tags:
  - stage/01
  - stage/02
  - stage/03
  - stage/04
  - stage/06
  - area/huginn
  - severity/note
---

# Huginn Observation Gates Not Wired for Stages 1-6

## Description

The MASTERPLAN specifies Huginn observation gates for Stage 1 (board_mutation, zobrist_update, fen4_roundtrip, piece_list_sync), Stage 2 (move_generation, make_unmake, legality_filter, perft), Stage 3 (scoring_event, elimination_event, dkw_trigger, terrain_conversion, game_over_event, turn_advance, claim_win_attempt), Stage 4 (command_receive, response_send, position_set, search_request), and Stage 6 (eval_call, eval_comparison). None of these are implemented as `huginn_observe!` calls.

The root cause is a chicken-and-egg problem: the `HuginnBuffer` has no global instance ([[downstream_log_stage_00]] known limitation #3), and Board/MoveGen/GameState/Protocol/Eval methods would need to accept a buffer parameter to fire observations. This would pollute the API signatures across the entire codebase.

## Affected Components

- [[stage_00_skeleton]] -- Huginn buffer design
- [[stage_01_board]] -- 4 gates specified but unwired
- [[stage_02_movegen]] -- 4 gates specified but unwired
- [[stage_03_gamestate]] -- 7 gates specified but unwired
- [[stage_04_protocol]] -- 4 gates specified but unwired
- [[stage_06_bootstrap_eval]] -- 2 gates specified but unwired (eval_call, eval_comparison)
- [[audit_log_stage_01]] -- future conflict analysis #3
- [[audit_log_stage_02]] -- dead code note

## Workaround

Stage 1 implemented `verify_zobrist()` and `verify_piece_lists()` debug methods as functional substitutes. Stage 2 tests verify Zobrist round-trips and piece list sync directly. These serve the same purpose as the Huginn gates during testing.

## Resolution

<!-- Wire gates when Stage 4 (Odin Protocol) establishes the engine runtime and buffer plumbing, or when a global buffer pattern is adopted. -->

## Related

- [[downstream_log_stage_00]] -- known limitation #3 (no global buffer)
- [[audit_log_stage_01]] -- future conflict analysis #3
- [[audit_log_stage_02]] -- dead code note
