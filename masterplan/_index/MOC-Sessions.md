---
type: moc
tags:
  - type/moc
last_updated: 2026-03-06
---

# Session Journal

Chronological index of build sessions. Each session note captures what was attempted, what worked, what was learned. [[HANDOFF]] gets overwritten each session; these notes preserve the history.

## Sessions

<!-- Add newest sessions at the top -->
- **2026-03-06** [[Session-2026-03-06-Stage-20-Gen0-Complete]] -- Stage 20 complete: datagen (40K samples), Kaggle GPU training, weights loaded, i32 overflow fixed, T13 passes. Pipeline proven end-to-end.
- **2026-03-05** [[Session-2026-03-05-Stage-20-Entry]] -- Stage 20 entry: formal spec written in MASTERPLAN, pre-audit complete, datagen launched (1000 games depth 4). Kaggle account set up for GPU training.
- **2026-02-27** [[Session-2026-02-27-Stage10-Cleanup-Tag]] -- Stage 10 cleanup: git push (18 commits), username anonymization, audit log + downstream log filled, tagged `stage-10-complete` / `v1.10`. Game analysis vs baselines.
- **2026-02-27** [[Session-2026-02-27-Stage10-MCTS]] -- Stage 10: Gumbel MCTS implementation. SplitMix64 PRNG, MctsNode, priors, Gumbel+Top-k, Sequential Halving, PUCT, MaxN backprop, progressive widening, SimConfig, Searcher trait. 1000 sims in 124ms release. AC1-AC8 pass. 440 tests.
- **2026-02-27** [[Session-2026-02-27-Observer-Baselines-Stage10Prep]] -- Observer infrastructure (AGENT_CONDUCT 1.18, LogFile toggle), human baselines (6 games, Elo 1954-3438), depth-8 diagnostic (~2100-2300 Elo), Stage 10 Claude.T prompt written.
- **2026-02-27** [[Session-2026-02-27-PreStage10-Cleanup]] -- Pre-Stage-10 cleanup: audit fixes (W1/W2/N1), pawn-push eval mitigations (dev bonuses, pawn gate, king displacement), Vec clone retrofit (Arc position_history, fixed-size piece_lists). Issue-Vec-Clone-Cost resolved. 408 tests.
- **2026-02-27** [[Session-2026-02-27-BRS-ScoreCap-PawnStructure]] -- BRS score cap (9999 display clamp), connected pawn bonus (+8cp), development bonus, depth 8 default, false mate early-termination gate. New issue: pawn-push preference + king walk.
- **2026-02-27** [[Session-2026-02-27-Multi-Perspective]] -- Multi-perspective opponent modeling: 3-term blend (paranoid + BRS + anti-leader), dynamic context-driven weights, 7 new tests. ENGINE_VERSION v0.5.0.
- **2026-02-27** [[Session-2026-02-27-Game-Analysis-Fixes]] -- Self-play game analysis: likelihood tuning (0.7→0.5), TT player-awareness (root_player Zobrist), TT persistence (BrsSearcher in OdinEngine), root TT probe safety, hybrid reply fallback. 3 issues resolved.
- **2026-02-26** [[Session-2026-02-26-BRS-Architecture-Investigation]] -- Narrowing fix (root-capture protection), hanging penalty experiment (reverted), deep BRS architecture investigation: paranoid modeling too aggressive, TT not player-aware, TT fresh per search. 4 issues cataloged.
- **2026-02-26** [[Session-2026-02-26-PST-Tuning]] -- PST tuning: knight gradient flattened (+23→+10cp first hop), bishop development strengthened. Clippy cleanup.
- **2026-02-26** [[Session-2026-02-26-KingSafety-SEE-Hotfixes]] -- King safety + SEE hotfixes: KING_GRID rank 1 made negative, SEE defense check, pawn shield + open king file constants raised.
- **2026-02-25** [[Session-2026-02-25-Stage9-TT-Ordering]] -- Stage 9: TT & Move Ordering. TranspositionTable (depth-preferred, mate-score ply adjustment), full ordering pipeline (TT hint → win caps → killers → counter-move → history quiets → lose caps), simplified SEE. 58% node reduction at depth 6. 387 engine tests.
- **2026-02-25** [[Session-2026-02-25-PostElim-Crash-Fix]] -- Post-elimination crash fix: engine panicked when BRS search reached eliminated player's turn (kingless board → generate_legal corrupts state). Four-layer fix: alphabeta skip, quiescence skip, board scanner Active-only filter, king square 255 sentinel. Eval strengthening: PAWN_SHIELD_BONUS 35, MVV-LVA ordering, THREAT_PENALTY 50. Version canary v0.4.1-fix. User verified.
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
