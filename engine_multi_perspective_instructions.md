# Multi-Perspective Opponent Modeling — Implementation Instructions

**For:** Claude.T (terminal agent)
**Scope:** 3-term blend (paranoid + BRS + anti_leader). Vulture and convergent terms deferred.
**Primary file:** `odin-engine/src/search/board_scanner.rs`

## Before You Start

1. Read `masterplan/AGENT_CONDUCT.md` Section 1.1 (stage entry protocol).
2. Read `masterplan/HANDOFF.md` and `masterplan/STATUS.md` for current state.
3. Run `cargo test` — all 389 tests must pass before you touch anything.
4. Read `odin-engine/src/search/board_scanner.rs` in full — every change lives here.

## Background

The current hybrid reply scoring at MIN nodes uses a 2-term formula:

```
score = harm_to_root * likelihood + objective_strength * (1 - likelihood)
```

This is purely root-centric: "does this move hurt ME?" Analysis of human 4PC games (2400-2650 Elo) shows opponents also target the leader, pile on the weak, and exploit crossfire. The engine misses all of this.

We're replacing it with a 3-term multi-perspective blend:

```
score = w_paranoid    * harm_to_root
      + w_brs         * objective_strength
      + w_anti_leader * harm_to_leader
```

Weights are dynamic, context-driven from BoardContext, normalized to sum to 1.0.

A future phase will add `w_vulture * harm_to_weakest` and `w_convergent * harm_to_opp_target`. Do NOT implement those now.

---

## Step 1: Add `find_leader()` helper

Add this near `find_weakest()` (line ~225). Mirror its structure:

```rust
/// Find the strongest active player by material + FFA score.
fn find_leader(material: &[i32; 4], gs: &GameState) -> Player {
    let scores = gs.scores();
    let mut leader = Player::Red;
    let mut best_strength = i32::MIN;
    for &p in &Player::ALL {
        if gs.player_status(p) == PlayerStatus::Eliminated {
            continue;
        }
        // Combined strength: raw material + FFA score contribution
        let strength = material[p.index()] + scores[p.index()] * 50;
        if strength > best_strength {
            best_strength = strength;
            leader = p;
        }
    }
    leader
}
```

The `* 50` factor converts FFA score points (0-3 typically) into centipawn-scale so material dominates but score breaks ties.

---

## Step 2: Generalize `compute_harm_to_root()` → `compute_harm_to_player()`

The existing function (line ~668) is hardcoded to `root_player`. Generalize it:

1. Rename `compute_harm_to_root` → `compute_harm_to_player`
2. Add a `target_player: Player` parameter (replacing the hardcoded `root_player`)
3. All internal references to `root_player` become `target_player`

The signature becomes:
```rust
fn compute_harm_to_player(mv: Move, board: &Board, target_player: Player) -> f64
```

The body is identical — capture check uses `target_player`, king proximity uses `target_player`'s king square.

Add a convenience wrapper for readability:
```rust
#[inline]
fn compute_harm_to_root(mv: Move, board: &Board, root_player: Player) -> f64 {
    compute_harm_to_player(mv, board, root_player)
}
```

---

## Step 3: Extend `BoardContext` with 2 new fields

Add to the `BoardContext` struct (line ~22):

```rust
/// The player with the highest combined strength (material + score).
pub leader_player: Player,
/// Material totals per player in centipawns (already computed in scan_board).
pub material: [i32; 4],
```

Update `scan_board()` to populate them. The `material` array is already computed locally (line ~72). The leader comes from your new `find_leader()`. Add these two lines around line ~196 (just before constructing BoardContext):

```rust
let leader_player = find_leader(&material, gs);
```

Then add to the return struct:
```rust
BoardContext {
    game_mode: gs.game_mode(),
    root_player,
    weakest_player,
    leader_player,    // NEW
    material,         // NEW (was local variable, now stored)
    most_dangerous,
    // ... rest unchanged
}
```

---

## Step 4: Add `BlendWeights` struct + `compute_blend_weights()`

Add above `score_reply()` (replacing the `LIKELIHOOD_*` constants):

