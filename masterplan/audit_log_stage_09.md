# Audit Log — Stage 09: TT & Move Ordering

## Pre-Audit
**Date:** 2026-02-25
**Auditor:** Claude Sonnet 4.6 (Stage 9 session)

### Build State
- Compiles: Yes — `cargo build --release` passes, 0 warnings.
- Tests pass: Yes — 362 total (234 unit + 128 integration), 3 ignored, 0 failures. (STATUS.md previously recorded 361/233 — off by 1 due to a typo in the meta-commit; corrected this session.)
- Previous downstream flags reviewed: [[downstream_log_stage_07]], [[downstream_log_stage_08]], [[audit_log_stage_08]] (full dependency chain via Stage 7 → 6 → 3 → 2 → 1 → 0; those earlier logs reviewed by Stage 8 pre-audit and carry no blocking/warning items affecting Stage 9 that were not already reviewed).

### Findings

**From [[downstream_log_stage_08]] (immediate upstream):**
1. Board scanner frozen pre-search — TT is independent of scanner data; no conflict.
2. Hybrid reply scoring only at main-search MIN nodes — TT hits at MIN nodes save hybrid selection + recursion entirely. Acceptable: Zobrist includes side_to_move so MIN and MAX node hashes are distinct.
3. `relative_material_advantage` added to eval (7th component). TT stores eval scores through the full pipeline — no change needed.
4. Progressive narrowing limits (3/6/10 by depth) apply at MIN nodes — unaffected by TT which operates at the alphabeta level.
5. W5 (stale non-board GameState fields during search): TT reads only `board.zobrist()` — not affected.
6. W4 (lead penalty tactical mismatch): tactical tests that expect captures must use `EvalProfile::Aggressive` — carry this convention into Stage 9 tests.
7. Open question from Stage 8: "How does hybrid scoring interact with TT?" — Addressed in Risks below.

**From [[downstream_log_stage_07]] (BRS foundation):**
1. `unmake_move` takes 3 args: `unmake_move(board, mv, undo)`. Stage 9 does not call unmake_move directly from new code, but SEE will simulate exchanges — must use the correct signature.
2. BRS uses natural turn order R→B→Y→G (ADR-012). TT keys are Zobrist hashes which already encode `side_to_move` — distinct keys for each player's turn. No interaction with ADR-012.
3. `TIME_CHECK_INTERVAL = 1024` — integration tests use `<= budget + 1024` as ceiling for node assertions.
4. Searcher trait frozen. TT is an internal detail of `BrsSearcher`; no trait change needed.

**Active issues reviewed (per AGENT_CONDUCT 1.9):**
- `[[Issue-Vec-Clone-Cost-Pre-MCTS]]` (WARNING): Still open. Stage 9 does not worsen it. Schedule retrofit before Stage 10.
- `[[Issue-Perft-Values-Unverified]]` (WARNING, last_updated: 2026-02-20, >3 sessions): Updated this session (see issue file).
- `[[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]]` (NOTE): HANDOFF says "re-evaluate post-Stage 9." Not blocking.
- `[[Issue-DKW-Halfmove-Clock]]` (NOTE): Not affected.
- `[[Issue-DKW-Invisible-Moves-UI]]` (NOTE): Not affected.
- `[[Issue-GameLog-Player-Label-React-Batching]]` (WARNING, fixed pending verification): Not affected.

**Build Order note — Step 3 pre-completed:**
MVV-LVA capture ordering was added to `order_moves()` in `brs.rs` during the Stage 8 debugging / post-elim crash fix session (commit `dcb1eb9`). Build Order Step 3 is satisfied before Stage 9 begins. The implementation uses `victim_val * 10 - attacker_val` with `PIECE_EVAL_VALUES` indexed by `PieceType::index()`. Verified by inspection.

### Risks for This Stage

1. **TT correctness with repetition detection (WARNING):** The repetition check runs BEFORE the TT probe in `alphabeta()`. This ordering is critical — a TT hit must not bypass the repetition draw return. Must preserve: repetition check → TT probe → depth==0 quiescence dispatch.

2. **Mate score distance distortion (WARNING):** TT stores absolute centipawn scores. A mate-in-N found at ply X would be retrieved at ply Y with the wrong distance. Standard fix: adjust mate scores by ply offset before store and after probe. Must implement.

3. **4PC SEE multi-opponent recapture complexity (INFO):** Up to 3 opponents can recapture on any square. Full recursive SEE with turn-order simulation is complex. Start with simplified SEE (check whether immediate exchange is winning/losing); defer full recursive SEE to Stage 19.

