# Issue: EP Remove-Piece Crash V2 (Post-Fix Regression)

**Status:** RESOLVED -- root cause confirmed, fix applied and verified
**Discovered:** 2026-03-04
**Resolved:** 2026-03-05
**Stage:** 19 Phase 5 stress testing
**Files:** `odin-engine/src/movegen/moves.rs`, `odin-engine/src/movegen/generate.rs`

---

## Symptom

```
thread 'main' panicked at odin-engine\srcoardoard_struct.rs:425:47:
remove_piece: square is empty
```

Same panic location as the original EP bug (fixed in Stage 19 Phase 5 Attempt 2).
Crash rate: ~1 in 430 games (~0.23%). Engine crash detection recovers and continues.

---

## Occurrence

- **Game 144** of the 2026-03-04 stress test run (rotation 6/6)
- Green eliminated at ply 58, Yellow eliminated at ply 123
- Crash at ply 153 (Blue to move -- crash occurs during MCTS search, not actual game sequence)
- Last recorded actual move: ply 152, Red: e1e2

---

## Original Fix (Attempt 2, 2026-03-02)

The first EP crash used `player.prev()` to find the en-passant capturing pawn's pusher. After eliminations, `prev()` returns the wrong (eliminated) player, whose expected pawn square is empty -> `remove_piece` panics.

**Fix applied:** `find_ep_captured_pawn_sq()` in `moves.rs` -- scans all 4 PAWN_FORWARD directions from `ep_target`, looking for a pawn matching each candidate player.

---

## Why the Fix Still Failed

`find_ep_captured_pawn_sq` had a **fallback**:

```rust
// Fallback: use prev() (normal case without eliminations)
en_passant_captured_sq(ep_target, capturing_player.prev())
```

If the board scan found NO pawn in any of the 3 checked directions:
1. Fallback fires
2. `capturing_player.prev()` may return eliminated player
3. `en_passant_captured_sq` computes wrong square
4. `board.remove_piece(wrong_square)` -> PANIC (square is empty)

---

## Root Cause (Confirmed 2026-03-05)

**Self-EP scenario in `check_elimination_chain`.**

The scenario:
1. Blue double-pushes a pawn -- `ep_sq` is set (e.g., ep_target = (2,4))
2. Red is immediately in checkmate -> Red is eliminated
3. `check_elimination_chain` advances to Blue as the next active player
4. Blue's pseudo-legal EP moves are generated -- Blue has a pawn at (1,3) that attacks (2,4)
5. `find_ep_captured_pawn_sq` scans for an enemy pawn near ep_target, skipping Blue (the capturing player)
6. Blue's own double-pushed pawn IS at (3,4), but the scan skips Blue
7. No other player's pawn is there -> scan returns nothing
8. Old fallback: `en_passant_captured_sq(ep_target, Blue.prev())` -> wrong square -> PANIC

**Confirmed by diagnostic panic output:**
```
find_ep_captured_pawn_sq: no pawn found! ep_target=(2,4) capturing=Blue
scanned=[Red->(2,5)[empty], Yellow->(2,3)[empty], Green->(1,4)[empty]]
```
All three scanned squares are empty because the pawn is Blue's own (skipped by design).

---

## Hypotheses Investigated

### Hypothesis 1 -- DKW King Captures EP Pawn (RULED OUT)

Green and Yellow were eliminated by checkmate/stalemate -- status is `PlayerStatus::Eliminated` with pieces removed entirely. `process_dkw_moves` skips `Eliminated` players. No DKW king active in game_0144 at ply 153.

**Separate bug found and fixed:** `generate_dkw_move` did not filter captures, so DKW kings COULD capture pieces in games where they ARE active. Fixed in Session 3.

### Hypothesis 2 -- Corner Cutout Scan Gap (NOT THE CAUSE)

The scan bounds check correctly skips corner cutouts. `square_from` returns `None` for invalid corner squares, which the scan handles gracefully.

### Hypothesis 3 -- BRS Skip Mechanism (RULED OUT)

BRS's eliminated-player skip never occurs between a `make_move`/`unmake_move` pair. Not on the crash path.

