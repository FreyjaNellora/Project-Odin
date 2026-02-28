# PROJECT ODIN — STATUS

**Last Updated:** 2026-02-28
**Updated By:** Claude Opus 4.6 (Stage 11 implementation complete, pending human review)

---

## Current State

| Field | Value |
|-------|-------|
| **Current Stage** | Stage 11 (Hybrid Integration) — IMPLEMENTATION COMPLETE. Pending human review + tag. |
| **Current Build-Order Step** | Stage 12 (Self-Play & Regression Testing) — not started. |
| **Build Compiles** | Yes — `cargo build` passes, 0 warnings, 0 clippy warnings |
| **Tests Pass** | Yes — engine: 281 unit + 176 integration = 457 total (4 ignored); UI: 54 Vitest. |
| **Blocking Issues** | None |

---

## Stage Completion Tracker

| Stage | Name | Status | Audited | Git Tag | Notes |
|-------|------|--------|---------|---------|-------|
| 0 | Project Skeleton | complete | post-audit done | stage-00-complete / v1.0 | |
| 1 | Board Representation | complete | post-audit done | stage-01-complete / v1.1 | |
| 2 | Move Generation + Attack Query API | complete | post-audit done | stage-02-complete / v1.2 | |
| 3 | Game State & Rules | complete | post-audit done | stage-03-complete / v1.3 | |
| 4 | Odin Protocol | complete | post-audit done | stage-04-complete / v1.4 | |
| 5 | Basic UI Shell | complete | post-audit done | stage-05-complete / v1.5 | |
| 6 | Bootstrap Eval + Evaluator Trait | complete | post-audit done | stage-06-complete / v1.6 | |
| 7 | Plain BRS + Searcher Trait | complete | post-audit done | stage-07-complete / v1.7 | |
| 8 | BRS/Paranoid Hybrid Layer | complete | post-audit done | stage-08-complete / v1.8 | User verified. Post-elim crash fixed (v0.4.1-fix). |
| 9 | TT & Move Ordering | complete | post-audit done | stage-09-complete / v1.9 | 58% node reduction at depth 6; 387 tests. |
| 10 | MCTS | complete | post-audit done | stage-10-complete / v1.10 | Gumbel MCTS standalone, 1000 sims in 124ms release. 440 tests. |
| 11 | Hybrid Integration | complete | post-audit done | — | HybridController: BRS→MCTS two-phase. 457 tests. Pending tag. |
| 12 | Self-Play & Regression Testing | not-started | — | — | |
| 13 | Time Management | not-started | — | — | |
| 14 | NNUE Feature Design & Architecture | not-started | — | — | |
| 15 | NNUE Training Pipeline | not-started | — | — | |
| 16 | NNUE Integration | not-started | — | — | |
| 17 | Game Mode Variant Tuning | not-started | — | — | |
| 18 | Full UI | not-started | — | — | |
| 19 | Optimization & Hardening | not-started | — | — | |

**Status values:** `not-started` | `in-progress` | `complete` | `blocked`
**Audited values:** `—` (not applicable) | `pre-audit done` | `post-audit done` | `audit-failed`

---

## Documentation Status

| Document | Status | Notes |
|----------|--------|-------|
| MASTERPLAN.md | current | v3.1 (minor refinements applied per recent commit). |
| AGENT_CONDUCT.md | current | v1.2 — Section 1.18 added (Diagnostic Observer Protocol). |
| 4PC_RULES_REFERENCE.md | current | Complete game rules. |
| DECISIONS.md | current | 15 ADRs. ADR-007/008 superseded by ADR-015 (Huginn → tracing). ADR-014 (UI Vision), ADR-015 (Retire Huginn). |
| HANDOFF.md | current | Stage 11 complete, pending review + tag. |
| STATUS.md (this file) | current | |
| README.md | current | Project overview at repo root. |
| audit_log_stage_00.md through audit_log_stage_11.md | current | All complete. |
| downstream_log_stage_00.md through downstream_log_stage_11.md | current | All complete. |

---

## What the Next Session Should Do First

1. Read STATUS.md + HANDOFF.md
2. Human reviews Stage 11 changes, tags `stage-11-complete` / `v1.11`
3. Begin Stage 12 (Self-Play & Regression Testing) per AGENT_CONDUCT.md Section 1.1

