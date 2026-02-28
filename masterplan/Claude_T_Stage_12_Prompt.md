# Stage 12: Self-Play & Regression Testing — Implementation Prompt

## Context

You are implementing Stage 12 of Project Odin, a 4-player chess engine.
The engine is at tag `stage-11-complete` / `v1.11` with 457 tests passing (281 unit + 176 integration, 4 ignored), 0 clippy warnings.

**What exists:**
- `odin-engine/` — Rust engine with BRS+MCTS hybrid search (`search/hybrid.rs`), bootstrap eval, Odin protocol
- `observer/` — Node.js self-play tool (`observer.mjs`) that spawns one engine instance, plays FFA games, logs moves + evals to JSON/markdown
- `observer/baselines/` — 6 human games from chess.com (2 strong 3000+ Elo, 3 weak ~2100 Elo, 1 engine v0.4.3) with structured JSON + markdown summaries
- Engine binary at `target/release/odin-engine.exe`

**What Stage 12 builds (from MASTERPLAN.md):**
1. Match manager — play N games between two engine versions, rotating colors, compute Elo difference
2. SPRT — Sequential Probability Ratio Test (H0: elo≤0, H1: elo≥5, alpha/beta=0.05)
3. Regression test suite — positions with known best moves or evaluation ranges
4. Data logging — store all self-play games for future NNUE training data (Stage 15)

**Acceptance criteria:**
- AC1: Match manager runs 1000+ games stably
- AC2: Elo calculations consistent with statistical theory
- AC3: SPRT correctly identifies improvements and rejections
- AC4: Regression tests catch known failure modes

---

## Step 0: Orientation (READ FIRST)

Read these files to understand the current architecture:

| File | Why |
|------|-----|
| `masterplan/STATUS.md` | Current project state |
| `masterplan/HANDOFF.md` | Last session context |
| `masterplan/AGENT_CONDUCT.md` Section 1.1 | Stage entry protocol |
| `observer/observer.mjs` | Existing self-play infrastructure — understand how it spawns engines, sends commands, parses responses |
| `observer/config.json` | Current observer config |
| `observer/baselines/README.md` | Human game baselines and Elo tier benchmarks |
| `odin-engine/src/protocol/mod.rs` | Odin protocol handler — understand command/response format |
| `odin-engine/tests/stage_07_brs.rs` | Existing test patterns (helpers, position builders, assertions) |
| `odin-engine/tests/stage_11_hybrid.rs` | Tactical test positions (free queen capture, single legal move) |

---

## Step 1: Pre-Audit

Fill the **pre-audit** section of `masterplan/audit_log_stage_12.md`:
- List all files you plan to create or modify
- Confirm acceptance criteria mapping
- Note any risks or open questions

---

## Step 2: Regression Test Suite (AC4)

**This is the highest-value deliverable.** Create tactical puzzle positions where the engine MUST find the right move.

### File: `odin-engine/tests/stage_12_regression.rs`

Build a regression test suite with positions that catch known failure modes. Each test:
1. Constructs a position programmatically (using `Board::empty()` + `place_piece()` + `GameState::new()`)
2. Runs the hybrid searcher at a reasonable depth/budget
3. Asserts the engine finds the correct move OR scores above a threshold

**Required regression positions (minimum 8):**

| # | Position | Expected | What it catches |
|---|----------|----------|-----------------|
| R1 | Free queen capture (existing: Red Qh7 takes Blue Qg8) | `score > 0`, finds capture | Basic tactics |
| R2 | Hanging bishop: piece on a square attacked by opponent pawn | Doesn't play bishop there | Horizon effect (walking into captures) |
| R3 | Defended piece vs undefended: two captures available, one is defended | Prefers undefended target | SEE / capture ordering |
| R4 | Fork position: knight can fork king + queen | Finds the fork move | Multi-target tactics |
| R5 | Pin: piece pinned to king, don't move the pinned piece | Doesn't move pinned piece | Pin awareness |
| R6 | Recapture: opponent just captured, engine should recapture | Finds recapture | Exchange sequences |
| R7 | Safe king: don't walk king into open file | King stays safe | King safety eval |
| R8 | Material trade when ahead: simplify when up material | Score stays high after trade | Strategic evaluation |

**Position construction pattern** (copy from stage_11_hybrid.rs):
```rust
fn make_regression_position_N() -> GameState {
    let mut board = Board::empty();
    board.set_side_to_move(Player::Red);
    board.set_castling_rights(0);
    // Place pieces...
    // ALWAYS place all 4 kings (required for valid 4-player position)
    // Use square_from(file, rank) — remember 14×14 board, corners are invalid
    GameState::new(board, GameMode::FreeForAll, false)
}
```

