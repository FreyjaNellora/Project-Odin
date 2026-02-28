---
type: issue
date_opened: 2026-02-26
last_updated: 2026-02-26
date_resolved:
stage: 9
severity: warning
status: open
tags: [area/search, area/tt, area/protocol, severity/warning]
---

# Issue: TT Discarded Between Searches — Easy Performance Win Lost

## Description

`protocol/mod.rs` line 237 creates a **fresh `BrsSearcher`** per `go` command:

```rust
let mut searcher =
    BrsSearcher::with_info_callback(Box::new(BootstrapEvaluator::new(profile)), cb);
let result = searcher.search(&position, budget);
```

The `BrsSearcher` contains a `TranspositionTable` (1M entries, ~12MB). Every search starts with an empty TT. Iterative deepening within a single search reuses the TT effectively (58% node reduction at depth 6), but between moves ALL knowledge is lost.

In a typical game, Red's move at ply N explores many positions that are still relevant at ply N+4 (Red's next turn). The current approach discards all this information.

**Note:** This issue is coupled with [[Issue-TT-Not-Player-Aware]]. The TT cannot be naively persisted because entries from Red's search would contaminate Blue's search. Both issues must be fixed together.

## Affected Components

- [[Component-Protocol]] — `protocol/mod.rs`, `handle_go()` function
- [[Component-Search]] — `BrsSearcher` struct, TT lifecycle

## Proposed Fix

**Step 1:** Fix [[Issue-TT-Not-Player-Aware]] first (add root_player to TT hash).

**Step 2:** Hoist `BrsSearcher` into `OdinEngine` state (or the protocol handler's persistent state). On each `go` command:
1. Call `searcher.tt.increment_generation()` (already exists — 6-bit wrapping counter)
2. Reuse the existing TT entries (stale entries are naturally replaced by depth-preferred + generation-aware replacement policy)
3. Reset per-search state (killers, history, countermoves are already in `BrsContext`, which is created fresh each search)

**Expected benefit:** Significant node reduction on subsequent searches. The TT from depth-8 search of ply N provides move ordering hints and score bounds for ply N+4. Exact benefit requires measurement (Stage 12 self-play).

**Step 3 (optional):** When MCTS is added (Stage 10), the persistent TT can also warm MCTS rollouts.

## Workaround

Currently: each search starts cold. Iterative deepening within a single search still gets TT benefit. The fresh-per-search approach is correct, just suboptimal.

## Resolution

<!-- When fix is implemented, describe what was done and set status to "pending-verification". -->

## Related

- [[Issue-TT-Not-Player-Aware]] — must be fixed first; TT persistence without player-awareness causes score contamination
- [[Session-2026-02-26-BRS-Architecture-Investigation]] — analysis that identified this
- [[stage_09_tt_ordering]] — TT implementation stage
- [[Component-Search]] — BRS search implementation
- [[Component-Protocol]] — protocol handler where BrsSearcher is created
