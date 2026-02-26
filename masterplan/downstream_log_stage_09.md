# Downstream Log — Stage 09: TT & Move Ordering

**Date:** 2026-02-25
**Author:** Claude Sonnet 4.6 (Stage 9 session)

## Notes for Future Stages

### Must-Know

1. **TT lives in BrsSearcher; persists across searches.** `BrsSearcher::tt: TranspositionTable` is not reset between `search()` calls. This is intentional — the TT accumulates position knowledge across iterative deepening depths and between moves. If a test needs a clean TT, it must construct a new `BrsSearcher`.

2. **TT probe/store is in `alphabeta()`, not in `max_node` or `min_node`.** Probe happens after the repetition check and before depth==0 quiescence dispatch. Store happens at the bottom of alphabeta() after the search result is known. The repetition-then-TT ordering is critical — never move the TT probe above the repetition check.

3. **Aborted searches do not store in TT.** When `self.stopped == true`, `alphabeta()` returns early without storing. This prevents caching partial results from time-budget-exceeded searches.

4. **Terminal nodes (checkmate/stalemate) are stored as TT_EXACT.** These scores are static facts. The best_move is stored as None (no move from a mated position).

5. **Eliminated-player skip does not write TT.** The `alphabeta()` early return for eliminated players does not reach the TT store. This is acceptable — the structural skip always produces the same result (deterministic).

6. **Counter-move indexing: `from * TOTAL_SQUARES + to` (flat Vec, not 2D array).** `TOTAL_SQUARES = 196`. If TOTAL_SQUARES changes (e.g., board geometry change), update both the constant and the Vec size. Current size: 196 × 196 × ~8 bytes ≈ 300 KB on heap.

7. **History heuristic is root-player-indexed only.** `history[root_player.index()][pt][to]` — only the root player's history is updated (Stage 9 only uses MAX node beta cutoffs). A future stage that wants per-opponent history must extend the indexing.

8. **SEE is simplified (single-exchange).** `see(mv, threshold) -> bool` returns `captured_val - attacker_val >= threshold`. No recursive exchange simulation for 4PC. Full recursive SEE planned for Stage 19. To upgrade, replace the body of `see()` — the signature is the permanent interface.

9. **Killer moves cleared per search.** `killers: [[Option<Move>; 2]; MAX_DEPTH]` is zero-initialized in `BrsContext::new()`. They are NOT carried across `search()` calls (unlike TT). Killers are position-specific; stale killers from a different position would add noise.

10. **`order_moves()` signature changed from Stage 7/8.** The old `order_moves(moves, pv_move)` is replaced with `order_moves(moves, hint_move, killers, countermove, history, player_idx)`. Any Stage 10+ code that calls `order_moves` directly must use the new signature.

### API Contracts

**TranspositionTable (public, `search/tt.rs`):**
```rust
pub const TT_EXACT: u8;   // 0b01
pub const TT_LOWER: u8;   // 0b10 (score is lower bound, failed high)
pub const TT_UPPER: u8;   // 0b11 (score is upper bound, failed low)
pub const TT_DEFAULT_ENTRIES: usize; // 1 << 20 (~12 MB)

pub struct TTProbe {
    pub score: Option<i16>,      // Some = early return value
    pub best_move: Option<u16>,  // compressed move hint (from | to<<8)
}

pub struct TranspositionTable { /* opaque */ }

impl TranspositionTable {
    pub fn new(num_entries: usize) -> Self         // num_entries must be power of 2
    pub fn with_mb(mb: usize) -> Self
    pub fn len(&self) -> usize
    pub fn increment_generation(&mut self)          // call once per search()
    pub fn probe(&self, hash: u64, depth: u8, alpha: &mut i16, beta: &mut i16, ply: u8) -> TTProbe
    pub fn store(&mut self, hash: u64, best_move: Option<u16>, score: i16, depth: u8, flag: u8, ply: u8)
    pub fn compress_move(mv: Move) -> u16           // from | to<<8
    pub fn decompress_move(compressed: u16, moves: &[Move]) -> Option<Move>
}
```

**BrsSearcher (public, `search/brs.rs`) — unchanged Searcher trait interface:**
```rust
impl Searcher for BrsSearcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult;
}
```
TT is an internal field; both `BrsSearcher::new()` and `with_info_callback()` initialize TT at `TT_DEFAULT_ENTRIES` (1 << 20, ~12 MB).

**`see()` (private free function, `search/brs.rs`):**
```rust
fn see(mv: Move, threshold: i16) -> bool
// Returns true if captured_val - attacker_val >= threshold.
// Simplified single-exchange only; full recursive deferred to Stage 19.
```

**BrsContext new private fields:**
```rust
killers: [[Option<Move>; 2]; MAX_DEPTH],             // 2 killers per ply, reset per search
history: [[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT], // [player][pt][to], reset per search
countermoves: Vec<Option<Move>>,                     // flat [from*196+to], reset per search
last_opp_move: [Option<Move>; MAX_DEPTH],            // set by min_node before recursing
```