**IMPORTANT constraints:**
- All 4 kings must be present in every position
- Avoid invalid corner squares (see `board.rs` — files 0-2 with ranks 0-2, etc.)
- Use `depth_budget(8)` for tactical positions (2 full rotations in 4-player BRS)
- Assert `score > 0` or `score > threshold` rather than asserting specific moves (the engine may find equally good alternatives)
- Use `#[ignore]` on positions the engine can't currently solve — these become aspirational targets

**Test helpers to reuse:**
```rust
fn make_hybrid() -> HybridController {
    HybridController::new(EvalProfile::Standard)
}

fn depth_budget(depth: u8) -> SearchBudget {
    SearchBudget { max_depth: Some(depth), max_nodes: None, max_time_ms: None }
}

fn assert_legal(gs: &GameState, mv: Move) {
    let mut gs_check = gs.clone();
    let legal = gs_check.legal_moves();
    assert!(legal.contains(&mv), "move {} is not legal", mv.to_algebraic());
}
```

---

## Step 3: Match Manager (AC1)

### File: `observer/match.mjs`

Extend the observer infrastructure with a match manager that plays games between two engine binaries.

**Architecture:**
- Takes two engine paths as arguments (engine A vs engine B)
- Plays N games, rotating which engine gets which color seat
- In 4-player FFA: each game has 4 seats. Alternate which seats engine A vs B occupy across games
- Records results: winner, final scores per player, move count

**Rotation scheme for 4-player:**
Each game assigns seats [Red, Blue, Yellow, Green]. With 2 engines, rotate:
- Game 1: A=Red,Blue  B=Yellow,Green
- Game 2: A=Yellow,Green  B=Red,Blue
- Game 3: A=Red,Yellow  B=Blue,Green
- Game 4: A=Blue,Green  B=Red,Yellow
- ... repeat

This ensures balanced color exposure.

**Config:** `observer/match_config.json`
```json
{
  "engine_a": "../target/release/odin-engine.exe",
  "engine_b": "../target/release/odin-engine-baseline.exe",
  "games": 100,
  "depth": 6,
  "game_mode": "ffa",
  "eval_profile": "standard",
  "stop_at": { "max_ply": 200, "on_gameover": true },
  "output_dir": "./match_reports"
}
```

**Usage:** `node match.mjs` or `node match.mjs --config match_config.json`

**Key implementation detail:** The match manager needs to run TWO engine processes simultaneously per game (one for engine A's seats, one for engine B's seats). When it's engine A's turn, send the position to engine A; when it's engine B's turn, send to engine B. Both engines maintain their own state.

**Protocol flow per move:**
1. Determine whose turn it is (side_to_move)
2. Determine which engine controls that seat (A or B)
3. Send `position startpos moves ...` + `go depth N` to that engine
4. Wait for `bestmove`
5. Apply the move and continue

**Output:** Per-game JSON + overall summary with win/draw/loss counts per engine.

---

## Step 4: Elo Calculation (AC2)

### File: `observer/elo.mjs`

Implement Elo difference calculation from match results.

**For 4-player FFA**, scoring is point-based (not win/loss). Use the following approach:

1. Each game produces 4 final scores (one per seat).
2. Engine A's result = average of its seats' scores. Engine B's result = average of its seats' scores.
3. Normalize: if A_avg > B_avg, count as a "win" for A (score=1). If equal, draw (score=0.5). If less, loss (score=0).
4. Aggregate across games: `actual_score = wins / total_games`
5. Elo difference: `delta_elo = -400 * log10(1/actual_score - 1)`

**Also implement** the inverse: `expected_score(elo_diff) = 1 / (1 + 10^(-elo_diff/400))`

**Confidence interval:** Use normal approximation:
- `variance = actual_score * (1 - actual_score) / N`
- 95% CI: `actual_score ± 1.96 * sqrt(variance)`
- Convert CI bounds to Elo

**Export:** `function calculateElo(results) -> { elo_diff, ci_low, ci_high, wins, losses, draws, n }`

---

## Step 5: SPRT Implementation (AC3)

### File: `observer/sprt.mjs`

Implement Sequential Probability Ratio Test for early stopping.

**Parameters:**
- H0: elo_diff ≤ 0 (new engine is not better)
- H1: elo_diff ≥ 5 (new engine is at least 5 Elo better)
- alpha = 0.05 (false positive rate)
- beta = 0.05 (false negative rate)

**Algorithm:**
1. After each game, compute the Log-Likelihood Ratio (LLR):
   ```
   LLR = sum over games of: log(L(result | H1) / L(result | H0))
   ```
   Where `L(result | H) = expected_score(H)^wins * (1 - expected_score(H))^losses`

2. Bounds:
   - Lower bound: `log(beta / (1 - alpha))`
   - Upper bound: `log((1 - beta) / alpha)`

3. Decision:
   - If LLR ≥ upper bound → Accept H1 (new engine IS better), stop
   - If LLR ≤ lower bound → Accept H0 (new engine is NOT better), stop
   - Otherwise → continue playing games

