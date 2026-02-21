---
type: moc
tags:
  - type/moc
last_updated: 2026-02-21
---

# Wikilink Registry

Single source of truth for all `[[wikilink]]` targets in the vault. Before creating a new link, check this list. Before creating a new file, add its target here. See [[AGENT_CONDUCT]] Section 1.12 for the full wikilink discipline rules.

---

## Core Documents

| Target | File | Purpose |
|---|---|---|
| `[[MASTERPLAN]]` | `MASTERPLAN.md` | Full project specification — stages, architecture, acceptance criteria |
| `[[AGENT_CONDUCT]]` | `AGENT_CONDUCT.md` | Agent behavior rules, audit checklist, Huginn reporting spec |
| `[[4PC_RULES_REFERENCE]]` | `4PC_RULES_REFERENCE.md` | Complete 4-player chess rules per chess.com |
| `[[DECISIONS]]` | `DECISIONS.md` | Architectural decision records (ADRs) |
| `[[STATUS]]` | `STATUS.md` | Current project state — what stage, what's blocked |
| `[[HANDOFF]]` | `HANDOFF.md` | Session continuity — what happened, what's next |
| `[[CLAUDE]]` | `CLAUDE.md` | Vault instructions for AI agents |

## Index / Maps of Content

| Target | File | Purpose |
|---|---|---|
| `[[MOC-Project-Odin]]` | `_index/MOC-Project-Odin.md` | Top-level project navigation |
| `[[MOC-Tier-1-Foundation]]` | `_index/MOC-Tier-1-Foundation.md` | Stages 0-5 hub |
| `[[MOC-Tier-2-Simple-Search]]` | `_index/MOC-Tier-2-Simple-Search.md` | Stages 6-7 hub |
| `[[MOC-Tier-3-Strengthen-Search]]` | `_index/MOC-Tier-3-Strengthen-Search.md` | Stages 8-11 hub |
| `[[MOC-Tier-4-Measurement]]` | `_index/MOC-Tier-4-Measurement.md` | Stages 12-13 hub |
| `[[MOC-Tier-5-Learn]]` | `_index/MOC-Tier-5-Learn.md` | Stages 14-16 hub |
| `[[MOC-Tier-6-Polish]]` | `_index/MOC-Tier-6-Polish.md` | Stages 17-19 hub |
| `[[MOC-Active-Issues]]` | `_index/MOC-Active-Issues.md` | Open issues registry |
| `[[MOC-Sessions]]` | `_index/MOC-Sessions.md` | Session history |
| `[[Wikilink-Registry]]` | `_index/Wikilink-Registry.md` | This file — canonical wikilink index |

## Stage Specs

| Target | File | Purpose |
|---|---|---|
| `[[stage_00_skeleton]]` | `stages/stage_00_skeleton.md` | Stage 0: Project skeleton, Huginn core, CI |
| `[[stage_01_board]]` | `stages/stage_01_board.md` | Stage 1: Board representation, Zobrist hashing |
| `[[stage_02_movegen]]` | `stages/stage_02_movegen.md` | Stage 2: Move generation, perft, legality |
| `[[stage_03_gamestate]]` | `stages/stage_03_gamestate.md` | Stage 3: Game state, rules, elimination, scoring |
| `[[stage_04_protocol]]` | `stages/stage_04_protocol.md` | Stage 4: Odin Protocol (engine-UI communication) |
| `[[stage_05_basic_ui]]` | `stages/stage_05_basic_ui.md` | Stage 5: Basic UI (board display, move input) |
| `[[stage_06_bootstrap_eval]]` | `stages/stage_06_bootstrap_eval.md` | Stage 6: Bootstrap evaluation, Evaluator trait |
| `[[stage_07_plain_brs]]` | `stages/stage_07_plain_brs.md` | Stage 7: Plain BRS search, Searcher trait |
| `[[stage_08_brs_hybrid]]` | `stages/stage_08_brs_hybrid.md` | Stage 8: BRS hybrid scoring, board context |
| `[[stage_09_tt_ordering]]` | `stages/stage_09_tt_ordering.md` | Stage 9: Transposition table, move ordering |
| `[[stage_10_mcts]]` | `stages/stage_10_mcts.md` | Stage 10: MCTS search |
| `[[stage_11_hybrid_integration]]` | `stages/stage_11_hybrid_integration.md` | Stage 11: BRS+MCTS hybrid controller |
| `[[stage_12_self_play]]` | `stages/stage_12_self_play.md` | Stage 12: Self-play framework, SPRT |
| `[[stage_13_time_management]]` | `stages/stage_13_time_management.md` | Stage 13: Time management, panic time |
| `[[stage_14_nnue_design]]` | `stages/stage_14_nnue_design.md` | Stage 14: NNUE architecture, inference |
| `[[stage_15_nnue_training]]` | `stages/stage_15_nnue_training.md` | Stage 15: NNUE training pipeline (Python) |
| `[[stage_16_nnue_integration]]` | `stages/stage_16_nnue_integration.md` | Stage 16: NNUE replaces bootstrap eval |
| `[[stage_17_variants]]` | `stages/stage_17_variants.md` | Stage 17: Variant tuning (DKW, terrain, 960) |
| `[[stage_18_full_ui]]` | `stages/stage_18_full_ui.md` | Stage 18: Full UI (arrows, dashboard, analysis) |
| `[[stage_19_polish]]` | `stages/stage_19_polish.md` | Stage 19: Optimization, profiling, hardening |

