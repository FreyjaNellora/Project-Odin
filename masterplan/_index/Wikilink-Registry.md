---
type: moc
tags:
  - type/moc
last_updated: 2026-02-19
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
<!-- Example: | `[[Component-Board]]` | `components/Component-Board.md` | Board representation implementation details | -->

_None yet._

### Connections
<!-- Example: | `[[Connection-Board-to-MoveGen]]` | `connections/Connection-Board-to-MoveGen.md` | How Board feeds into MoveGen | -->

_None yet._

### Sessions
<!-- Example: | `[[Session-2026-02-19-Stage-0-Kickoff]]` | `sessions/Session-2026-02-19-Stage-0-Kickoff.md` | First implementation session | -->

_None yet._

### Issues
<!-- Example: | `[[Issue-Zobrist-Hash-Mismatch]]` | `issues/Issue-Zobrist-Hash-Mismatch.md` | Zobrist hash diverges after castling unmake | -->

_None yet._

### Patterns
<!-- Example: | `[[Pattern-Huginn-Gate-Wiring]]` | `patterns/Pattern-Huginn-Gate-Wiring.md` | How to add a new Huginn observation point | -->

_None yet._