**Integration with match manager:** After each game completes, run SPRT check. If SPRT triggers, stop the match early and report the result.

---

## Step 6: Data Logging for NNUE Training (AC1 partial)

### Output format: `observer/match_reports/game_NNNN.json`

Each game should log structured data suitable for NNUE training:

```json
{
  "game_id": 1,
  "engine_a": "v1.11",
  "engine_b": "v1.10",
  "seat_assignment": { "Red": "A", "Blue": "B", "Yellow": "A", "Green": "B" },
  "result": { "Red": 45, "Blue": 22, "Yellow": 38, "Green": 10 },
  "winner": "Red",
  "moves": [
    {
      "ply": 0,
      "player": "Red",
      "engine": "A",
      "move": "f2f4",
      "score_cp": 4488,
      "depth": 8,
      "nodes": 1992,
      "time_ms": 4036,
      "position_fen": "..."
    }
  ]
}
```

**Position encoding:** Since there's no standard 4-player FEN, use the engine's internal `position startpos moves ...` command string. Store the full move list up to that point so positions can be reconstructed.

---

## Step 7: Integration — Automated Pipeline

### File: `observer/run_match.sh` (or `.bat` for Windows)

A convenience script that:
1. Builds the current engine (`cargo build --release`)
2. Copies the old baseline binary (if it exists)
3. Runs the match manager
4. Reports Elo + SPRT result
5. If SPRT accepts H1: "New engine is better by ~N Elo"
6. If SPRT accepts H0: "No improvement detected"

---

## Build Order Summary

| Step | What | Files | AC |
|------|------|-------|-----|
| 1 | Pre-audit | `masterplan/audit_log_stage_12.md` | — |
| 2 | Regression test suite | `odin-engine/tests/stage_12_regression.rs` | AC4 |
| 3 | Match manager | `observer/match.mjs`, `observer/match_config.json` | AC1 |
| 4 | Elo calculation | `observer/elo.mjs` | AC2 |
| 5 | SPRT | `observer/sprt.mjs` | AC3 |
| 6 | Data logging | Integrated into match.mjs output | AC1 |
| 7 | Pipeline script | `observer/run_match.sh` | — |
| 8 | Post-audit + docs | audit log, downstream log, STATUS.md, HANDOFF.md | — |

---

## Scope Boundaries — DO NOT CHANGE

- **DO NOT** modify any engine search code (hybrid.rs, brs.rs, mcts.rs)
- **DO NOT** modify eval code
- **DO NOT** modify the protocol handler
- **DO NOT** modify existing tests (stage_07, stage_09, stage_11)
- **DO NOT** add opening books or endgame tablebases (masterplan explicitly says "you don't need these")
- The regression tests are READ-ONLY on engine internals — they test via the public `Searcher` trait interface

---

## Critical Invariants

1. **457 existing tests must still pass.** Run `cargo test` before and after.
2. **The `Searcher` trait is FROZEN.** `search(&mut self, &GameState, SearchBudget) -> SearchResult`
3. **The `Evaluator` trait is FROZEN.** `eval_scalar(&self, &GameState, Player) -> i16`
4. **4-player board is 14×14 with invalid corners.** When constructing test positions, avoid files 0-2 / ranks 0-2 corners (see `Board::is_valid_square()`).
5. **Turn order: R→B→Y→G.** `Player::ALL = [Red, Blue, Yellow, Green]`.
6. **perft invariants:** perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050.

---

## Known Limitations (DO NOT try to fix these)

The engine currently exhibits tactical blunders (walking pieces into captures) due to:
1. **Bootstrap eval weakness** — static eval can't fully assess material loss from limited MCTS simulations. Fixed by NNUE (Stages 14-16).
2. **MCTS with 2000 sims across many survivors** — thin simulation budget per candidate. Improves with time management (Stage 13).
3. **BRS 0cp spread** — BRS sometimes can't distinguish survivor moves. The hybrid passes them all to MCTS which may choose poorly.

These are known W10-W14 limitations documented in `masterplan/downstream_log_stage_11.md`. The regression test suite should document them (use `#[ignore]` for positions the engine can't solve yet) but NOT attempt to fix them.

---

## Verification Checklist

Before declaring Stage 12 complete:
- [ ] `cargo test` — all 457+ tests pass (original 457 + new regression tests)
- [ ] `cargo clippy` — 0 warnings
- [ ] Match manager runs 10+ games without crashing
- [ ] Elo calculation produces reasonable numbers (not NaN, not ±infinity)
- [ ] SPRT stops early when given a clearly better/worse engine (test with depth 4 vs depth 8)
- [ ] Regression tests document known-good and known-bad positions
- [ ] Game data logged in structured JSON format
- [ ] Post-audit in `masterplan/audit_log_stage_12.md`
- [ ] `masterplan/STATUS.md` and `masterplan/HANDOFF.md` updated