## Audit Logs

| Target | File | Purpose |
|---|---|---|
| `[[audit_log_stage_00]]` | `audit_logs/audit_log_stage_00.md` | Stage 0 audit findings |
| `[[audit_log_stage_01]]` | `audit_logs/audit_log_stage_01.md` | Stage 1 audit findings |
| `[[audit_log_stage_02]]` | `audit_logs/audit_log_stage_02.md` | Stage 2 audit findings |
| `[[audit_log_stage_03]]` | `audit_logs/audit_log_stage_03.md` | Stage 3 audit findings |
| `[[audit_log_stage_04]]` | `audit_logs/audit_log_stage_04.md` | Stage 4 audit findings |
| `[[audit_log_stage_05]]` | `audit_logs/audit_log_stage_05.md` | Stage 5 audit findings |
| `[[audit_log_stage_06]]` | `audit_logs/audit_log_stage_06.md` | Stage 6 audit findings |
| `[[audit_log_stage_07]]` | `audit_logs/audit_log_stage_07.md` | Stage 7 audit findings |
| `[[audit_log_stage_08]]` | `audit_logs/audit_log_stage_08.md` | Stage 8 audit findings |
| `[[audit_log_stage_09]]` | `audit_logs/audit_log_stage_09.md` | Stage 9 audit findings |
| `[[audit_log_stage_10]]` | `audit_logs/audit_log_stage_10.md` | Stage 10 audit findings |
| `[[audit_log_stage_11]]` | `audit_logs/audit_log_stage_11.md` | Stage 11 audit findings |
| `[[audit_log_stage_12]]` | `audit_logs/audit_log_stage_12.md` | Stage 12 audit findings |
| `[[audit_log_stage_13]]` | `audit_logs/audit_log_stage_13.md` | Stage 13 audit findings |
| `[[audit_log_stage_14]]` | `audit_logs/audit_log_stage_14.md` | Stage 14 audit findings |
| `[[audit_log_stage_15]]` | `audit_logs/audit_log_stage_15.md` | Stage 15 audit findings |
| `[[audit_log_stage_16]]` | `audit_logs/audit_log_stage_16.md` | Stage 16 audit findings |
| `[[audit_log_stage_17]]` | `audit_logs/audit_log_stage_17.md` | Stage 17 audit findings |
| `[[audit_log_stage_18]]` | `audit_logs/audit_log_stage_18.md` | Stage 18 audit findings |
| `[[audit_log_stage_19]]` | `audit_logs/audit_log_stage_19.md` | Stage 19 audit findings |

## Downstream Logs

