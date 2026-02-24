# Stage 8: BRS/Paranoid Hybrid Layer + GameMode/EvalProfile — Build Order

**Approved:** 2026-02-23
**Status:** Step 0 complete; Step 0b next

---

## Context

Stage 8 adds hybrid opponent modeling on top of plain BRS. The user identified a fundamental problem: BRS + lead penalty makes Odin avoid accumulating points — correct for Last King Standing but wrong for FFA. Rather than bolt GameMode/EvalProfile on separately, we integrate it as Stage 8's foundation. ADR-013 already says Stage 8 is where "scoring context enters the board scanner." The game mode and eval profile ARE that scoring context.

**Two independent axes:**
- **GameMode** (rules layer): FFA vs LKS — affects win conditions, claim-win gating
- **EvalProfile** (eval personality): Standard vs Aggressive — affects lead penalty, FFA point weight

These are orthogonal: you can run Aggressive profile in LKS mode for comparative testing.

---

## Critical Upstream Notes

- **W5 (Stale GameState fields during search):** `player_status` and `scores` are NOT updated during make/unmake. Board scanner runs pre-search (accurate data). Eval during search reads root-position scores/statuses. This is acceptable for now — same as Stage 7.
- **W4 (Lead penalty tactical mismatch):** Engine sometimes prefers check over free queen capture. The Aggressive profile (lead penalty OFF) fixes this for FFA. Tactical suite `[unverified]` positions need resolution.
- **API contracts:** Evaluator trait signature is permanent (no changes). Searcher trait signature is permanent. BootstrapEvaluator can gain internal state — trait takes `&self`.
- **Performance baselines:** BRS depth 6 release = 109ms/10,916 nodes. Eval < 10us/call. All 302 existing tests must pass.

---

## Build Order (10 steps)

### Step 0: GameMode + EvalProfile Foundation -- COMPLETE

**Files modified:**

**`odin-engine/src/gamestate/mod.rs`**
- Expanded `GameMode`: added `LastKingStanding` variant
- Removed `#[allow(dead_code)]` on `GameState` struct
- Added `pub fn game_mode(&self) -> GameMode` accessor
- Added convenience constructors: `new_standard_lks()`, `new_standard_lks_terrain()`
- Gated `check_claim_win` call behind `self.game_mode == GameMode::FreeForAll`
- Rewrote `end_game()`: FFA winner = highest score; LKS winner = last standing

**`odin-engine/src/eval/mod.rs`**
- Defined `EvalProfile` enum: `Standard`, `Aggressive` (`Copy, Clone, Debug, PartialEq, Eq`)
- Defined `EvalWeights` struct (returned by `EvalProfile::weights()`):
  ```
  ffa_point_weight: i16       // Standard=50, Aggressive=120
  lead_penalty_enabled: bool  // Standard=true, Aggressive=false
  lead_penalty_divisor: i16   // Standard=4
  max_lead_penalty: i16       // Standard=150
  ```
- Changed `BootstrapEvaluator` from unit struct to `{ weights: EvalWeights }`
- `BootstrapEvaluator::new(profile: EvalProfile)` — takes profile as arg
- `eval_for_player` passes `weights: &EvalWeights` to multi_player functions

**`odin-engine/src/eval/multi_player.rs`**
- `lead_penalty()`: added `enabled: bool, divisor: i16, max_penalty: i16` params. Returns 0 when `!enabled`.
- `ffa_points_eval()`: added `weight: i16` param (replaces `FFA_POINT_WEIGHT` constant)
- Old module constants inlined into test helpers
- All existing unit tests pass Standard-profile values (same behavior)

**`odin-engine/src/protocol/types.rs`**
- Added to `EngineOptions`:
  ```rust
  pub game_mode: GameMode,               // default: FreeForAll
  pub eval_profile: Option<EvalProfile>,  // default: None (auto-resolve)
  ```
- Added `resolved_eval_profile(&self) -> EvalProfile`: None + FFA -> Aggressive, None + LKS -> Standard, Some(p) -> p

**`odin-engine/src/protocol/mod.rs`**
- `handle_setoption`: new match arms for `"gamemode"` (ffa/lks) and `"evalprofile"` (standard/aggressive/auto)
- `handle_position_fen4`: uses `self.options.game_mode` instead of hardcoded `GameMode::FreeForAll`
- `handle_position_startpos`: branches on `game_mode` + `terrain_mode` for constructor selection
- `handle_go`: `BootstrapEvaluator::new(self.options.resolved_eval_profile())`