---

## Known Regressions

None. All tests pass (457 engine + 54 UI Vitest).

---

## Non-Stage Changes

**2026-02-28 — Stage 11: Hybrid Integration (BRS→MCTS)** ([[Session-2026-02-28-Stage11-Hybrid]]):

HybridController in `search/hybrid.rs` (~280 lines). Two-phase search: BRS Phase 1 (tactical filter, adaptive 10-30% time budget) → MCTS Phase 2 (strategic search, BRS-informed priors + progressive history warm-start). BRS modifications: `last_history` + `last_root_move_scores` extraction, `take_info_callback`, null move ply>0 guard, root score tracking at ply 0. MCTS modifications: external_priors wired into root expansion (replaces MVV-LVA when available), `take_info_callback`, history cleanup. Protocol: `Option<HybridController>` replaces `Option<BrsSearcher>`. Adaptive time allocation: tactical positions (≥30% captures) get 30/70 BRS/MCTS split, quiet positions get 10/90. BRS_MAX_DEPTH=8. All AC1-AC7 pass. Tests: 281 unit + 176 integration = 457 total (4 ignored), 0 clippy warnings.

**2026-02-27 — Stage 10: Gumbel MCTS Implementation** ([[Session-2026-02-27-Stage10-MCTS]]):

Standalone Gumbel MCTS searcher in `search/mcts.rs` (~550 lines). Implements frozen Searcher trait. SplitMix64 PRNG (no rand dependency). Gumbel-Top-k + Sequential Halving at root, PUCT tree policy, 4-player MaxN backprop, progressive widening (non-root), eval_4vec leaf eval. SimConfig struct for clean parameter passing. MctsSearcher: new(), with_seed(), with_info_callback(). Stage 11 stubs: set_prior_policy(), set_history_table(), HistoryTable type alias. All AC1-AC8 pass. 1000 sims in 124ms release. Tests: 281 unit + 159 integration = 440 total (4 ignored), 0 clippy warnings.

**2026-02-27 — Observer Infrastructure + Baselines + Stage 10 Prep** ([[Session-2026-02-27-Observer-Baselines-Stage10Prep]]):

AGENT_CONDUCT Section 1.18 (Diagnostic Observer Protocol). Observer LogFile toggle in `observer.mjs`. Human baselines: 6 chess.com 4PC FFA games (2 strong 3000+, 3 weak 1954-2709, 1 engine v0.4.3) in `observer/baselines/` with structured JSON + markdown. Depth-8 diagnostic: engine plays at ~2100-2300 Elo avg (zero captures, piece shuffling, asymmetric sides). Stage 10 Claude.T prompt written (`stage_10_mcts_prompt.md`).

**2026-02-27 — Pre-Stage-10 Final Cleanup** ([[Session-2026-02-27-PreStage10-Cleanup]]):

Audit fixes: `lead_penalty()` ffa_point_weight threading (W1), `Player::prev()` consolidation (W2), clippy const assertions (N1). Pawn-push/king-walk mitigations: development bonuses increased (Knight 25→45, Bishop 15→30, Queen 35→50, Rook 15→25), connected pawn advance gate (2+ ranks from start), king displacement penalty (-40cp off home rank). Vec clone retrofit: `position_history` → `Arc<Vec<u64>>` (O(1) clone), `piece_lists` → fixed-size `[(PieceType, Square); 20]` arrays with `piece_counts` (zero heap alloc on clone). Issue-Vec-Clone-Cost-Pre-MCTS resolved. Tests: 267 unit + 141 integration = 408 total, all passing. 0 clippy warnings.

**2026-02-27 — BRS Score Cap + Pawn Structure + Depth 8** ([[Session-2026-02-27-BRS-ScoreCap-PawnStructure]]):

BRS phantom mate fix: depth >= 8 gate on mate-break early termination, `BRS_SCORE_CAP = 9,999` display clamping (info lines + SearchResult). Connected pawn bonus: +8cp per pawn defended by friendly pawn (new `pawn_structure.rs`). Development bonus: Queen +35cp, Knight +25cp, Rook +15cp, Bishop +15cp off back rank (new `development.rs`). Default depth 7 -> 8. New issue created: [[Issue-Pawn-Push-Preference-King-Walk]] (engine prefers pawn pushes over development, walks king). Tests: 264 unit + 141 integration = 405 total, all passing.