| Target | File | Purpose |
|---|---|---|
| `[[downstream_log_stage_00]]` | `downstream_logs/downstream_log_stage_00.md` | Stage 0 API contracts and notes for future stages |
| `[[downstream_log_stage_01]]` | `downstream_logs/downstream_log_stage_01.md` | Stage 1 API contracts and notes for future stages |
| `[[downstream_log_stage_02]]` | `downstream_logs/downstream_log_stage_02.md` | Stage 2 API contracts and notes for future stages |
| `[[downstream_log_stage_03]]` | `downstream_logs/downstream_log_stage_03.md` | Stage 3 API contracts and notes for future stages |
| `[[downstream_log_stage_04]]` | `downstream_logs/downstream_log_stage_04.md` | Stage 4 API contracts and notes for future stages |
| `[[downstream_log_stage_05]]` | `downstream_logs/downstream_log_stage_05.md` | Stage 5 API contracts and notes for future stages |
| `[[downstream_log_stage_06]]` | `downstream_logs/downstream_log_stage_06.md` | Stage 6 API contracts and notes for future stages |
| `[[downstream_log_stage_07]]` | `downstream_logs/downstream_log_stage_07.md` | Stage 7 API contracts and notes for future stages |
| `[[downstream_log_stage_08]]` | `downstream_logs/downstream_log_stage_08.md` | Stage 8 API contracts and notes for future stages |
| `[[downstream_log_stage_09]]` | `downstream_logs/downstream_log_stage_09.md` | Stage 9 API contracts and notes for future stages |
| `[[downstream_log_stage_10]]` | `downstream_logs/downstream_log_stage_10.md` | Stage 10 API contracts and notes for future stages |
| `[[downstream_log_stage_11]]` | `downstream_logs/downstream_log_stage_11.md` | Stage 11 API contracts and notes for future stages |
| `[[downstream_log_stage_12]]` | `downstream_logs/downstream_log_stage_12.md` | Stage 12 API contracts and notes for future stages |
| `[[downstream_log_stage_13]]` | `downstream_logs/downstream_log_stage_13.md` | Stage 13 API contracts and notes for future stages |
| `[[downstream_log_stage_14]]` | `downstream_logs/downstream_log_stage_14.md` | Stage 14 API contracts and notes for future stages |
| `[[downstream_log_stage_15]]` | `downstream_logs/downstream_log_stage_15.md` | Stage 15 API contracts and notes for future stages |
| `[[downstream_log_stage_16]]` | `downstream_logs/downstream_log_stage_16.md` | Stage 16 API contracts and notes for future stages |
| `[[downstream_log_stage_17]]` | `downstream_logs/downstream_log_stage_17.md` | Stage 17 API contracts and notes for future stages |
| `[[downstream_log_stage_18]]` | `downstream_logs/downstream_log_stage_18.md` | Stage 18 API contracts and notes for future stages |
| `[[downstream_log_stage_19]]` | `downstream_logs/downstream_log_stage_19.md` | Stage 19 API contracts and notes for future stages |

## Agent-Created Notes (populated during implementation)

These sections grow as agents create notes during development. Add entries here immediately when creating new files.

### Components

| Target | File | Purpose |
|---|---|---|
| `[[Component-Board]]` | `components/Component-Board.md` | Board representation: 14x14 array, piece lists, Zobrist, FEN4 |
| `[[Component-MoveGen]]` | `components/Component-MoveGen.md` | Move generation, attack queries, make/unmake, perft |
| `[[Component-GameState]]` | `components/Component-GameState.md` | Game state, scoring, rules, elimination, DKW, terrain, game-over |
| `[[Component-Protocol]]` | `components/Component-Protocol.md` | Odin Protocol: command parsing, response formatting, engine loop |
| `[[Component-BasicUI]]` | `components/Component-BasicUI.md` | Basic UI Shell: Tauri v2, SVG board, engine IPC, debug console |
| `[[Component-Eval]]` | `components/Component-Eval.md` | Bootstrap Evaluator: Evaluator trait, material, PST, king safety, multi-player eval |

### Connections

| Target | File | Purpose |
|---|---|---|
| `[[Connection-Board-to-MoveGen]]` | `connections/Connection-Board-to-MoveGen.md` | How Board feeds position state into MoveGen |
| `[[Connection-MoveGen-to-GameState]]` | `connections/Connection-MoveGen-to-GameState.md` | How MoveGen provides legal moves, attack queries, and make_move to GameState |
| `[[Connection-Board-to-GameState]]` | `connections/Connection-Board-to-GameState.md` | How Board is wrapped and accessed by GameState |
| `[[Connection-GameState-to-Protocol]]` | `connections/Connection-GameState-to-Protocol.md` | How Protocol owns and drives GameState |
| `[[Connection-Protocol-to-UI]]` | `connections/Connection-Protocol-to-UI.md` | How UI communicates with engine via Tauri IPC |
| `[[Connection-GameState-to-Eval]]` | `connections/Connection-GameState-to-Eval.md` | How Eval reads GameState for position scoring |
| `[[Connection-Eval-to-Search]]` | `connections/Connection-Eval-to-Search.md` | How Search calls Eval through the Evaluator trait |

