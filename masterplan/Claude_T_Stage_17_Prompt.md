# Stage 17 Prompt — Game Mode Variant Tuning

You are implementing Stage 17 of Project Odin, a four-player chess engine (14x14 board, R/B/Y/G).

**Read these files before writing any code:**
1. `masterplan/STATUS.md` — project state (Stage 16 complete, 536 tests, v1.16)
2. `masterplan/HANDOFF.md` — last session summary
3. `masterplan/AGENT_CONDUCT.md` Section 1.1 — stage entry protocol
4. `masterplan/DECISIONS.md` — ADR-014 (EvalProfile), ADR-012 (Turn Order)
5. `masterplan/MASTERPLAN.md` — Stage 17 section
6. `masterplan/downstream_log_stage_16.md` — NNUE integration contracts (W23-W25)

---

## What You're Building

The rules engine already supports DKW, FFA scoring, and Terrain modes (Stage 3). But the **search and eval are mode-blind** — they don't exploit mode-specific knowledge. Stage 17 makes the engine play intelligently in each variant.

Additionally, Chess960 position generation does not exist at all and must be built from scratch.

Currently:
- `GameMode::FreeForAll` and `GameMode::LastKingStanding` exist but search doesn't branch on them.
- DKW mechanics work in the rules layer (`process_dkw_moves`, `generate_dkw_move`), but search treats DKW kings as normal obstacles.
- FFA scoring is fully implemented (capture points, check bonuses, claim-win), and `BootstrapEvaluator` includes FFA point weight + lead penalty via `EvalProfile`. But search doesn't optimize for point accumulation vs. elimination.
- Terrain conversion works (blocking rays, uncapturable walls), but eval has no terrain-aware terms.
- Chess960 doesn't exist.

After this stage:
- BRS and MCTS handle DKW random moves as stochastic elements.
- Eval accounts for terrain geometry (fortresses, blocked lines, outpost squares near terrain walls).
- FFA scoring is better integrated — engine considers point-gaining moves more strategically.
- Chess960 valid starting positions can be generated and played.
- Mode-specific eval/search parameter tuning is validated by self-play.

---

## Existing Architecture You're Working With

### Game Mode & Variant Flags

```rust
// gamestate/mod.rs
pub enum GameMode {
    FreeForAll,
    LastKingStanding,
}

pub struct GameState {
    game_mode: GameMode,
    terrain_mode: bool,   // Terrain conversion on elimination
    rng_seed: u64,        // LCG seed for DKW random move selection
    // ...
}
```

Game modes and terrain are independent axes: you can have FFA+Terrain, LKS+Terrain, FFA (no terrain), LKS (no terrain).

### DKW System (gamestate/rules.rs)

```rust
pub enum PlayerStatus {
    Active,
    DeadKingWalking,  // Pieces are Dead, king makes random moves
    Eliminated,       // Removed from game
}

/// Random king move for DKW player. Returns None if king is stuck.
pub fn generate_dkw_move(board: &mut Board, player: Player, seed: &mut u64) -> Option<Move>;

/// Convert all pieces to Dead status (king stays, walks randomly).
pub fn convert_to_dead(board: &mut Board, player: Player);

/// Convert all pieces to Terrain (immovable walls). King removed.
pub fn convert_to_terrain(board: &mut Board, player: Player);
```

DKW moves are "instant" — processed by `process_dkw_moves()` after each real move, before checkmate/stalemate detection. They are **permanent** (no undo). The DKW king's move is random (LCG from `rng_seed`).

**Key insight for search:** After a move in the game tree, DKW players make random king moves. This is a stochastic event. The search currently ignores this — it doesn't model what DKW kings might do.

### FFA Scoring (gamestate/scoring.rs)

```rust
pub const CAPTURE_PAWN: i32 = 1;
pub const CAPTURE_KNIGHT: i32 = 3;
pub const CAPTURE_BISHOP: i32 = 5;
pub const CAPTURE_ROOK: i32 = 5;
pub const CAPTURE_QUEEN: i32 = 9;
pub const CAPTURE_PROMOTED_QUEEN: i32 = 1;
pub const CHECKMATE_POINTS: i32 = 20;
pub const STALEMATE_POINTS: i32 = 20;  // Awarded to the stalemated player!
pub const DRAW_POINTS: i32 = 10;
pub const DOUBLE_CHECK_BONUS: i32 = 1;
pub const TRIPLE_CHECK_BONUS: i32 = 5;
pub const CLAIM_WIN_LEAD: i32 = 21;    // 21+ point lead with 2 active → win
```

