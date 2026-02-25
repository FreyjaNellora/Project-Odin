---
type: session
date: 2026-02-25
stage: "Post-Stage 8 (non-stage work)"
tags:
  - type/session
  - area/search
  - area/engine
---

# Session: 2026-02-25 — Post-Elimination Crash Fix

Second session of the day (after [[Session-2026-02-25-UI-Bugfixes]]). Started with eval strengthening from Terminal Claude's specification, then discovered a hard crash during playtesting that prevented games from continuing past any player elimination.

## What Was Done

### 1. Eval Strengthening (`dcb1eb9`)

Applied Fix 4 from Terminal Claude's post-audit spec to strengthen eval and search quality:

- `PAWN_SHIELD_BONUS`: 15 → 35 (`odin-engine/src/eval/king_safety.rs`)
- `OPEN_KING_FILE_PENALTY: i16 = 25` added (new constant + `open_file_penalty()` with `forward_delta()` helper)
- `THREAT_PENALTY_PER_OPPONENT`: 30 → 50 (`odin-engine/src/eval/multi_player.rs`)
- MVV-LVA (Most Valuable Victim – Least Valuable Attacker) capture ordering added to `order_moves()` in `brs.rs`: `score = victim_value * 10 - attacker_value`
- King PST adjustments per spec

### 2. Post-Elimination Crash Discovered

During playtesting, Red was checkmated by Green's `Qn7i2` at move 7. After `eliminated Red` + `bestmove n7i2`, the game disconnected — no further moves from Blue/Yellow/Green.

**Root cause:** `make_move` advances `side_to_move` via `.next()` (Red→Blue→Yellow→Green→Red) regardless of `PlayerStatus`. When the BRS search tree reached Red's virtual turn post-elimination:
- Red's king had been removed via `remove_king`
- `king_squares[Red.index()]` still held the stale square index (not cleared)
- `generate_legal` / `is_in_check` read the stale square → board state corruption → panic

**Secondary:** `board_scanner.rs` `per_opponent` and `most_dangerous` arrays (`[Player; 3]`) used the full non-root opponent count for indexing. Post-elimination with < 3 active opponents, unused slots were uninitialized, causing stale opponent profiles.

**Tertiary:** `remove_king` left `king_squares[player.index()]` with the stale square index rather than writing a 255 sentinel, so stale reads returned a plausible-looking but invalid value.

### 3. Four-Layer Crash Fix

**Fix 1 — alphabeta eliminated-player skip** (`brs.rs`, commit `5eaa072`):
```rust
if self.gs.player_status(current) != PlayerStatus::Active {
    let next = current.next();
    self.gs.board_mut().set_side_to_move(next);
    let score = self.alphabeta(depth, alpha, beta, ply);
    self.gs.board_mut().set_side_to_move(current);
    return score;
}
```
ADR-012 constraint preserved: no `set_side_to_move` is inserted between a `make_move`/`unmake_move` pair.

**Fix 2 — board scanner Active-only filter** (`board_scanner.rs`, commit `5eaa072`):
- `opponents_of()` filters to `PlayerStatus::Active` only
- `per_opponent` array (`[Player; 3]`) padded with `root_player` sentinel for unused slots via `.get(i).copied().unwrap_or(root_player)`
- `most_dangerous` array likewise padded
- Guard `if opp == root_player { continue; }` skips sentinel entries in profile loop

**Fix 3 — king square sentinel 255** (`board_struct.rs` + `rules.rs`, commit `5eaa072`):
- `Board::has_king(player)` — returns `king_squares[i] != 255`
- `Board::clear_king_square(player)` — writes 255 sentinel
- `remove_king()` now calls `board.clear_king_square(player)` after removing the piece, so stale reads return 255 (clearly invalid) rather than a real-looking wrong square

**Fix 4 — quiescence eliminated-player skip** (`brs.rs`, commit `445638d`):
Same skip pattern applied in `quiescence()` before the MAX/MIN branch dispatch. `quiescence()` also calls `generate_legal` on `side_to_move()` and would crash on the same path via quiescence extension at depth=0 leaf nodes.

### 4. Binary Verification Canary

Added `ENGINE_VERSION = "v0.4.1-fix"` in `emitter.rs` to confirm the Tauri app loads the correct rebuilt binary. Verified: `echo "odin" | target/debug/odin-engine.exe` → `id name Odin v0.4.1-fix`. Version string retained permanently.

## User Verification

User confirmed: "you fixed the issue!" — game continues correctly after player elimination.

## Files Modified

### Engine
- `odin-engine/src/search/brs.rs` — alphabeta skip (Fix 1) + quiescence skip (Fix 4) + MVV-LVA ordering
- `odin-engine/src/search/board_scanner.rs` — Active-only opponent filter + sentinel padding (Fix 2)
- `odin-engine/src/board/board_struct.rs` — `has_king()`, `clear_king_square()` (Fix 3)
- `odin-engine/src/gamestate/rules.rs` — `remove_king()` calls `clear_king_square()` (Fix 3)
- `odin-engine/src/protocol/emitter.rs` — `ENGINE_VERSION = "v0.4.1-fix"`
- `odin-engine/src/eval/king_safety.rs` — `PAWN_SHIELD_BONUS`, `OPEN_KING_FILE_PENALTY`, `open_file_penalty()`
- `odin-engine/src/eval/multi_player.rs` — `THREAT_PENALTY_PER_OPPONENT`

## Test Counts

- Engine: 361 (233 unit + 128 integration, 3 ignored) — no new tests; smoke_play tests cover post-elimination game continuation
- UI Vitest: 54 — unchanged
- 0 failures

## Commits

- `dcb1eb9` — `[Eval] Strengthen king safety + MVV-LVA capture ordering (Stage 8 debugging)`
- `5eaa072` — `[Search] Fix post-elimination crash: skip eliminated players in BRS alphabeta`
- `445638d` — `[Fix] Quiescence eliminated-player skip + version bump v0.4.1-fix`