### Sessions

| Target | File | Purpose |
|---|---|---|
| `[[Session-2026-02-20-Stage01]]` | `sessions/session-2026-02-20-stage01.md` | Stage 1 implementation session |
| `[[Session-2026-02-20-Stage02]]` | `sessions/session-2026-02-20-stage02.md` | Stage 2 implementation session |
| `[[Session-2026-02-20-Stage03]]` | `sessions/session-2026-02-20-stage03.md` | Stage 3 implementation session |
| `[[Session-2026-02-20-Stage04]]` | `sessions/Session-2026-02-20-Stage04.md` | Stage 4 implementation session |
| `[[Session-2026-02-20-Stage05]]` | `sessions/Session-2026-02-20-Stage05.md` | Stage 5 implementation session |
| `[[Session-2026-02-20-Stage05-Bugfix]]` | `sessions/Session-2026-02-20-Stage05-Bugfix.md` | Stage 5 bugfix session: en passant, castling, play modes, React batching |
| `[[Session-2026-02-21-Stage06]]` | `sessions/Session-2026-02-21-Stage06.md` | Stage 6: Bootstrap Eval + Evaluator trait implementation |

### Issues

| Target | File | Purpose |
|---|---|---|
| `[[Issue-EP-Representation-4PC]]` | `issues/Issue-EP-Representation-4PC.md` | En passant file→square fix for 4PC (resolved) |
| `[[Issue-Perft-Values-Unverified]]` | `issues/Issue-Perft-Values-Unverified.md` | Perft values lack external verification (open) |
| `[[Issue-Huginn-Gates-Unwired]]` | `issues/Issue-Huginn-Gates-Unwired.md` | Stages 1-6 Huginn gates not wired (open) |
| `[[Issue-DKW-Halfmove-Clock]]` | `issues/Issue-DKW-Halfmove-Clock.md` | DKW instant moves increment halfmove clock (open) |
| `[[Issue-DKW-Invisible-Moves-UI]]` | `issues/Issue-DKW-Invisible-Moves-UI.md` | DKW king instant moves not visible in UI (open) |
| `[[Issue-UI-EP-False-Positive]]` | `issues/Issue-UI-EP-False-Positive.md` | En passant false positive for Blue/Green UI display (resolved) |
| `[[Issue-UI-Castling-Blue-Green]]` | `issues/Issue-UI-Castling-Blue-Green.md` | Castling display broken for Blue/Green UI (resolved) |
| `[[Issue-UI-AdvancePlayer-React-Batching]]` | `issues/Issue-UI-AdvancePlayer-React-Batching.md` | advancePlayer wrong player from React 18 batching (resolved) |
| `[[Issue-Vec-Clone-Cost-Pre-MCTS]]` | `issues/Issue-Vec-Clone-Cost-Pre-MCTS.md` | Vec clone cost in Board/GameState — retrofit before Stage 10 (open) |

### Patterns

| Target | File | Purpose |
|---|---|---|
| `[[Pattern-Pawn-Reverse-Lookup]]` | `patterns/Pattern-Pawn-Reverse-Lookup.md` | Use (player+2)%4 for reverse pawn attack detection in 4PC |
| `[[Pattern-EP-Captured-Square-4PC]]` | `patterns/Pattern-EP-Captured-Square-4PC.md` | Use prev_player's forward direction for EP captured pawn location |
| `[[Pattern-Terrain-Awareness]]` | `patterns/Pattern-Terrain-Awareness.md` | Terrain pieces block movement and don't give check at MoveGen level |
| `[[Pattern-DKW-Instant-Moves]]` | `patterns/Pattern-DKW-Instant-Moves.md` | DKW king moves happen instantly between turns via side_to_move swap |
| `[[Pattern-React-Ref-Async-State]]` | `patterns/Pattern-React-Ref-Async-State.md` | Use refs alongside React state for synchronous reads in async chains |