**All `BootstrapEvaluator::new()` call sites updated** to pass a profile:
- `search/brs.rs` test helper
- `tests/stage_06_eval.rs` (9 call sites)
- `tests/stage_07_brs.rs` (1 call site)

**Tests added (11 new unit tests):**
- `test_aggressive_no_lead_penalty`, `test_aggressive_higher_ffa_weight`, `test_standard_has_lead_penalty`
- `test_aggressive_profile_valid_scores`, `test_eval_weights_debug`, `test_standard_profile_matches_original_behavior`
- `test_lead_penalty_disabled_returns_zero`, `test_ffa_points_eval_aggressive_weight`
- `test_resolved_profile_auto_ffa`, `test_resolved_profile_auto_lks`, `test_resolved_profile_explicit_override`

**Result:** 316 tests pass (210 unit + 106 integration), 0 warnings.

**ADR to record:** ADR-014: GameMode/EvalProfile Separation. Game rules (win conditions) are independent from eval personality (how aggressively the engine plays). Default pairing: FFA->Aggressive, LKS->Standard. Override via `setoption name evalprofile`.

---

### Step 0b: UI — Game Settings Controls + Config Tags

**Selection hierarchy (descending order):**

```
1. GAME MODE    (FFA / LKS)           <- determines rules
2. EVAL PROFILE (Auto / Std / Aggro)  <- resolves based on #1 when Auto
3. TERRAIN      (On / Off toggle)     <- orthogonal to above
4. [New Game]   button                <- sends all, starts game
```

**How selections gate each other:**
- Game Mode is always selectable (default: FFA)
- Eval Profile is always selectable (default: Auto). When Auto is selected, the button label shows resolved value: `Auto (Aggressive)` for FFA, `Auto (Standard)` for LKS. Switching Game Mode updates the Auto label in real-time before clicking New Game.
- Terrain toggle is always available (default: Off)
- All combos are valid (no lockout) — cross-mode testing is intentional
- New Game button sends all settings then starts

**Command ordering (enforced in `newGame()`):**
```
setoption name GameMode value ffa|lks        <- rules first
setoption name EvalProfile value auto|...    <- personality second
setoption name Terrain value true|false      <- terrain third
position startpos                            <- then position
isready                                      <- then sync
```

**Engine-side guarantees (no confusion possible):**
- `EngineOptions` stores all three settings as typed enums/bools, not raw strings
- `resolved_eval_profile()` always returns a concrete `EvalProfile` — no `None` reaches the evaluator
- `handle_go` creates `BootstrapEvaluator::new(resolved_profile)` fresh per search — profile is baked into the evaluator instance, not checked dynamically during eval
- `handle_position_*` reads `game_mode` from options — `GameState` is constructed with the correct mode
- Claim-win gate checks `self.game_mode` structurally at runtime — LKS can never trigger claim-win

**Config tags (visible in UI):**
- Row of small colored badge pills displayed below the scores section
- Shows the ACTIVE resolved config: `[FFA] [Aggressive] [Normal]` or `[LKS] [Standard] [Terrain]`
- Tags update immediately when selections change (before New Game) to show what WILL be used
- Tags serve double duty: user clarity now + self-play data tagging later

**Files modified:**

**`odin-ui/src/hooks/useGameState.ts`**
- Add state: `gameMode: 'ffa' | 'lks'` (default `'ffa'`), `evalProfile: 'auto' | 'standard' | 'aggressive'` (default `'auto'`), `terrainMode: boolean` (default `false`)
- Add refs for async access (same pattern as `engineDelayRef`)
- Add setters exported in `UseGameStateResult`: `setGameMode`, `setEvalProfile`, `setTerrainMode`
- Add computed `resolvedEvalProfile`: auto + ffa -> `'aggressive'`, auto + lks -> `'standard'`, else explicit
- Replace `newGame(terrain: boolean)` signature with `newGame()` — reads from state
- In `newGame()`, send setoption commands in strict order: gamemode -> evalprofile -> terrain -> position startpos -> isready

**`odin-ui/src/components/GameControls.tsx`**
- Update `GameControlsProps`: drop `onNewGame: (terrain: boolean) => void`, add:
  - `gameMode`, `evalProfile`, `resolvedEvalProfile`, `terrainMode`
  - `onSetGameMode`, `onSetEvalProfile`, `onSetTerrainMode`, `onNewGame` (no args)
