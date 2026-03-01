# HANDOFF — Last Session Summary

**Date:** 2026-02-28
**Stage:** Stage 17 (Game Mode Variant Tuning) — IMPLEMENTATION COMPLETE.
**Next:** Human review, then commit + tag `stage-17-complete` / `v1.17`.

## What Was Done This Session

### Stage 17: Game Mode Variant Tuning (Claude.T implementation)

#### Chess960 Position Generator (Steps 1a-1e)

1. **`board_struct.rs` — `castling_starts` field** — Added `[(Square, Square, Square); PLAYER_COUNT]` to Board storing (king_start, ks_rook, qs_rook) per player. Initialized in both `Board::empty()` and `Board::starting_position()` with standard values. Accessors `castling_starts()` and `set_castling_starts()`.

2. **`moves.rs` — Castling refactored** — `castling_config(player, board)` reads from `board.castling_starts()`. `castling_empty_squares()` fully rewritten: computes union of king-to-destination + rook-to-destination paths, minus start squares. `castling_king_path()` updated with board param. `walk_path_inclusive()` helper added. **Critical Chess960 fix:** make/unmake castling uses atomic remove-both-then-place pattern (prevents "square already occupied" panic when king destination overlaps rook start).

3. **`generate.rs`** — `generate_castling()` passes `board` to all castling helpers.

4. **`chess960.rs` (CREATED)** — `generate_back_rank(seed)`: standard Chess960 placement algorithm using SplitMix64. `is_valid_chess960()`: validation function.

5. **`board_struct.rs` — `chess960_position(seed)`** — Generates Chess960 starting position for all 4 players. Red/Green use array as-is, Blue/Yellow use reversed array. KS/QS rook identification via `determine_ks_qs_rook()` helper.

6. **`protocol/` — Chess960 option** — `chess960: bool` in EngineOptions, `setoption name Chess960 value true/false`, `handle_position_startpos` uses `Board::chess960_position()` when enabled.

#### DKW Awareness (Steps 3a-3c)

7. **`brs.rs` — Dead piece capture ordering** — `order_moves()` checks `PieceStatus::Alive` on captured piece. Dead captures get `victim_val = 1` (minimal, sorts after alive captures).

8. **`mcts.rs` — Dead piece priors** — `compute_priors()` signature changed to accept `board: &Board`. Dead captures get `victim_val = 1.0`. All 3 call sites + 2 test call sites updated.

9. **`eval/dkw.rs` (CREATED)** — DKW proximity penalty: 20cp when a Dead King Walking is within Manhattan distance 3 of player's king.

#### FFA Strategy (Step 4)

10. **`eval/ffa_strategy.rs` (CREATED)** — Claim-win urgency bonus (50cp Standard, 100cp Aggressive) when player has 15+ point lead with 2 active players. Opponent claim-win threat penalty. Gated on `GameMode::FreeForAll`.

#### Terrain Evaluation (Step 5)

11. **`eval/terrain.rs` (CREATED)** — King wall bonus (20cp × adjacent terrain, max 2), trap penalty (30cp at 3+ adjacent), fortress bonus (15cp per piece adjacent to terrain), outpost bonus (10cp for knight/bishop). Gated on `terrain_mode()`.

#### EvalWeights Expansion (Step 6)

12. **`eval/mod.rs`** — 5 new fields: `terrain_fortress_bonus`, `terrain_king_wall_bonus`, `terrain_king_trap_penalty`, `dkw_proximity_penalty`, `claim_win_urgency_bonus`. Both Standard and Aggressive profiles configured. DKW/FFA/Terrain integrated into `eval_for_player()`.

13. **`lib.rs`** — `mod variants` → `pub mod variants` (for test access).

14. **`stage_17_variant_tuning.rs` (CREATED)** — 18 acceptance tests (T1-T18).

### Build Verification

- `cargo build` — 0 errors, 0 warnings
- `cargo clippy -p odin-engine` — 0 warnings
- `cargo test -p odin-engine` — 557 tests (308 unit + 249 integration, 6 ignored), 0 failures
- perft(1-4) unchanged: 20/395/7800/152050