**2026-02-27 — Multi-Perspective Opponent Modeling** ([[Session-2026-02-27-Multi-Perspective]]):

Replaced 2-term likelihood formula with 3-term dynamic blend: `score = w_paranoid * harm_to_root + w_brs * objective_strength + w_anti_leader * harm_to_leader`. Weights are context-driven (opponent targeting, material gaps, vulnerability), normalized to 1.0. Added `find_leader()`, `compute_harm_to_player()`, `BlendWeights` struct, `compute_blend_weights()`. Deleted 5 `LIKELIHOOD_*` constants. Updated `ScoredReply` (likelihood → harm_to_leader). 7 new tests. ENGINE_VERSION = v0.5.0-multi-perspective. Tests: 253 unit + 143 integration = 396 total, all passing.

**2026-02-27 — Engine Game Analysis Fixes** ([[Session-2026-02-27-Game-Analysis-Fixes]]):

Self-play game analysis (11 moves/player) revealed 3 bugs + 4 additional issues, all rooted in paranoid opponent modeling (80/20 blend). Fixes: (1) Likelihood constants tuned to 50/50 blend (`LIKELIHOOD_BASE_TARGETS_ROOT` 0.7→0.5, `LIKELIHOOD_EXPOSED_PENALTY` 0.3→0.5, `LIKELIHOOD_BASE_NON_ROOT` 0.2→0.3). (2) `select_hybrid_reply` fallback replaced with eval-based `pick_objectively_strongest()`. (3) TT made player-aware via `root_player` Zobrist keys XOR'd into `tt_hash`. (4) `BrsSearcher` persisted in `OdinEngine` (TT survives across `go` commands). (5) Root TT probe safety: ply-0 probe returns move hint only, no alpha/beta tightening. 3 issues resolved: [[Issue-BRS-Paranoid-Opponent-Modeling]], [[Issue-TT-Not-Player-Aware]], [[Issue-TT-Fresh-Per-Search]]. Tests: 246 unit + 143 integration = 389 total, all passing.

**2026-02-26 — Narrowing Fix + BRS Architecture Investigation** ([[Session-2026-02-26-BRS-Architecture-Investigation]]):

Progressive narrowing was too aggressive at depth 7+ (limit=3), pruning opponent capture moves. Fix: widened limits (12/8/5), added root-capture protection in `board_scanner.rs`. Hanging piece penalty experiment reverted (double-counted search threats, caused Nf3→e1 regression). Deep BRS architecture investigation: hybrid scoring too paranoid (80/20 blend), TT not player-aware, TT fresh per search. 4 issues created: [[Issue-BRS-Paranoid-Opponent-Modeling]], [[Issue-TT-Not-Player-Aware]], [[Issue-TT-Fresh-Per-Search]], [[Issue-Hanging-Piece-Eval-Double-Count]] (resolved). ENGINE_VERSION = v0.4.3-narrowing. Tests: 248 unit + 141 integration = 389 total, all passing.

**2026-02-26 — PST Tuning: Knight Gradient + Bishop Development** ([[Session-2026-02-26-PST-Tuning]]):

User observed "knight chess" — all four players opening with 3-4 knight moves each, bishops rarely developing. Root cause: knight gradient was spring-loaded (+23cp first hop) dominating all alternatives. Fix: flattened KNIGHT_GRID (first hop -8→+5 = **+10cp**, was -15→+8 = **+23cp**), redesigned BISHOP_GRID (rank0 -15cp back-rank penalty, rank1 center +15cp, ranks 4-8 +32cp), ROOK_GRID center preference, QUEEN_GRID minor boost. New 2-move math: g-pawn + bishop fianchetto = +45cp vs two knights = +20cp. Clippy: 12 pre-existing warnings cleared (board_scanner, brs, tt, protocol files).
Tests: 248 unit + 141 integration = 389 total, all passing.

**2026-02-26 — King Safety + SEE Hotfixes** ([[Session-2026-02-26-KingSafety-SEE-Hotfixes]]):