- Add three new `control-section` blocks between Scores and Play Mode:
  1. **GAME MODE** section — two toggle buttons: `FFA` / `LKS` (reuse `.btn-mode` pattern)
  2. **EVAL PROFILE** section — three toggle buttons: `Auto (Aggressive)` / `Standard` / `Aggressive` (Auto label dynamically shows resolved value)
  3. **TERRAIN** section — two toggle buttons: `Off` / `On` (same `.btn-mode` style)
- Add config tags row below scores: `.config-tags` div with `.config-tag` pill badges
- Replace two New Game buttons with single `New Game` button

**`odin-ui/src/styles/GameControls.css`**
- Add `.config-tags` (flex row, gap, margin)
- Add `.config-tag` (small pill: `font-size: 10px`, `padding: 2px 6px`, `border-radius: 8px`, colored background per type — blue for mode, green for profile, orange for terrain)
- Reuse existing `.mode-selector` and `.btn-mode` patterns for all three selectors

**`odin-ui/src/App.tsx`**
- Thread new state/setter props from `useGameState` -> `GameControls`
- Update `onNewGame` prop (no boolean parameter)

---

### Step 1: Board Scanner (pre-search analysis)

Per MASTERPLAN spec. Produces `BoardContext` struct. Runs once before search, < 1ms.

**Key integration with GameMode/EvalProfile:**
- `BoardContext` includes a `game_mode: GameMode` field so downstream hybrid scoring knows the mode
- FFA mode: `best_target` for each opponent considers capture opportunity value (FFA points), not just king vulnerability
- LKS mode: `best_target` considers king exposure and elimination potential
- Point standings (`scores` from GameState) feed into opponent aggression assessment — trailing opponents in FFA are more aggressive toward leaders

**New file:** `odin-engine/src/search/board_scanner.rs`

**Struct (per MASTERPLAN):**
```rust
pub struct BoardContext {
    game_mode: GameMode,
    weakest_player: Player,
    most_dangerous: [Player; 3],
    root_danger_level: f64,
    high_value_targets: [(Square, Player); 8],
    high_value_target_count: u8,
    convergence: Option<(Player, Player, Player)>,
    per_opponent: [OpponentProfile; 3],
}

pub struct OpponentProfile {
    player: Player,
    aggression_toward_root: f64,
    own_vulnerability: f64,
    best_target: Player,
    can_afford_to_attack_root: bool,
    supporting_attack_on_root: bool,
}
```

**Tests:** hand-verify output on 5+ test positions. Verify < 1ms in release.

---

### Step 2: Move Classifier (cheap filter at opponent nodes)

Per MASTERPLAN spec. Classifies each opponent move as "relevant" (captures root pieces, checks root king, lands near root king) or "background." Table lookups using attack query API.

**No mode-specific behavior** — classification is purely tactical.

**Tests:** Verify classification on known positions. Count relevant vs background.

---

### Step 3: Hybrid Reply Scoring

Per MASTERPLAN spec. Replaces `select_best_opponent_reply` with hybrid scoring on relevant moves.

```
score = harm_to_root * likelihood + objective_strength * (1 - likelihood)
```

**Mode-specific likelihood adjustments:**
- FFA: Lower base likelihood for opponents targeting root when they have better scoring opportunities elsewhere. If opponent has a 9-point queen capture from another player, likelihood of them wasting a move on root drops.
- LKS: Higher base likelihood — survival instinct means opponents DO gang up on the leader more consistently.
- The `EvalProfile` affects `objective_strength` calculation (via the evaluator), so Aggressive profile inherently values captures higher.

**Tests:** Compare hybrid vs plain BRS on tactical suite positions.

---

### Step 4: Progressive Narrowing

Per MASTERPLAN spec. Depth schedule:
```
depth 1-3: top 8-10 candidates
depth 4-6: top 5-6
depth 7+:  top 3
```

Plus background fallback (strongest single move from non-relevant moves).

**Tests:** Measure node count reduction at depth 6+.

---

### Step 5: Delta Updater (optional for v1)

Patches board scanner every 2 plies instead of full re-read. Compare against full re-read for accuracy.

---

### Step 6: Tactical Suite + A/B Comparison

Per MASTERPLAN acceptance criteria:
- Expand tactical suite to 20+ positions
- Resolve `[unverified]` mate positions
- Run suite with hybrid BRS (Stage 8) vs plain BRS (v1.7 tag)
- Hybrid must find best move in >= as many positions
- Record results in audit log