### Hypothesis 4 -- TT Cross-Position EP Move (NOT ON THIS CRASH PATH)

TT stores moves as from+to only (drops EP flag). `decompress_move` re-derives the flag from legal moves. This could cause issues in the BRS layer, but the crash backtrace goes through `expand_node -> apply_move`, not through BRS/TT. Separate deferred concern.

---

## Fix Applied (2026-03-05)

### Part 1: `find_ep_captured_pawn_sq` -> `Option<Square>` (pub)

**File:** `odin-engine/src/movegen/moves.rs`

Changed return type from `Square` (with silent fallback) to `Option<Square>`, made `pub`:

```rust
pub fn find_ep_captured_pawn_sq(
    board: &Board,
    ep_target: Square,
    capturing_player: Player,
) -> Option<Square> {
    let file = file_of(ep_target) as i8;
    let rank = rank_of(ep_target) as i8;
    for pidx in 0..4 {
        let candidate = Player::from_index(pidx).unwrap();
        if candidate == capturing_player { continue; }
        let (df, dr) = PAWN_FORWARD[candidate.index()];
        let cf = file + df;
        let cr = rank + dr;
        if cf >= 0 && cf < 14 && cr >= 0 && cr < 14 {
            if let Some(sq) = square_from(cf as u8, cr as u8) {
                if let Some(piece) = board.piece_at(sq) {
                    if piece.piece_type == PieceType::Pawn && piece.owner == candidate {
                        return Some(sq);
                    }
                }
            }
        }
    }
    None
}
```

`make_move` EP branch updated to use `.expect()` (panics with clear message if invalid EP move somehow reaches it):

```rust
FLAG_EN_PASSANT => {
    let captured_pawn_sq = find_ep_captured_pawn_sq(board, to, player)
        .expect("make_move EP: no enemy pawn near ep_target (invalid EP move generated)");
    captured_piece = Some(board.remove_piece(captured_pawn_sq));
    board.move_piece(from, to);
}
```

### Part 2: EP Generation Gated on Enemy Pawn Validation

**File:** `odin-engine/src/movegen/generate.rs`

EP generation in `generate_pseudo_legal` gated on `find_ep_captured_pawn_sq().is_some()`:

```rust
// En passant capture: only generate if an enemy pawn actually exists
// near ep_target. This prevents an invalid self-capture when the player
// who just double-pushed is later tested for checkmate in
// check_elimination_chain (their own pawn set ep_sq, but the scan
// skips the current player, finds nothing, and the fallback crashes).
if let Some(ep_sq) = board.en_passant() {
    if target_sq == ep_sq && find_ep_captured_pawn_sq(board, ep_sq, player).is_some() {
        moves.push_move(Move::new_en_passant(sq, ep_sq));
    }
}
```

---

## Verification

- `observer/reproduce_crash.mjs`: 500 attempts, ZERO crashes (vs. crash at attempt 2 pre-fix)
- `cargo build --release`: succeeded, 0 warnings
- Release binary confirmed working

---

## Deferred Issues (Not Part of This Fix)

### EP Rule Correctness
`ep_sq` is cleared after every `make_move`, which denies eligible players whose turn comes after an ineligible player's move. The correct rule: each eligible player gets one chance on their own immediate next turn. Deferred -- separate from crash.

### TT EP Flag Concern
`compress_move` stores only from+to (drops EP flag). `decompress_move` re-derives the flag by searching legal moves. Could cause issues in the BRS layer if a stale EP move is replayed. Deferred -- separate concern.

---

## Related

- [[Issue-EP-Remove-Piece-Crash]] -- original EP bug (fixed, closed)
- `masterplan/TEMP_STRESS_TEST_LOG.md` -- Attempt 1-2 history
- `odin-engine/src/movegen/moves.rs` -- `find_ep_captured_pawn_sq` (pub, returns Option)
- `odin-engine/src/movegen/generate.rs` -- EP generation gate
- `odin-engine/src/gamestate/rules.rs:66-88` -- `generate_dkw_move` (DKW capture fix applied here)
- `observer/reproduce_crash.mjs` -- crash reproduction tool (now points to release binary)