### Bugs Found & Fixed During Implementation

1. **Board::empty() castling_starts = zeros** — Square 0 maps to invalid corner (0,0). FEN4-loaded boards start from empty, triggering `castling_config` with invalid squares → panic in `walk_path_inclusive`. Fix: initialize with standard values.

2. **Chess960 castling "square already occupied"** — Standard `move_piece(from, to)` fails when king destination = rook start (or vice versa). Fix: atomic remove-both-then-place pattern for both make_move and unmake_move castling.

---

## What's Next — Priority-Ordered

### 1. Review + Tag Stage 17

Human reviews Stage 17 changes, commits, tags `stage-17-complete` / `v1.17`.

### 2. Self-Play Validation (Optional)

Using `observer/match.mjs`:
- FFA: default vs. tuned weights (50+ games)
- LKS: default vs. tuned weights (50+ games)
- Terrain: terrain-aware vs. naive (50+ games)

### 3. Gen-0 Pipeline (If Not Done)

Stage 15 Gen-0 pipeline produces trained NNUE weights.

### 4. Begin Stage 18 (Full UI)

Per MASTERPLAN.

---

## Known Issues

- **W26 (new):** DKW chance nodes in MCTS skipped — negligible impact.
- **W27 (new):** FFA self-stalemate detection skipped — too complex for marginal gain.
- **W28 (new):** Chess960 FEN notation not addressed — `position startpos` only.
- **W29 (new):** Castling make/unmake uses atomic remove-both-then-place for Chess960.
- **W30 (new):** Board::empty() initializes castling_starts with standard values.
- **W18 (carried):** King moves mark `needs_refresh` even without king bucketing.
- **W19 (carried):** EP/castling fall back to full refresh.
- **W20 (carried):** `serde` + `serde_json` in engine (datagen CLI path only).
- **W13 (carried):** MCTS score 9999 (max) in some positions.
- **Pondering not implemented:** Deferred from Stage 13.

## Files Created/Modified This Session

- `odin-engine/src/lib.rs` — MODIFIED (`pub mod variants`)
- `odin-engine/src/variants/mod.rs` — MODIFIED (`pub mod chess960`)
- `odin-engine/src/variants/chess960.rs` — CREATED
- `odin-engine/src/board/board_struct.rs` — MODIFIED (castling_starts, chess960_position, determine_ks_qs_rook)
- `odin-engine/src/movegen/moves.rs` — MODIFIED (castling refactor, Chess960-safe make/unmake)
- `odin-engine/src/movegen/generate.rs` — MODIFIED (pass board to castling helpers)
- `odin-engine/src/search/brs.rs` — MODIFIED (dead piece ordering fix)
- `odin-engine/src/search/mcts.rs` — MODIFIED (compute_priors board param, dead piece fix)
- `odin-engine/src/eval/mod.rs` — MODIFIED (3 modules, EvalWeights expansion, eval_for_player integration)
- `odin-engine/src/eval/dkw.rs` — CREATED
- `odin-engine/src/eval/ffa_strategy.rs` — CREATED
- `odin-engine/src/eval/terrain.rs` — CREATED
- `odin-engine/src/protocol/types.rs` — MODIFIED (chess960 field)
- `odin-engine/src/protocol/mod.rs` — MODIFIED (Chess960 setoption, startpos handling)
- `odin-engine/tests/stage_17_variant_tuning.rs` — CREATED (18 tests)
- `masterplan/audit_log_stage_17.md` — FILLED
- `masterplan/downstream_log_stage_17.md` — FILLED
- `masterplan/STATUS.md` — UPDATED
- `masterplan/HANDOFF.md` — REWRITTEN (this file)
- `masterplan/sessions/Session-2026-02-28-Stage17-Variant-Tuning.md` — CREATED

## Test Counts

- Engine: 557 (308 unit + 249 integration, 6 ignored)
- Python: 8 (pytest)
- UI Vitest: 54
- Total: 0 failures, 0 clippy warnings