**Additional:** Run suite with both Standard and Aggressive profiles to measure aggression impact.

---

### Step 7: Smoke-Play Validation

Per MASTERPLAN acceptance criteria:
- 5 games, engine controls Red, opponents random, 20 moves each
- Inspect first 5 moves per game
- Engine must develop, capture hanging material, avoid king danger

**Additional:** Run smoke-play in both FFA and LKS modes.

---

### Step 8: Tracing Instrumentation

Add `tracing::debug!` calls at key observation points throughout the Stage 8 code:
- Board context output (per-opponent profiles, danger levels)
- Cheap filter results (relevant vs background move counts)
- Reply scoring breakdown (per-move objective strength, harm, likelihood)
- Progressive narrowing (depth, candidate count, truncation)

These replace the Huginn gates originally specified. The `tracing` crate was adopted as a simpler, working alternative (see ADR-015).

---

### Step 9: Audit + Documentation

- Pre-audit and post-audit per AGENT_CONDUCT 1.1 / 1.5
- ADR-014: GameMode/EvalProfile Separation
- Update STATUS.md, HANDOFF.md
- Create vault notes: Component-BoardScanner, Component-EvalProfile, Connection-GameMode-to-Eval
- Session note in `masterplan/sessions/`
- Downstream log for Stage 8

---

## Files Summary

| File | Action | Purpose |
|------|--------|---------|
| **Engine** | | |
| `gamestate/mod.rs` | modified (Step 0) | GameMode::LKS, game_mode() accessor, claim-win gate, end_game mode logic |
| `eval/mod.rs` | modified (Step 0) | EvalProfile, EvalWeights, BootstrapEvaluator config |
| `eval/multi_player.rs` | modified (Step 0) | Parameterize lead_penalty + ffa_points_eval |
| `protocol/types.rs` | modified (Step 0) | EngineOptions: game_mode, eval_profile, resolved_eval_profile() |
| `protocol/mod.rs` | modified (Step 0) | setoption parsing, position/go plumbing |
| `search/board_scanner.rs` | **new** (Step 1) | BoardContext, OpponentProfile, pre-search scan |
| `search/mod.rs` | modify (Step 1) | Export board_scanner module |
| `search/brs.rs` | modify (Step 3-4) | Integrate hybrid scoring, classifier, progressive narrowing |
| `tests/stage_08_brs_hybrid.rs` | **new** (Step 6) | Integration tests |
| `tests/positions/tactical_suite.txt` | modify (Step 6) | Expand to 20+ positions, fix unverified |
| **UI** | | |
| `hooks/useGameState.ts` | modify (Step 0b) | gameMode/evalProfile/terrainMode state, newGame() rewrite |
| `components/GameControls.tsx` | modify (Step 0b) | Mode/Profile/Terrain selectors, config tags, single New Game button |
| `styles/GameControls.css` | modify (Step 0b) | Config tag pills, selector styles |
| `App.tsx` | modify (Step 0b) | Thread new props |
| **Docs** | | |
| `masterplan/DECISIONS.md` | modify (Step 9) | ADR-014 |
| `masterplan/audit_log_stage_08.md` | **new** (Step 9) | Audit log |
| `masterplan/downstream_log_stage_08.md` | **new** (Step 9) | Downstream log |

---

## Verification

**Engine (cargo):**

1. `cargo build` — passes
2. `cargo test` — all 302+ existing tests pass (zero regressions)
3. `cargo clippy` — no warnings
4. New unit tests for GameMode, EvalProfile, multi_player params
5. New integration tests for hybrid BRS, mode switching, profile switching
6. Tactical suite A/B: hybrid >= plain BRS correct positions
7. Smoke-play: 5 games in FFA, 5 in LKS, manual inspection
8. Performance: board scanner < 1ms, overall search not more than 2x slower than v1.7 at same depth
9. Protocol round-trip: `setoption name gamemode value lks` -> `position startpos` -> `go depth 4` works correctly

**UI (npm/vitest):**
10. `npm test` in `odin-ui/` — all 54+ existing Vitest tests pass
11. Manual: Switch GameMode FFA -> LKS, verify Auto label changes from `(Aggressive)` to `(Standard)`
12. Manual: Click New Game, verify config tags show correct resolved values
13. Manual: Set LKS + Aggressive (cross-mode), click New Game, verify engine plays with no errors
14. Manual: Verify command ordering in CommunicationLog: gamemode -> evalprofile -> terrain -> position startpos -> isready