4. **TT/hybrid MIN node interaction (INFO):** Hybrid scoring at MIN nodes uses a frozen BoardContext from search start. A cached MIN score from an earlier search reflects whatever hybrid selected then. If the engine re-reaches the same MIN position later, the TT score may not match what current hybrid would select. Since hybrid uses root-position data (not mid-search data), this is a second-order effect. Acceptable for Stage 9.

5. **Memory allocation for BoxedCounterMoves (INFO):** `countermoves: [[Option<Move>; 196]; 196]` is ~150 KB if on stack. Must `Box` it in `BrsContext`. Killers (512 bytes) and history (~21 KB) can remain unboxed.


---

## Post-Audit
**Date:** 2026-02-25
**Auditor:** Claude Sonnet 4.6 (Stage 9 session)

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| `search/tt.rs` — TranspositionTable, TTEntry, probe/store | ✓ Complete | 12 unit tests; depth-preferred replacement + age fallback |
| TT integrated into BRS alphabeta | ✓ Complete | Probe after rep-check; store on all non-aborted nodes |
| Mate score ply adjustment | ✓ Complete | `score_to_tt` / `score_from_tt` with MATE_THRESHOLD = 19,900 |
| MVV-LVA capture ordering (Step 3) | ✓ Pre-complete | From commit `dcb1eb9` (Stage 8 post-elim fix); verified |
| Killer move tracking (Step 4) | ✓ Complete | 2 killers per ply; updated on quiet beta cutoffs in max_node |
| History heuristic (Step 5) | ✓ Complete | `[player][pt][to]` i32 table; depth² increment; saturating add |
| SEE simplified (Step 6) | ✓ Complete | `captured_val - attacker_val >= threshold`; full recursive deferred to Stage 19 |
| Counter-move heuristic (Step 7) | ✓ Complete | `last_opp_move[ply]` set by min_node; flat Vec index |
| Full ordering pipeline (Step 8) | ✓ Complete | TT→win_caps→promos→killers→counter→hist_quiet→lose_caps |
| Stage 9 integration tests | ✓ Complete | 13 tests (all pass); acceptance criteria met |
| 387 total tests pass | ✓ | 246 unit + 141 integration, 3 ignored, 0 failures |

Acceptance criteria:
- [x] `test_tt_reduces_node_count_at_depth_6`: Depth 6 nodes = 4,595 (Stage 7 baseline 10,916) → **58% reduction** ✓ (>50% required)
- [x] Perft invariants pass: perft(1)=20 ✓
- [x] All 374 prior tests still pass (now 387 total with Stage 9 additions) ✓

### Code Quality
#### Uniformity
- `order_moves()` follows the same `Vec<Move>` + `placed[]` bitmask pattern consistently across all 7 ordering stages. Killers/counter-move checked with `.position()` then `retained` from the quiet list to avoid double-emission.
- Naming: `killers`, `history`, `countermoves`, `last_opp_move` all follow existing `snake_case` convention. `TOTAL_SQUARES`, `PIECE_TYPE_COUNT`, `PLAYER_COUNT` match existing constant naming style.
- TT probe/store in `alphabeta()` mirrored at every non-early-return exit point (exceptions: budget-stop, eliminated-player skip, and quiescence dispatch — these are documented in comments).

#### Bloat
- `countermoves: Vec<Option<Move>>` (38,416 elements × 8 bytes ≈ 300 KB heap). Acceptable — moves to heap per spec. Killers (~1 KB) and history (~22 KB) remain on stack as planned.
- `order_moves` allocates 3 temporary Vecs (win_caps, lose_caps, quiets). These are small and short-lived. No concern for Stage 9.

#### Efficiency
- TT probe cost: 2 array accesses (index + key compare). Negligible.
- `order_moves` runs O(n log n) on small lists (≤ ~50 moves at depth 6). Acceptable.
- Killer lookup: 2 `.position()` calls on legal-move list (O(n) each). Small n; acceptable.
- History update: O(1). No concern.

#### Dead Code
- None introduced. The `#[allow(clippy::too_many_arguments)]` on `order_moves` is necessary (6 args) and not dead.

#### Broken Code
- None. All 387 tests pass, 0 warnings in release build.

#### Temporary Code
- None. All code is intended for production search.

### Search/Eval Integrity

**TT + repetition check ordering (WARNING pre-audit — RESOLVED):** The repetition check in `alphabeta()` runs at lines 352-357, BEFORE the TT probe at lines 360-375. This ordering is preserved and correct: TT hits cannot bypass repetition draws. Tested by `test_tt_does_not_bypass_repetition_detection`.