FFA scoring is already wired into `apply_move()`:
- Capture points added automatically
- Check bonuses added automatically
- Claim-win checked in `check_game_over()` (FFA mode only)

**What's missing in eval/search:**
- No strategic preference for high-value captures (9pt queen vs 1pt pawn) beyond material value
- No modeling of "point race" dynamics — when to go for points vs. safety
- Self-stalemate is worth 20 points (tactical opportunity when losing), not modeled

### Terrain in Movegen (movegen/attacks.rs)

Terrain pieces:
- **Block sliding rays** (line 74: `if piece.is_terrain() => break`)
- **Cannot be captured** (skipped in capture generation)
- **Don't attack** (excluded from attack detection)
- **Don't move** (skipped in piece iteration)

**What's missing in eval:**
- No bonus for pieces sheltered behind terrain walls (fortress detection)
- No penalty for pieces blocked by terrain (reduced mobility)
- No outpost detection near terrain walls
- No assessment of how terrain changes king safety geometry

### EvalProfile (eval/mod.rs)

```rust
pub enum EvalProfile {
    Standard,    // Conservative: lead penalty active, 50cp/FFA point
    Aggressive,  // FFA-optimized: no lead penalty, 120cp/FFA point
}

pub struct EvalWeights {
    pub ffa_point_weight: i16,
    pub lead_penalty_enabled: bool,
    pub lead_penalty_divisor: i16,
    pub max_lead_penalty: i16,
}
```

**Auto-resolution:** FFA → Aggressive, LKS → Standard (see `EngineOptions::resolved_eval_profile()`).

### BootstrapEvaluator (eval/mod.rs)

The bootstrap eval computes:
```
material + positional(PST) + development + pawn_structure + king_safety
  - threat_penalty + lead_penalty + ffa_points + relative_material
```

All components operate on the board as-is (with Dead/Terrain pieces), but none have game-mode-specific logic. The bootstrap eval is still used for **opponent move selection** in BRS free functions (W23 from Stage 16).

### Search Architecture

**BRS** (`search/brs.rs`): Alpha-beta with iterative deepening, aspiration windows, null move pruning, quiescence search. Has NNUE accumulator push/pop at 4 make/unmake sites. Uses `nnue_eval_scalar()` for leaf eval.

**MCTS** (`search/mcts.rs`): Gumbel MCTS with sequential halving, PUCT tree policy, 4-player MaxN backprop. Has NNUE accumulator tracking during simulations with elimination-aware refresh.

**Hybrid** (`search/hybrid.rs`): BRS phase → MCTS phase. BRS provides tactical filtering, MCTS provides strategic search.

**Constructor signatures (from Stage 16):**
```rust
BrsSearcher::new(evaluator: Box<dyn Evaluator>, nnue_weights: Option<Arc<NnueWeights>>)
MctsSearcher::new(evaluator: Box<dyn Evaluator>, nnue_weights: Option<Arc<NnueWeights>>)
HybridController::new(profile: EvalProfile, nnue_path: Option<&str>)
```

### Board Starting Position (board/board_struct.rs)

`Board::starting_position()` is hardcoded — Red bottom (rows 0-1), Blue left (columns 0-1), Yellow top (rows 13-12), Green right (columns 13-12). Each player has R N B Q K B N R on their back rank with 8 pawns in front.

Castling is bitmask-based: `castling_rights: u8` with 2 bits per player (kingside + queenside).

### Variants Module (src/variants/mod.rs)

Currently just a stub:
```rust
// Game mode variant tuning — Stage 17
```

---

## Build Order

### Step 1: Chess960 Position Generator

**New file:** `src/variants/chess960.rs`

Build a Chess960 starting position generator for 4-player chess:

1. **Generate a valid random back rank** following Chess960 rules:
   - Bishops on opposite-colored squares
   - King between the two rooks
   - 8 pieces: R, N, B, Q, K, B, N, R (any permutation satisfying constraints)

