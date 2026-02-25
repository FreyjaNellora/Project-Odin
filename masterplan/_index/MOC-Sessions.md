---
type: moc
tags:
  - type/moc
last_updated: 2026-02-24
---

# Session Journal

Chronological index of build sessions. Each session note captures what was attempted, what worked, what was learned. [[HANDOFF]] gets overwritten each session; these notes preserve the history.

## Sessions

<!-- Add newest sessions at the top -->
- **2026-02-25** [[Session-2026-02-25-UI-Bugfixes]] -- In-search repetition detection (BRS rep_stack), depth 7 default, piece-prefix notation in game log, game log player label bug fixed (React 18 batching — currentPlayerRef read lazily inside updater).
- **2026-02-24** [[Session-2026-02-24-Bugfix-Pause-Resume]] -- UI bugfix: pause/resume race condition sent duplicate `go` commands, causing one player to move twice. Two guards added to `sendGoFromRef` and `togglePause`. 361 engine + 54 Vitest.
- **2026-02-23** [[Session-2026-02-23-Stage08]] -- Stage 8: BRS/Paranoid Hybrid Layer. Board scanner, move classifier, hybrid reply scoring, progressive narrowing (~49% node reduction), eval fix (relative material advantage), tactical suite (23 tests), smoke-play validation. 361 tests.
- **2026-02-23** [[Session-UI-QoL-2026-02-23]] -- UI QoL additions (non-stage): coordinate labels, enriched game log, engine internals panel, communication log, board zoom, layout reorganization. 54 Vitest (no regressions).
- **2026-02-21** [[Session-2026-02-21-Stage07-Bugfix2]] -- Stage 7 bugfix pass 2: UI parser dropped `eliminated Red checkmate` events (Bug C); fixed 3 Stage 7 integration tests broken by protocol nextturn addition. 54 Vitest / 504 engine tests.
- **2026-02-21** [[Session-2026-02-21-Stage07]] -- Stage 7: Plain BRS + Searcher trait. Alpha-beta, iterative deepening, quiescence, aspiration windows, null move, LMR, PV. Engine playable. 302 tests.
- **2026-02-21** [[Session-2026-02-21-Stage06]] -- Stage 6: Bootstrap Eval + Evaluator trait. Material counting, PSTs with 4-player rotation, king safety, multi-player eval. 275 tests.
- **2026-02-20** [[Session-2026-02-20-Stage05-Bugfix]] -- Stage 5 bugfixes: en passant/castling for Blue/Green, responsive board, play modes (Manual/Semi-Auto/Full Auto), speed control, advancePlayer React 18 batching fix.
- **2026-02-20** [[Session-2026-02-20-Stage05]] -- Stage 5: Basic UI Shell. Tauri v2 scaffolding, SVG board renderer, engine subprocess IPC, click-to-move, debug console, game controls. 45 Vitest tests.
- **2026-02-20** [[Session-2026-02-20-Stage04]] -- Stage 4: Odin Protocol. Command parser, response emitter, OdinEngine loop, position setting, random-move go stub. 229 tests (156 unit + 73 integration).
- **2026-02-20** [[Session-2026-02-20-Stage03]] -- Stage 3: GameState, scoring, rules, elimination pipeline, DKW, terrain conversion, game-over detection. 164 tests (108 unit + 56 integration).
- **2026-02-20** [[Session-2026-02-20-Stage02]] -- Stage 2: Move generation, attack queries, make/unmake, perft. Fixed EP representation and two 4PC-specific bugs.
- **2026-02-20** [[Session-2026-02-20-Stage01]] -- Stage 1: Board representation, square indexing, Zobrist hashing, FEN4 serialization.
- **2026-02-19** Stage 0: Project skeleton. _(No session note — predates vault note protocol)_