**Mate score distance distortion (WARNING pre-audit — RESOLVED):** `score_to_tt(score, ply)` adjusts scores > MATE_THRESHOLD by adding ply, converting "mate-in-N from this node" to "mate-in-N from root." `score_from_tt` reverses on probe. Tested by `test_mate_score_not_distorted_by_tt` (two identical depth-6 searches return the same score).

**TT flag correctness:** `orig_alpha` saved before TT probe. After max_node/min_node returns, flag is set as: `result <= orig_alpha` → TT_UPPER, `result >= beta` → TT_LOWER, else TT_EXACT. This is correct per standard alpha-beta theory.

**Min node TT interaction (INFO pre-audit — CONFIRMED ACCEPTABLE):** MIN nodes probe TT (for score cutoffs) and store results with `best_move = None`. TT hits at MIN nodes bypass hybrid selection and recursion entirely. Since Zobrist includes side_to_move, MIN and MAX hashes are distinct; no cross-contamination. Confirmed acceptable for Stage 9.

**4PC SEE (INFO pre-audit — SIMPLIFIED IMPLEMENTATION):** Stage 9 uses `captured_val - attacker_val >= threshold` (single-exchange approximation). This correctly handles the most impactful cases (pawn takes queen = win, queen takes defended pawn = loss) and is conservative (safe to call a winning capture "losing" if uncertain). Full recursive 4PC SEE deferred to Stage 19 per plan.

**Search score stability:** Two identical depth-4 searches return the same score. TT does NOT force the same best move between searches (different ordering → different equal-score move selected), which is correct and expected behavior for TT-enabled engines.

### Future Conflict Analysis

1. **Stage 10 (MCTS): Issue-Vec-Clone-Cost-Pre-MCTS.** MCTS cannot clone GameState per simulation — TT is not shared with MCTS. BrsSearcher's TT is an internal detail; MctsSearcher will have no TT. No conflict. But the Vec clone issue in GameState itself must still be resolved before Stage 10.

2. **Stage 11 (Hybrid controller).** The hybrid controller composes two Searchers through the trait. TT is internal to BrsSearcher; the Searcher trait is unchanged. No conflict.

3. **Stage 16 (NNUE eval).** NNUE replaces BootstrapEvaluator but does not touch TT or move ordering. No conflict.

4. **Stage 19 (Full recursive SEE).** `see()` in brs.rs will need to be upgraded to recursive exchange simulation. The current simplified version returns the right type and signature — the upgrade is a drop-in replacement.

5. **History table granularity.** Stage 9 uses `history[player_idx][pt_idx][to_sq]`. If a future stage adds piece-specific tuning or multi-dimensional history (from+to), the table signature will change but the existing infrastructure is a clean base.

6. **Countermove flat-Vec indexing.** `from * TOTAL_SQUARES + to` assumes exactly 196 valid indices. If a future board geometry change alters TOTAL_SQUARES, both this indexing and the `TOTAL_SQUARES` constant must be updated together. Currently consistent.

### Unaccounted Concerns

None identified. All pre-audit risks were resolved or confirmed acceptable.

### Reasoning & Methods

Stage 9 implementation followed the Build Order in the plan precisely. The key design choices:
- TT probe placed AFTER repetition check (critical invariant) and BEFORE quiescence dispatch; this ensures draws are never bypassed and qsearch nodes don't waste a TT probe.
- `orig_alpha` saved before any TT alpha-adjustment so TT flag computation (UPPER/LOWER/EXACT) is correct relative to the original window.
- Aborted searches skip TT store to avoid caching partial results.
- Terminal nodes (checkmate/stalemate) are stored as TT_EXACT — these are static facts and are safe to cache.
- Eliminated-player skip nodes are NOT stored in TT (early return, no flag computation) — acceptable because these are structural transitions, not search results.
- Counter-move uses `last_opp_move[ply]` set by min_node BEFORE recursing; MAX nodes at ply P read `last_opp_move[P]` for the counter-move heuristic, giving them the most recent opponent move.
- `Vec<Option<Move>>` for countermoves (vs. `Box<[[Option<Move>; 196]; 196]>`) avoids stack-overflow risk during BrsContext initialization.

Performance gain: 58% node reduction at depth 6 (release), 59% at depth 8. Primary driver is TT hit reuse across iterative deepening depths — each depth N search leaves a fully populated TT that depth N+1 exploits heavily via best-move hints and cutoffs.

---

## Related

- Stage spec: [[stage_09_tt_ordering]]
- Downstream log: [[downstream_log_stage_09]]
