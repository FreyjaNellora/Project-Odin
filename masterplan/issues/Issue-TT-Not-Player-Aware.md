---
type: issue
date_opened: 2026-02-26
last_updated: 2026-02-26
date_resolved:
stage: 9
severity: warning
status: open
tags: [area/search, area/tt, severity/warning]
---

# Issue: TT Hash Does Not Include root_player — Latent Contamination Bug

## Description

The transposition table key is computed from `Board::zobrist_hash()`, which XORs in piece positions, castling rights, en passant, and `side_to_move`. It does **not** include `root_player` (the player the BRS search is being conducted for).

In BRS, the same board position with the same side to move can have completely different evaluations depending on which player is the root (MAX player):
- When Red is root: `eval_scalar(pos, Red)` evaluates from Red's perspective
- When Blue is root: `eval_scalar(pos, Blue)` evaluates from Blue's perspective

If a TT entry is stored during Red's search and later probed during Blue's search (same board position, same side_to_move), the stored score would be meaningless — it reflects Red's evaluation, not Blue's.

**Currently safe** because `protocol/mod.rs` line 237 creates a **fresh `BrsSearcher`** (and thus fresh TT) per `go` command. No TT entries survive between different players' searches.

**Becomes a real bug when:**
1. TT is persisted across searches (see [[Issue-TT-Fresh-Per-Search]])
2. TT is shared between MCTS simulations for different root players
3. Any future code reuses a `BrsSearcher` across multiple `go` commands

## Affected Components

- [[Component-Search]] — `BrsSearcher`, `alphabeta()`, TT probe/store
- `odin-engine/src/search/tt.rs` — `TTEntry` structure, `probe()`, `store()`
- `odin-engine/src/board/zobrist.rs` — hash computation

## Proposed Fix

**Option A — Include root_player in hash (recommended):**

Add a 4-element Zobrist key array for root_player in `zobrist.rs`. XOR the appropriate key when computing the position hash for TT purposes. This means the same position hashes differently depending on who is searching, preventing cross-player contamination.

Implementation: in `alphabeta()`, XOR `ZOBRIST_ROOT_PLAYER[root_player.index()]` into the hash before TT probe/store. The base `Board::zobrist_hash()` stays unchanged (it's used for repetition detection, which is player-independent).

**Option B — Separate TTs per player:**

Give each player their own TT. This wastes memory (4x) but is simpler. Not recommended.

**Option C — Clear TT between root_player changes:**

Call `tt.clear()` when root_player changes. Loses all knowledge. Not recommended.

**Timing:** Fix this BEFORE persisting TT across searches ([[Issue-TT-Fresh-Per-Search]]). If TT is persisted without this fix, scores will be wrong.

## Workaround

Currently: protocol creates fresh `BrsSearcher` per `go` command, so TT is always clean. No action needed until TT persistence is implemented.

## Resolution

<!-- When fix is implemented, describe what was done and set status to "pending-verification". -->

## Related

- [[Issue-TT-Fresh-Per-Search]] — companion issue; both must be fixed together
- [[Session-2026-02-26-BRS-Architecture-Investigation]] — analysis that identified this
- [[stage_09_tt_ordering]] — TT implementation stage
- [[Component-Search]] — BRS search implementation
