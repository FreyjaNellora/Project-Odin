# HANDOFF — Last Session Summary

**Date:** 2026-02-26
**Stage:** Post-Stage-9 — Eval + SEE Hotfixes (gameplay quality)
**Next:** Resolve `Issue-Vec-Clone-Cost-Pre-MCTS`, then begin Stage 10 (MCTS)

## What Was Done This Session

### Gameplay Quality Hotfixes (commit `a37b237`)

User observed two gameplay problems in the running app: Blue's king walking forward freely, and Blue pushing an undefended pawn that Yellow's bishop captured for free.

**Root causes found and fixed:**

**Bug 1 — King walk (Ka7b6): `pst.rs` KING_GRID rank 1 was mildly positive**
- KING_GRID rank 1 values were `[0,0,0,10,10,5,0,0,5,10,10,0,0,0]` — a king one step forward from the back rank was getting +5 to +10cp from PST.
- In 4PC, pawns may advance and strip king shield. After pawn pushes, the combined king safety + PST difference between a7 and b6 was essentially zero — engine saw no penalty for walking.
- Fix: changed rank 1 to `[0,0,0,-5,-5,-10,-15,-15,-10,-5,-5,0,0,0]`. King one step forward is now clearly penalized (up to -15cp at center files). All PST values remain within ±50cp bounds.

**Bug 2 — Hanging pawn / SEE misclassification: bishop×undefended pawn → lose_caps bucket**
- `see(bishop×pawn, 0)` computed `100 - 500 = -400 < 0` → classified as losing capture regardless of whether the pawn was defended.
- Losing captures go last in `order_moves()` pipeline. Progressive narrowing at depth 7+ (limit=3) then cuts the move before it's ever explored.
- Fix: `see()` now first checks if any opponent attacks `to_sq` via `is_square_attacked_by`. If the captured piece is undefended (no recapture possible), it's always a winning capture — `captured_val >= threshold` is returned directly without attacker-value subtraction.
- `see()` signature changed from `see(mv, threshold)` to `see(board, mv, player, threshold)`.
- `order_moves()` signature updated: `board: &Board` added, `player_idx: usize` replaced by `player: Player` (index derived inside).

**King safety constant increases:**
- `PAWN_SHIELD_BONUS`: 35 → 50cp (max 150cp for full 3-pawn shield vs 105cp before)
- `OPEN_KING_FILE_PENALTY`: 25 → 40cp
- Test assertion updated: `== 105` → `== 150`

**Tests:** All 387 engine tests pass. No regressions.

---

### Stage 9 Implementation — Full Build Order (Previous Session 2026-02-25)

Stage 9 added a Transposition Table and full move ordering pipeline to the BRS search.

**Pre-work (entry protocol):**
- Tags `stage-08-complete` / `v1.8` confirmed already applied (from previous session)
- `cargo build && cargo test`: 362 tests passing, 0 warnings — confirmed clean
- Pre-audit filled in `audit_log_stage_09.md`
- `Issue-Perft-Values-Unverified` staleness updated (last_updated → 2026-02-25)
- STATUS.md corrected: 361→362 tests (233→234 unit, typo from meta-commit)

**Step 1: TT data structure** (`search/tt.rs` — new file)
- `TTEntry` (12 bytes): key u32, best_move u16, score i16, depth u8, flags u8
- `TranspositionTable`: Vec<TTEntry>, power-of-2 mask, 6-bit generation counter
- API: `probe()`, `store()`, `compress_move()`, `decompress_move()`, `increment_generation()`
- Mate score ply adjustment: `score_to_tt` / `score_from_tt` (MATE_THRESHOLD = 19,900)
- Depth-preferred replacement with generation fallback
- 12 unit tests — all pass
- `pub mod tt;` added to `search/mod.rs`

**Step 2: TT integration into BRS** (`brs.rs`)
- `BrsSearcher.tt: TranspositionTable` (persists across searches; TT_DEFAULT_ENTRIES = 1<<20 ~12 MB)
- `BrsContext.tt: &'a mut TranspositionTable`
- `alphabeta()`: hash hoisted; TT probe AFTER rep-check, BEFORE qsearch dispatch; TT store at bottom (skipped when stopped); `orig_alpha` saved for flag computation; terminal nodes stored TT_EXACT
- `max_node()`: accepts `tt_move: Option<Move>` hint; uses `tt_move.or(pv_move)`
- Commit: `9f3ab88`

**Steps 3-8: Killer/History/SEE/Counter-move + Full Pipeline** (`brs.rs`)
- `TOTAL_SQUARES = 196`, `PIECE_TYPE_COUNT = 7`, `PLAYER_COUNT = 4` constants
- BrsContext additions: `killers [[Option<Move>; 2]; 64]`, `history [[[i32; 196]; 7]; 4]`, `countermoves Vec<Option<Move>>` (flat 196×196), `last_opp_move [Option<Move>; 64]`
- Beta cutoff in max_node: killers updated, history += depth², counter-move recorded
- min_node: `last_opp_move[ply+1] = Some(mv)` before recursing
- `see(mv, threshold) -> bool`: simplified single-exchange; full recursive SEE deferred to Stage 19
- `order_moves()` rewritten: TT hint → win_caps (SEE≥0, MVV-LVA desc) → promos → killers → counter-move → hist-sorted quiets → lose_caps (SEE<0)
- Commit: `5d9ccbd`