2. **Rotate for all 4 players.** The 14x14 board has 4 symmetric home zones. The same logical arrangement must be mapped to each player's back rank:
   - Red: d1-k1 (row 0, columns 3-10)
   - Blue: a4-a11 (column 0, rows 3-10)
   - Yellow: d14-k14 (row 13, columns 3-10, mirrored)
   - Green: n4-n11 (column 13, rows 3-10, mirrored)

3. **Update castling rights.** In Chess960, castling rights are identified by the rook's file position (not fixed king/queen side). Store which file each rook starts on for each player.

4. **Provide a `Board::chess960_position(seed: u64)` constructor** that generates a random Chess960 starting position. Keep `Board::starting_position()` unchanged.

5. **Add a protocol option:** `setoption name Chess960 value true/false`. When true, `position startpos` uses a random Chess960 arrangement.

**Note on Chess960 castling:** The standard castling destination squares are the same as in regular chess (king goes to c-file/g-file equivalent, rook goes to d-file/f-file equivalent), but the king and rook start on different squares. The `is_castle()` move flag and castling execution in `make_move` / `unmake_move` must work correctly for arbitrary king/rook starting positions. Review the existing castling implementation carefully — if it hardcodes starting squares, it will need adjustment. If it's already flexible (using the castling rights to find rook positions), it may work as-is.

### Step 2: DKW Chance Nodes in MCTS

**The key challenge.** When MCTS simulates a move and the resulting game state has DKW players, the DKW random moves create branching uncertainty. Currently, `gs.apply_move()` internally calls `process_dkw_moves()` which makes one random selection — but the search doesn't account for other possible DKW outcomes.

**Approach: Expectimax averaging in MCTS simulations.**

During MCTS simulation, after `apply_move()`:
1. Check if the `MoveResult` contains DKW moves (`!result.dkw_moves.is_empty()`)
2. If yes, this is a **chance node**. Instead of evaluating the single DKW outcome:
   - Sample N (e.g., 3-5) different DKW move sequences for this position
   - Average the leaf evaluations across all samples
   - This approximates the expected value over DKW randomness

**Implementation approach:**
- Add a `dkw_sample_count: usize` parameter (default 3) to MCTS config
- At chance nodes during simulation, clone the pre-DKW state, apply different random seeds, and average
- If no DKW players exist, this adds zero overhead (the common case)

**Alternative (simpler):** Just ignore DKW randomness and treat the single random DKW outcome as representative. DKW kings are typically weak and their random moves rarely change the position evaluation significantly. This is already what happens. **If you choose this simpler approach, document WHY in a W-note and move on.** The implementation effort may not be worth the playing strength gain.

### Step 3: DKW Awareness in BRS

**Simpler than MCTS.** In BRS alpha-beta search:
- DKW kings are obstacles. The eval should treat DKW pieces differently:
  - Dead pieces near the king are potential shields (can't be captured for points)
  - A wandering DKW king can block important squares
- Consider adding a small eval term: `dkw_king_proximity_penalty` — if a DKW king is near the player's own king, it adds uncertainty (random moves might interfere with the player's plans)