User observed Blue walking its king freely and pushing an undefended pawn taken for free by Yellow's bishop. Two eval/search bugs fixed:
1. `pst.rs` KING_GRID rank 1 was mildly positive (+5 to +10cp) — changed to negative (0 to -15cp). King one step forward is now clearly penalized.
2. `see()` didn't check piece defense before comparing piece values — bishop×undefended_pawn got classified as losing capture (100-500=-400), sent to the back of move ordering, and pruned by progressive narrowing. Fixed: `is_square_attacked_by` check on `to_sq` before exchange calculation. Also raised `PAWN_SHIELD_BONUS` 35→50cp, `OPEN_KING_FILE_PENALTY` 25→40cp.
Commit: `a37b237`. All 387 tests pass.

**2026-02-25 — Post-Elimination Crash Fix + Eval Strengthening** ([[Session-2026-02-25-PostElim-Crash-Fix]]):

Engine panicked when BRS search tree reached an eliminated player's virtual turn (`generate_legal` on kingless board). Four-layer fix: alphabeta skip, quiescence skip, board scanner Active-only filter, king square 255 sentinel (`remove_king` now calls `clear_king_square`). Added `has_king()` and `clear_king_square()` to `Board`. Eval strengthened: `PAWN_SHIELD_BONUS` 35, MVV-LVA capture ordering, `THREAT_PENALTY_PER_OPPONENT` 50. Binary verified via `ENGINE_VERSION = "v0.4.1-fix"` canary. User confirmed fix. Commits: `dcb1eb9`, `5eaa072`, `445638d`.

**2026-02-25 — In-Search Repetition + UI Bugfixes** ([[Session-2026-02-25-UI-Bugfixes]]):

In-search repetition detection added to BRS (`game_history` snapshot + `rep_stack` path-local, push/pop in max_node/min_node). Depth default raised to 7. Piece-prefix notation added to game log. Critical bug fixed: game log player labels were shifted by one due to React 18 batching — `currentPlayerRef.current` was read inside a deferred functional updater, seeing the *next* player's value. Fixed by snapshotting both `currentPlayerRef.current` and `boardRef.current` as locals before calling `setMoveHistory`. Commits: `f50fc57`, `b98c087`.

**2026-02-24 — UI Pause/Resume Bugfix** ([[Session-2026-02-24-Bugfix-Pause-Resume]]):

Fixed race condition in `useGameState.ts` where pausing and resuming auto-play could send duplicate `position + go` commands to the engine, causing one player to move twice in a row. Two guards added: `sendGoFromRef` checks `awaitingBestmoveRef` before sending, `togglePause` skips scheduling if a search is already in flight. See [[Issue-UI-Pause-Resume-Race-Condition]].

**2026-02-23 — UI QoL Session** ([[Session-UI-QoL-2026-02-23]]):

Added 4 new UI components and enhanced 2 existing ones outside the numbered stage pipeline:
- `AnalysisPanel` — prominent NPS display + search summary (replaces DebugConsole info section)
- `GameLog` — enriched move history with per-move eval/depth/nodes and player-colored borders
- `EngineInternals` — collapsible panel: search phase, BRS surviving, MCTS sims, per-player values
- `CommunicationLog` — raw protocol log + command input (split from DebugConsole)
- `BoardSquare` — optional coordinate labels on each square
- `BoardDisplay` — mouse wheel zoom (CSS transform, known buggy — polish later)
- Right panel reorganized from single DebugConsole to stacked sections

Follow-up items noted but not blocking:
- Board zoom frame boundary shift (cosmetic, polish phase)
- Info duplication between AnalysisPanel and EngineInternals
- No per-player scoring log (capture/elimination point tracking)

---

## Performance Baselines