```rust
/// Dynamic blend weights for multi-perspective opponent modeling.
/// All weights are non-negative and sum to 1.0.
#[derive(Debug, Clone)]
struct BlendWeights {
    /// Weight for harm-to-root (paranoid perspective).
    w_paranoid: f64,
    /// Weight for objective move strength (BRS/selfish perspective).
    w_brs: f64,
    /// Weight for harm-to-leader (anti-leader perspective).
    w_anti_leader: f64,
}

/// Compute dynamic blend weights for a specific opponent at a MIN node.
///
/// Weights depend on:
/// - Whether root is this opponent's best target
/// - Whether this opponent is supporting an attack on root
/// - Whether root is the leader (anti-leader folds into paranoid)
/// - The leader's material gap over the opponent
/// - The opponent's own vulnerability (exposed → more selfish/BRS)
fn compute_blend_weights(opponent: Player, ctx: &BoardContext) -> BlendWeights {
    let profile = ctx.per_opponent.iter().find(|p| p.player == opponent);

    // --- Paranoid base ---
    let mut w_paranoid = if profile.is_some_and(|p| p.best_target == ctx.root_player) {
        0.35
    } else {
        0.15
    };
    if profile.is_some_and(|p| p.supporting_attack_on_root) {
        w_paranoid += 0.10;
    }

    // --- BRS base ---
    let mut w_brs = 0.25;

    // --- Anti-leader ---
    let mut w_anti_leader = if ctx.root_player == ctx.leader_player {
        // Root IS the leader: anti-leader motivation folds into paranoid.
        // Everyone targets the leader, so paranoid already models this.
        w_paranoid += 0.15;
        0.0
    } else {
        // Scale by leader's material gap over this opponent.
        // Bigger gap = stronger anti-leader motivation.
        let opp_mat = profile.map_or(0, |p| ctx.material[p.player.index()]);
        let leader_mat = ctx.material[ctx.leader_player.index()];
        let gap = (leader_mat - opp_mat).max(0) as f64;
        // 300cp gap → ~0.15, 600cp+ gap → 0.25 (capped)
        (gap / 2400.0).min(0.25)
    };

    // --- Exposed opponent modifier ---
    // Highly vulnerable opponents play selfishly (defend themselves).
    // Boost BRS, dampen paranoid and anti-leader.
    if let Some(prof) = profile {
        if prof.own_vulnerability > 0.5 {
            let shift = 0.15;
            w_brs += shift;
            w_paranoid = (w_paranoid - shift * 0.5).max(0.05);
            w_anti_leader = (w_anti_leader - shift * 0.5).max(0.0);
        }
    }

    // --- Normalize to sum to 1.0 ---
    let total = w_paranoid + w_brs + w_anti_leader;
    if total > 0.0 {
        BlendWeights {
            w_paranoid: w_paranoid / total,
            w_brs: w_brs / total,
            w_anti_leader: w_anti_leader / total,
        }
    } else {
        // Fallback: pure BRS
        BlendWeights {
            w_paranoid: 0.0,
            w_brs: 1.0,
            w_anti_leader: 0.0,
        }
    }
}
```

---

## Step 5: Delete `LIKELIHOOD_*` constants

Remove ALL of these (lines ~577-594):
- `LIKELIHOOD_BASE_TARGETS_ROOT`
- `LIKELIHOOD_BEST_TARGET_BONUS`
- `LIKELIHOOD_SUPPORTING_BONUS`
- `LIKELIHOOD_EXPOSED_PENALTY`
- `LIKELIHOOD_BASE_NON_ROOT`

Their behavior is now absorbed into `compute_blend_weights()`.

---

## Step 6: Update `ScoredReply` struct

Replace:
```rust
pub struct ScoredReply {
    pub mv: Move,
    pub hybrid_score: f64,
    pub objective_strength: f64,
    pub harm_to_root: f64,
    pub likelihood: f64,
}
```

With:
```rust
pub struct ScoredReply {
    pub mv: Move,
    pub hybrid_score: f64,
    pub objective_strength: f64,
    pub harm_to_root: f64,
    pub harm_to_leader: f64,
}
```

(`likelihood` removed, `harm_to_leader` added)

---

## Step 7: Rewrite `score_reply()` with 3-term formula

Replace the entire `score_reply()` function body. New signature adds no new parameters — `ctx` already carries everything needed:

