---
type: issue
date_opened: 2026-02-25
last_updated: 2026-02-25
date_resolved: 2026-02-25
stage: "8"
severity: blocking
status: resolved
tags:
  - area/search
  - stage/08
---

# Issue: Post-Elimination BRS Crash

## Description

After any player was eliminated (e.g., Red checkmated), the engine disconnected instead of continuing the game with the remaining players. Engine sent `eliminated Red`, `nextturn Blue`, `bestmove <move>` correctly, then went silent — subsequent `go` from the UI caused a crash/panic.

**Root cause:** `make_move` advances `side_to_move` via `.next()` (Red→Blue→Yellow→Green→Red) regardless of `PlayerStatus`. The BRS search tree — in both `alphabeta()` and `quiescence()` — reached the eliminated player's virtual turn and called `generate_legal` on a kingless board. This corrupted board state and caused a panic.

Three compounding factors:

1. **Alphabeta/quiescence**: No eliminated-player check before `generate_legal` (primary crash).
2. **Board scanner arrays**: `per_opponent` and `most_dangerous` (`[Player; 3]`) were indexed with the active-opponent count. With < 3 active opponents post-elimination, unused slots were uninitialized → stale opponent profiles fed into hybrid scoring.
3. **Stale king square**: `remove_king` cleared the king piece from the board but left `king_squares[player.index()]` holding the stale square index. `is_in_check` read this stale value and worked with an invalid square.

## Affected Components

- [[Component-Search]] — `alphabeta()` and `quiescence()` in `brs.rs`
- [[Component-BoardScanner]] — `opponents_of()` and array initialization in `board_scanner.rs`
- [[Component-Board]] — `king_squares` field, `remove_king` in `board_struct.rs` / `rules.rs`
- [[Component-GameState]] — `PlayerStatus` check missing from search dispatch

## Workaround

None. Engine panicked after any player elimination, making multi-elimination games unplayable.

## Resolution

Four-layer fix. Commits: `5eaa072` + `445638d`.

**Fix 1 — alphabeta eliminated-player skip** (`brs.rs:374`):
```rust
if self.gs.player_status(current) != PlayerStatus::Active {
    let next = current.next();
    self.gs.board_mut().set_side_to_move(next);
    let score = self.alphabeta(depth, alpha, beta, ply);
    self.gs.board_mut().set_side_to_move(current);
    return score;
}
```
ADR-012 constraint preserved: `set_side_to_move` only used outside make/unmake pairs.

**Fix 2 — quiescence eliminated-player skip** (`brs.rs:562`):
Identical pattern in `quiescence()` before the MAX/MIN branch — quiescence also calls `generate_legal` on `side_to_move` and would crash on the same path at depth=0 leaf nodes.

**Fix 3 — board scanner Active-only filter** (`board_scanner.rs`):
- `opponents_of()` now filters to `PlayerStatus::Active` only
- `per_opponent` slots padded with `root_player` sentinel via `.get(i).copied().unwrap_or(root_player)`
- Sentinel-slot skip guard `if opp == root_player { continue; }` in profile loop
- `most_dangerous` array likewise padded

**Fix 4 — king square sentinel 255** (`board_struct.rs` + `rules.rs`):
- Added `Board::has_king(player)` — returns `king_squares[i] != 255`
- Added `Board::clear_king_square(player)` — writes 255 (clearly invalid sentinel)
- `remove_king()` now calls `clear_king_square()` so stale reads return 255 rather than a real-looking wrong square

Binary verified fresh via version canary (`ENGINE_VERSION = "v0.4.1-fix"`).
User confirmed: "you fixed the issue!" — game continues correctly after elimination.

## Related

- [[Session-2026-02-25-PostElim-Crash-Fix]]
- [[stage_08_brs_hybrid]]
- [[Component-Search]]
- [[Component-BoardScanner]]
- [[Component-Board]]