**`order_moves()` full signature (private):**
```rust
fn order_moves(
    moves: &[Move],
    hint_move: Option<Move>,                          // TT move (or PV fallback)
    killers: &[Option<Move>; 2],
    countermove: Option<Move>,
    history: &[[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT],
    player_idx: usize,
) -> Vec<Move>
```
Pipeline: TT hint → winning caps (SEE≥0, MVV-LVA) → non-cap promotions → killers → counter-move → hist-sorted quiets → losing caps (SEE<0, MVV-LVA).

### Known Limitations

**W6 — Simplified SEE (NEW, INFO):**
`see()` uses single-exchange approximation: `captured_val - attacker_val >= threshold`. Does not model 4PC recapture chains (up to 3 opponents). Misclassification risk: a capture that is winning after the immediate exchange but loses on recapture 2 moves later may be placed in the win_caps bucket. This is conservative and rare; the move still gets searched. Full recursive SEE planned for Stage 19.

**W5 — Stale GameState fields during search (carried from Stage 7):**
Still open. TT reads only `board.zobrist()` — not affected.

**W4 — Lead penalty tactical mismatch (carried from Stage 7):**
Still open. Stage 9 integration tests use `EvalProfile::Aggressive` to avoid it.

### Performance Baselines

**Stage 9 (release build), starting position, Standard profile:**

| Depth | Nodes (Stage 9) | Nodes (Stage 7) | Reduction | Elapsed (ms) |
|-------|-----------------|-----------------|-----------|--------------|
| 1     | 40              | 40              | 0%        | <1           |
| 2     | 100             | 100             | 0%        | <1           |
| 3     | 130             | 164             | 21%       | <1           |
| 4     | 227             | 356             | 36%       | <1           |
| 5     | 849             | 1,425           | 40%       | <1           |
| 6     | 4,595           | 10,916          | **58%**   | 50           |
| 7     | 8,097           | 19,309          | 58%       | 70           |
| 8     | 13,009          | 31,896          | **59%**   | 120          |

**Stage 9 (release build), starting position, Aggressive profile:**

| Depth | Nodes (Stage 9) | Elapsed (ms) |
|-------|-----------------|--------------|
| 6     | 4,064           | 34           |
| 8     | 12,205          | 185          |

**Key observations:**
- TT + ordering produces **58-59% node reduction** at depths 6-8 vs Stage 7 baseline. Acceptance criterion (>50%) met with margin.
- Reduction grows with depth: TT hits compound as the table fills across iterative deepening.
- Depth 8 in 120ms (release): improvement from Stage 7's 371ms. Deeper searches now practical within 5-second budget.
- Best move at depth 8 (`d2d3`) differs from Stage 7 (`j1i3`): different ordering changes which equal-score lines are found first. Both are legal and in the expected score range.
- For CI: cap integration tests at depth 4 (debug) or depth 6 (release). Depth 8 at 120ms release is well within the 5-second AC4 limit.

### Open Questions

1. **TT size: should it be configurable?** Default 1<<20 (~12 MB) is conservative. A `setoption Hash <mb>` parameter could allow `with_mb()` configuration. Standard for UCI engines. Deferred to Stage 11 or 17.

2. **History decay between searches?** History accumulates without decay. Over many searches, frequently-played quiet moves can saturate their history scores. Standard fix: divide by 2 every N searches (aging). Deferred to Stage 17.

3. **Is the delta board scanner updater worth implementing pre-Stage 10?** With TT, depth 8 is now practical in 120ms. Scanner data frozen at root may be staler at depth 10+. Evaluate after Stage 10 MCTS performance is measured.

4. **Upgrade SEE before Stage 11?** The hybrid controller in Stage 11 uses BRS as its primary searcher. Misclassified captures (simplified SEE) cause suboptimal ordering in multi-recapture positions. Evaluate after Stage 11 design is finalized.

5. **History table extraction for Progressive History (ADR-017).** Stage 11's hybrid controller needs to extract BRS's history table (`history: [[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT]`) after Phase 1 completes and pass it to MctsSearcher for Progressive History warm-start. BrsSearcher needs a `pub fn history_table(&self) -> &HistoryTable` accessor. Currently the history table is a private field in BrsContext, reset per search. Stage 11 must extract it AFTER the search completes but BEFORE BrsContext is dropped. Design consideration: should the accessor return a reference to the live table, or clone the relevant subset (e.g., only root player's history)?

### Reasoning

The TT implementation uses depth-preferred replacement with generation fallback — the standard approach in production chess engines. The generation mechanism (6-bit counter in flags bits 2-7) ensures old entries from previous games don't dominate the table. The `score_to_tt`/`score_from_tt` ply adjustment for mate scores is essential: without it, a mate-in-3 found at depth 6 (ply 3) would be retrieved at depth 7 (ply 4) as a "mate-in-2", causing score drift across iterative deepening depths.

The full move ordering pipeline (Steps 4-8) compounds with TT to produce the observed 58% node reduction: TT provides the best-move hint (tried first, immediately tightens alpha); killers and history identify refutation moves for quiet positions; SEE stratifies captures into winning and losing buckets so losing captures are tried last. Together these produce better alpha-beta bound tightening and more beta cutoffs earlier in the move list.

---

## Related

- Stage spec: [[stage_09_tt_ordering]]
- Audit log: [[audit_log_stage_09]]