```rust
pub fn score_reply(
    mv: Move,
    board: &Board,
    root_player: Player,
    opponent: Player,
    ctx: &BoardContext,
    obj_eval_delta: i16,
    max_eval_delta: i16,
) -> ScoredReply {
    // Objective strength: normalized eval improvement (0.0 to 1.0)
    let max_delta = (max_eval_delta.abs() as f64).max(1.0);
    let objective_strength = ((obj_eval_delta.abs() as f64) / max_delta).clamp(0.0, 1.0);

    // Harm terms
    let harm_to_root = compute_harm_to_root(mv, board, root_player);
    let harm_to_leader = if ctx.root_player == ctx.leader_player {
        // Root IS leader — anti-leader term already folded into paranoid weight.
        // Set to 0 so it contributes nothing even if weight somehow > 0.
        0.0
    } else {
        compute_harm_to_player(mv, board, ctx.leader_player)
    };

    // Dynamic blend weights
    let weights = compute_blend_weights(opponent, ctx);

    // 3-term multi-perspective score
    let hybrid_score = weights.w_paranoid * harm_to_root
        + weights.w_brs * objective_strength
        + weights.w_anti_leader * harm_to_leader;

    ScoredReply {
        mv,
        hybrid_score,
        objective_strength,
        harm_to_root,
        harm_to_leader,
    }
}
```

---

## Step 8: Verify `select_hybrid_reply()` — minimal changes

The orchestrator function (`select_hybrid_reply`, line ~733) should need NO changes to its logic. It calls `score_reply()` which now internally uses the 3-term formula. Verify:
- The second pass (line ~804) still calls `score_reply()` with the same arguments
- Sorting by `hybrid_score` descending still works
- The `classify_move` / `MoveClass::Relevant` path is unchanged

The only thing to check: if any code outside this file reads `ScoredReply.likelihood`, update it. Grep for `likelihood` across the codebase.

---

## Step 9: Tests

### Existing tests MUST pass
```
cargo test
```
All 389 tests must pass. If node-count tests in `stage_08_brs_hybrid.rs` or `stage_09_tt_ordering.rs` shift (different opponent replies → different tree shape), update thresholds with a comment explaining why:
```rust
// Threshold adjusted: multi-perspective scoring changes opponent reply selection
```

### New unit tests to add (in `board_scanner.rs` `mod tests`):

1. **`test_find_leader_starting_position`** — All players equal material at start; leader is Player::Red (tie-break by index).

2. **`test_find_leader_material_gap`** — Set up a position where one player has extra material. Verify `find_leader` returns them.

3. **`test_compute_harm_to_player_capture`** — A move capturing Blue's queen should have high harm_to_player(Blue), low harm_to_player(Red).

4. **`test_blend_weights_normalize`** — For any opponent + context, verify `w_paranoid + w_brs + w_anti_leader ≈ 1.0` (within f64 epsilon).

5. **`test_blend_weights_root_is_leader`** — When root IS the leader, `w_anti_leader == 0.0` and paranoid gets the boost.

6. **`test_blend_weights_exposed_opponent`** — Opponent with high vulnerability should have higher BRS weight.

7. **`test_score_reply_uses_blend`** — Set up a move that harms the leader but not root. Verify hybrid_score > 0 (anti-leader term contributes).

---

## Step 10: Update version string

In `odin-engine/src/protocol/emitter.rs`, update:
```rust
"v0.5.0-multi-perspective"
```

---

## Weight Behavior Reference

| Scenario | w_paranoid | w_brs | w_anti_leader |
|---|---|---|---|
| Opponent targets root, root not leader | ~0.47 | ~0.33 | ~0.20 |
| Opponent targets root, root IS leader | ~0.60 | ~0.40 | 0.00 |
| Opponent doesn't target root, big leader gap | ~0.20 | ~0.33 | ~0.47 |
| Opponent doesn't target root, no leader gap | ~0.38 | ~0.62 | 0.00 |
| Exposed opponent (high vulnerability) | lower | higher | lower |

These are approximate — normalization shifts exact values. The key invariant: **weights always sum to 1.0**.

---

## What NOT to do

- Do NOT add vulture (harm_to_weakest) or convergent (harm_to_opp_target) terms yet
- Do NOT change `select_hybrid_reply()` logic (narrowing, classification, fallback)
- Do NOT change `scan_board()` beyond adding `leader_player` and `material` fields
- Do NOT change `brs.rs` — the search calls are unchanged
- Do NOT remove `compute_harm_to_root()` wrapper — keep it for readability

---

## Session-End Protocol

When done, follow `masterplan/AGENT_CONDUCT.md` Section 1.14:
1. Update `masterplan/HANDOFF.md` with what was done and what's next
2. Update `masterplan/STATUS.md`
3. Create a session note in `masterplan/sessions/`
4. Update vault indexes if new issues/components were created

Test counts should remain ~389 + your new tests (aim for ~396+).
