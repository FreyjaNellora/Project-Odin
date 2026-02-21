# Issue: Vec Clone Cost — Pre-MCTS Retrofit Required

**Status:** Open
**Severity:** WARNING
**Introduced:** Stage 1 (piece_lists), Stage 3 (position_history)
**Affects:** Stage 10 (MCTS), Stage 12 (Self-Play)
**Action Required Before:** Stage 10

---

## Problem

Two core data structures use heap-allocated `Vec` that will cause silent performance degradation when MCTS clones `GameState` thousands of times per search:

1. **`Board.piece_lists: [Vec<(PieceType, Square)>; 4]`** (board_struct.rs:31) — Every `Board::clone()` allocates 4 new Vecs on the heap. MCTS may clone 5,000+ times per search.

2. **`GameState.position_history: Vec<u64>`** (gamestate/mod.rs:69) — Grows every move. In a 200-move game with 5,000 MCTS simulations, that's 5,000 clones of a Vec containing 200+ entries.

Both agents identified these issues in their downstream logs but deferred because nothing in the spec mandated the fix at build time.

## Evidence

- downstream_log_stage_01.md line 69: "Piece lists use `Vec`, not fixed-size arrays. [...] If profiling shows this is a bottleneck in make/unmake (Stage 2), consider switching to fixed-capacity `ArrayVec<(PieceType, Square), 16>`."
- downstream_log_stage_03.md line 35: "position_history grows unbounded. For long games or MCTS with many clones, the Vec<u64> could become large."

## Required Fix

### piece_lists (Refinement 1)
Replace `Vec<(PieceType, Square)>` with fixed-size arrays:
```rust
piece_lists: [[Option<(PieceType, Square)>; 32]; 4],  // 32 = max pieces per player (16 original + 16 promoted). None-padded.
piece_counts: [u8; 4],  // Active piece count per player, for iteration bounds.
```
When a piece is captured, swap the last active piece into the captured slot and decrement the count. This keeps iteration contiguous and avoids holes.

**Impact:** Changes the return type of `piece_list()` and every call site that iterates over it (movegen, eval, gamestate rules). Requires updating `verify_piece_lists()` and all tests that check piece list contents.

### position_history (Refinement 2)
Replace `Vec<u64>` with `Arc<Vec<u64>>`:
```rust
position_history: Arc<Vec<u64>>,  // Shared between clones. Copy-on-write via Arc::make_mut when appending.
```
Clones share the history by reference. Appending a new position calls `Arc::make_mut`, which only copies if refcount > 1.

**Impact:** Minimal API change. `is_draw_by_repetition` receives `&[u64]` which still works via `Arc::deref()`.

## When to Apply

Schedule as a dedicated retrofit pass between Stage 9 (TT & Move Ordering) and Stage 10 (MCTS). This gives a clean boundary: all search code that touches Board/GameState is built, and MCTS hasn't been started yet.

Create a verification test: clone GameState 1,000 times and assert total heap allocation is bounded (not proportional to clone count).

---

## Related

- [[Component-Board]]
- [[Component-GameState]]
- [[downstream_log_stage_01]]
- [[downstream_log_stage_03]]
- [[MASTERPLAN]] Section 4, Stage 10