**Integration tests** (`tests/stage_09_tt_ordering.rs` — new file, 13 tests)
- TT reduces nodes at depth 6 (acceptance criterion)
- Score stability on repeat searches
- Mate score not distorted by TT
- Perft(1) = 20 unchanged
- Monotone node growth with fresh searchers
- TT hint enables faster warm search
- Killers improve repeat search node counts
- No history overflow at depth 7
- TT does not bypass repetition detection
- PV starts with best_move
- Commit: `a7dae37`

### Post-Audit
- Post-audit section filled in `audit_log_stage_09.md`
- `downstream_log_stage_09.md` written (previously a shell)

## Performance Results

| Depth | Nodes (Stage 9) | Nodes (Stage 7) | Reduction |
|-------|-----------------|-----------------|-----------|
| 6     | 4,595           | 10,916          | **58%**   |
| 8     | 13,009          | 31,896          | **59%**   |

Acceptance criterion of >50% node reduction at depth 6: **MET with margin**.

## What's Next

1. **Resolve `Issue-Vec-Clone-Cost-Pre-MCTS`** (WARNING): MCTS cannot clone GameState per simulation. Recommended order: Refinement 2 first (`position_history: Vec<u64>` → `Arc<Vec<u64>>`, minimal), then Refinement 1 (`piece_lists` → fixed-size array, heavier). Full details in issue file.
2. **Begin Stage 10 (MCTS)**: Read `masterplan/stages/stage_10_mcts.md`, upstream audit/downstream logs (stages 7-9), `cargo build && cargo test`.
3. **Stage 9 tags** (`stage-09-complete` / `v1.9`) — already applied from previous session. No re-tag needed.

## Known Issues

- `Issue-Vec-Clone-Cost-Pre-MCTS` (WARNING): OPEN — **resolve before Stage 10**
- W6 (simplified SEE): `see()` is single-exchange only; full recursive 4PC SEE deferred to Stage 19
- W5 (stale GameState fields during search): acceptable for bootstrap eval, revisit for NNUE
- W4 (lead penalty tactical mismatch): mitigated by Aggressive profile for FFA
- `Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch` (NOTE): still open, not blocking
- `Issue-DKW-Halfmove-Clock` (NOTE): still open, not blocking
- `Issue-GameLog-Player-Label-React-Batching` (WARNING, fixed pending verification): not re-tested this session

## Files Modified This Session (2026-02-26)

### Engine
- `odin-engine/src/eval/pst.rs` — KING_GRID rank 1 made negative (king safety, prior commit); KNIGHT_GRID gradient flattened; BISHOP_GRID rank 0-1 strengthened; ROOK_GRID center preference; QUEEN_GRID minor boost
- `odin-engine/src/eval/king_safety.rs` — `PAWN_SHIELD_BONUS` 35→50, `OPEN_KING_FILE_PENALTY` 25→40; test updated
- `odin-engine/src/search/brs.rs` — `see()` defense check via `is_square_attacked_by`; `order_moves()` takes `board` + `player`; clippy fixes (is_multiple_of, manual_inspect)
- `odin-engine/src/search/board_scanner.rs` — clippy fixes (get_first, range_loop, collapsible_if, match_equality, manual_range_contains, map_or, needless_range_loop)
- `odin-engine/src/search/tt.rs` — `is_empty()` added (clippy len_without_is_empty)
- `odin-engine/src/protocol/emitter.rs`, `odin-engine/src/protocol/mod.rs` — clippy formatting
- `odin-engine/tests/stage_06_eval.rs`, `tests/stage_07_brs.rs`, `tests/stage_08_brs_hybrid.rs` — clippy formatting

### Documentation
- `masterplan/HANDOFF.md` — updated (this file)
- `masterplan/STATUS.md` — non-stage change added
- `masterplan/sessions/Session-2026-02-26-PST-Tuning.md` — NEW

## Files Modified Previous Session (2026-02-25)

### Engine
- `odin-engine/src/search/tt.rs` — NEW (TT data structure)
- `odin-engine/src/search/brs.rs` — TT integration + ordering pipeline
- `odin-engine/src/search/mod.rs` — `pub mod tt;` added
- `odin-engine/tests/stage_09_tt_ordering.rs` — NEW (13 integration tests)

### Documentation
- `masterplan/audit_log_stage_09.md` — pre + post audit filled
- `masterplan/downstream_log_stage_09.md` — filled (was a shell)
- `masterplan/STATUS.md` — Stage 9 complete, test counts updated, performance baselines added
- `masterplan/HANDOFF.md` — updated (this file)
- `masterplan/issues/Issue-Perft-Values-Unverified.md` — staleness updated

## Test Counts

- Engine: 387 (246 unit + 141 integration, 3 ignored)
- UI Vitest: 54
- Total: 0 failures