| Metric | Value | Stage | Notes |
|---|---|---|---|
| `cargo build` (dev) | 0.70s | 0 | Empty project baseline |
| `cargo build --release` | 1.30s | 0 | Binary: 129,024 bytes |
| Test count | 2 | 0 | |
| `cargo build` (dev, incremental) | ~0.18s | 1 | Board module added |
| `cargo build --release` | ~0.33s | 1 | Binary: 129,024 bytes (unchanged — main.rs is empty) |
| Test count | 64 | 1 | 44 unit + 2 stage-00 + 18 stage-01 |
| Test count | 125 | 2 | 87 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 |
| perft(1) | 20 | 2 | Permanent invariant |
| perft(2) | 395 | 2 | Permanent invariant |
| perft(3) | 7,800 | 2 | Permanent invariant |
| perft(4) | 152,050 / ~0.56s | 2 | Permanent invariant (debug build) |
| 1000 random games @ 100 ply | ~15s | 2 | Debug build |
| Test count | 164 | 3 | 108 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 |
| 1000 random games via GameState | ~104s | 3 | Normal mode, debug build (permanent invariant) |
| 1000 random games via GameState (terrain) | ~104s | 3 | Terrain mode, debug build (permanent invariant) |
| Test count | 229 | 4 | 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 |
| Vitest test count | 45 | 5 | 29 board-constants + 16 protocol-parser |
| Vitest test count | 54 | 7 (bugfix) | 29 board-constants + 25 protocol-parser (9 new) |
| Tauri backend compile (fresh) | ~11s | 5 | Debug profile |
| Test count | 275 | 6 | 191 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 + 11 stage-06 |
| eval_scalar per call | <10us | 6 | Release build, starting position |
| Starting material per player | 4300cp | 6 | 8P + 2N + 2B + 2R + Q + K |
| Test count | 302 | 7 | 196 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 + 11 stage-06 + 22 stage-07 |
| Test count | 316 | 8 (Step 0) | 210 unit + 106 integration (11 new unit tests for GameMode/EvalProfile) |
| Test count | 362 | 8 (complete) | 234 unit + 128 integration (3 ignored). Board scanner, hybrid scoring, eval fix, smoke-play. Post-elim crash fix added 1 unit test (has_king/clear_king_square). |
| Test count | 387 | 9 (complete) | 246 unit + 141 integration (3 ignored). TT (12 unit), Stage 9 tests (13 integration). |
| BRS depth 6 (debug, starting pos) | 1,547ms / 10,916 nodes | 7 | ~7k NPS debug |
| BRS depth 6 (release, starting pos) | 109ms / 10,916 nodes | 7 | ~100k NPS release |
| BRS depth 8 (release, starting pos) | 371ms / 31,896 nodes | 7 | Move converges at depth 6 (j1i3); stable 6-8 |
| BRS depth 4 (CI cap) | 80ms debug / 4ms release | 7 | Stable move e1f3 |
| Hybrid BRS depth 6 (release) | < 10,916 nodes (~49% reduction) | 8 | Progressive narrowing active |
| Hybrid BRS depth 8 (release) | < 31,896 nodes (~46% reduction) | 8 | Progressive narrowing active |
| Board scanner | < 1ms per call | 8 | Release build |
| TT+Ordering depth 6 (release, Standard) | 50ms / 4,595 nodes | 9 | **58% node reduction** vs Stage 7 baseline |
| TT+Ordering depth 8 (release, Standard) | 120ms / 13,009 nodes | 9 | **59% node reduction** vs Stage 7 baseline |
| TT+Ordering depth 6 (release, Aggressive) | 34ms / 4,064 nodes | 9 | |
| TT+Ordering depth 8 (release, Aggressive) | 185ms / 12,205 nodes | 9 | |
| Test count | 396 | 9 (multi-perspective) | 253 unit + 143 integration (3 ignored). 7 new board_scanner tests for blend weights. |
| Test count | 405 | 9 (score cap + pawn) | 264 unit + 141 integration (3 ignored). 8 new pawn_structure tests, 3 development tests. |
| Test count | 408 | 9 (pre-10 cleanup) | 267 unit + 141 integration (3 ignored). Audit fixes, eval mitigations, Vec clone retrofit. |
| MCTS 1000 sims (release, starting pos) | 124ms / 986 nodes | 10 | Gumbel MCTS standalone. AC5: <5s target met. |
| Test count | 440 | 10 | 281 unit + 159 integration (4 ignored). +14 MCTS unit, +18 MCTS integration. |
| Hybrid `go depth 8` (debug, starting pos) | ~10s (BRS ~4s + MCTS ~6s) | 11 | Two-phase: BRS depth 8 + MCTS 2000 sims. |
| Test count | 457 | 11 | 281 unit + 176 integration (4 ignored). +17 Stage 11 hybrid integration. |

---

*Update this file at the end of every session. It takes 2 minutes and saves the next session 30 minutes of orientation.*