**Optional:** In the move ordering phase, deprioritize capturing Dead pieces (they're worth 0 points in FFA). Currently `capture_points()` already returns 0 for Dead/Terrain, but the MVV-LVA move ordering might still rank them highly based on piece type.

### Step 4: FFA Scoring Strategy in Eval

Enhance the bootstrap eval's FFA awareness:

1. **Point-per-move efficiency.** In FFA, a queen capture (9 points) is far more valuable than a pawn capture (1 point). The eval already handles this via material values, but add a term that considers:
   - "Capturable opponent pieces" — pieces that are hanging or poorly defended, weighted by their FFA point value (not just material centipawn value, which differs)
   - This is a lightweight static analysis, not a search extension

2. **Self-stalemate detection.** If a player is losing badly (material deficit > threshold), actively seek stalemate positions (20 points). Add an eval term:
   - When material is very low AND behind, bonus for positions where legal moves are decreasing
   - This is aspirational — implement only if it doesn't complicate the eval significantly

3. **Claim-win proximity.** When point lead approaches 21, increase urgency:
   - Player close to claiming: bonus for maintaining lead, penalty for trades that reduce point differential
   - Opponent close to claiming: bonus for capturing that opponent's pieces, penalty for ignoring the threat

**Important:** These eval terms only activate in `GameMode::FreeForAll`. In LKS, FFA points don't determine the winner.

### Step 5: Terrain-Aware Evaluation

Add terrain geometry assessment to the bootstrap eval:

1. **Terrain fortress detection.** Pieces sheltered behind terrain walls (terrain between the piece and opponents' attack vectors) get a small bonus. Terrain creates "safe zones" where pieces are naturally protected.

2. **Terrain mobility penalty.** Pieces surrounded by terrain on multiple sides have reduced mobility. Penalize pieces that can't move due to terrain blocking.

3. **King safety near terrain.** Terrain adjacent to the king acts like a permanent wall:
   - Terrain on king's flanks reduces attack surface → king safety bonus
   - BUT terrain behind the king can trap it → king safety penalty if escape routes are limited

4. **Outpost detection near terrain.** Knights/bishops on squares adjacent to terrain have enhanced stability (terrain can't be pushed/exchanged). Small bonus for pieces positioned next to terrain walls.

**Important:** These eval terms only activate when `terrain_mode == true` on the `GameState`. When terrain is off, skip all terrain eval work.

**Performance concern:** Terrain eval must be fast. Use simple neighbor checks (8 adjacent squares), not deep analysis. These are heuristic bonuses/penalties, not exact calculations.

### Step 6: Mode-Specific Parameter Tuning

Different game modes may benefit from different search parameters:

1. **EvalProfile expansion.** Add new profiles or extend the existing `EvalWeights`:
   ```rust
   pub struct EvalWeights {
       // Existing
       pub ffa_point_weight: i16,
       pub lead_penalty_enabled: bool,
       pub lead_penalty_divisor: i16,
       pub max_lead_penalty: i16,
       // New: terrain bonuses
       pub terrain_fortress_bonus: i16,
       pub terrain_mobility_penalty: i16,
       pub terrain_king_safety_bonus: i16,
       // New: DKW awareness
       pub dkw_proximity_penalty: i16,
       // New: FFA claim-win urgency
       pub claim_win_urgency_bonus: i16,
   }
   ```

2. **Validate via self-play.** Use the Stage 12 observer infrastructure (`observer/match.mjs`):
   - Run FFA games: compare default vs. tuned FFA weights
   - Run LKS games: compare default vs. tuned LKS weights
   - Run Terrain games: compare terrain-aware vs. terrain-naive
   - Minimum 50 games per comparison

3. **If tuning doesn't help, revert.** The default weights should be safe fallbacks. Only keep tuned values if they demonstrably improve play.

---

## Critical Warnings from Previous Stages

### W23 (Stage 16): Opponent move selection uses BootstrapEvaluator
The free functions `select_best_opponent_reply`, `select_hybrid_reply`, `pick_objectively_strongest` in BRS use `BootstrapEvaluator`, not NNUE. Any eval improvements you make to `BootstrapEvaluator` will affect opponent move selection in BRS.

### W25 (Stage 16): Constructor signatures changed
`BrsSearcher::new()`, `MctsSearcher::new()`, `HybridController::new()` all take `nnue_weights`/`nnue_path` parameter. Keep passing these correctly.

### Turn order R→B→Y→G (ADR-012)
Never alter `side_to_move` outside make/unmake. DKW moves follow turn order starting from `current_player.next()`.

### Evaluator trait is FROZEN
Do not change `eval_scalar(&self, &GameState, Player) -> i16` or `eval_4vec(&self, &GameState) -> [f64; 4]`. The BootstrapEvaluator implementation can change (add new eval terms), but the trait signatures cannot.

### Searcher trait is FROZEN
`search(&mut self, &GameState, SearchBudget) -> SearchResult`

### perft invariants are permanent
perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050. Chess960 uses a different starting position so perft values will differ — **only assert standard perft on the standard starting position.**

---

## Acceptance Criteria (Tests Required)

### Chess960 Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T1 | `test_chess960_valid_position` | Generated Chess960 positions have bishops on opposite colors, king between rooks |
| T2 | `test_chess960_all_players_symmetric` | All 4 players have the same logical piece arrangement |
| T3 | `test_chess960_deterministic_seed` | Same seed produces same position |
| T4 | `test_chess960_different_seeds_differ` | Different seeds produce different positions (at least some of the time) |
| T5 | `test_chess960_castling_legal` | Castling works correctly from non-standard king/rook positions |

### DKW Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T6 | `test_dkw_eval_penalty` | DKW kings near own king produce a different eval than DKW kings far away |
| T7 | `test_dead_piece_capture_ordering` | Dead piece captures are not prioritized over alive piece captures in move ordering |

### FFA Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T8 | `test_ffa_claim_win_urgency` | Eval reflects urgency when point lead approaches 21 |
| T9 | `test_ffa_eval_mode_gated` | FFA-specific eval terms return 0 in LKS mode |

### Terrain Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T10 | `test_terrain_fortress_bonus` | Pieces behind terrain walls get higher eval than exposed pieces |
| T11 | `test_terrain_eval_mode_gated` | Terrain eval terms return 0 when terrain_mode is false |
| T12 | `test_terrain_king_safety` | King adjacent to terrain walls gets modified safety score |

### Integration Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T13 | `test_ffa_game_completion` | Full FFA game runs to completion with correct scoring |
| T14 | `test_lks_terrain_game_completion` | LKS+Terrain game runs to completion |
| T15 | `test_chess960_game_runs` | Engine can play a game from Chess960 starting position |
| T16 | `test_perft_standard_unchanged` | Standard starting position perft invariants preserved |

### Regression Tests

| ID | Test | What it verifies |
|----|------|-----------------|
| T17 | `test_no_regression_standard_ffa` | Self-play in standard FFA: new engine doesn't lose to old engine |
| T18 | `test_dkw_game_no_panic` | Full game with DKW player completes without panic |

---

## Critical Invariants — DO NOT VIOLATE

1. **Evaluator trait is FROZEN.** `eval_scalar(&self, &GameState, Player) -> i16` and `eval_4vec(&self, &GameState) -> [f64; 4]` cannot change.
2. **Searcher trait is FROZEN.** `search(&mut self, &GameState, SearchBudget) -> SearchResult` cannot change.
3. **perft invariants** on standard starting position: perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050.
4. **Turn order R→B→Y→G.** Never alter `side_to_move` outside make/unmake.
5. **DKW moves are permanent.** `process_dkw_moves()` uses `make_move` without storing undo. Do not try to unmake DKW moves.
6. **Terrain pieces are immovable and uncapturable.** Movegen and attack detection already enforce this.
7. **NNUE accumulator push/pop ordering.** If you touch BRS/MCTS search paths, maintain the push-before-make, pop-after-unmake invariant.
8. **`unmake_move` takes 3 args:** `unmake_move(board, mv, undo)`.
9. **TT probe MUST come AFTER repetition check in alphabeta().**
10. **NNUE weights are `Option<Arc<NnueWeights>>`** — always propagate correctly through constructors.

---

## Scope Boundaries — What NOT To Build

- No new game modes beyond what exists (FFA, LKS). No teams mode.
- No NNUE retraining or architecture changes (frozen from Stage 14).
- No SIMD optimization (Stage 19).
- No UI changes (separate from engine).
- No distributed search or threading.
- No changes to the `.onnue` file format.
- Don't modify `GameMode` enum unless strictly necessary for Chess960 (prefer a separate `chess960: bool` flag like `terrain_mode`).

---

## Pre-Audit Checklist

1. `cargo build && cargo test` — verify all existing 536 + new tests pass
2. `cargo clippy` — verify 0 warnings
3. Create `masterplan/audit_log_stage_17.md` with pre-audit section

## Post-Audit Checklist

1. `cargo test` — all existing + new tests pass
2. `cargo clippy` — 0 warnings
3. Fill post-audit section of `audit_log_stage_17.md`
4. Fill `masterplan/downstream_log_stage_17.md`
5. Update `masterplan/STATUS.md` and `masterplan/HANDOFF.md`
6. Create session note in `masterplan/sessions/`
