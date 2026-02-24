---
type: component
stage_introduced: 7
tags:
  - stage/07
  - area/search
status: active
last_updated: 2026-02-23
---

# Component: Search (Searcher Trait + BRS)

The search subsystem behind the `Searcher` trait. Stage 7 delivers `BrsSearcher` — a plain Best-Reply Search with alpha-beta, iterative deepening, quiescence search, aspiration windows, null move pruning, LMR, and PV tracking. The trait is the permanent interface boundary; `MctsSearcher` (Stage 10) and the hybrid controller (Stage 11) both work through it.

## Purpose

Selects the best move from a given position within a time/depth/node budget. Without search, the engine makes random moves. BRS makes it playable for the first time. The `Searcher` trait decouples search implementations from the protocol — future searchers (MCTS, hybrid) plug in without touching the rest of the engine.

## Files

- `search/mod.rs` — Searcher trait, SearchBudget, SearchResult (permanent contracts)
- `search/brs.rs` — BrsSearcher, BrsContext, all BRS algorithm logic
- `search/board_scanner.rs` — BoardContext, hybrid reply scoring, progressive narrowing (Stage 8) — see [[Component-BoardScanner]]

## Key Types

- **Searcher** (trait) — Permanent search boundary. `search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult`. Frozen from Stage 7.
- **SearchBudget** — `max_depth: Option<u8>`, `max_nodes: Option<u64>`, `max_time_ms: Option<u64>`. All fields optional; None = no limit.
- **SearchResult** — `best_move: Move`, `score: i16`, `depth: u8`, `nodes: u64`, `pv: Vec<Move>`. Score always from root player's perspective.
- **BrsSearcher** — Concrete searcher. Owns `Box<dyn Evaluator>` and an optional info callback. Constructed per `go` command.
- **BrsContext** (private) — Internal mutable state for one search call. Holds cloned GameState, evaluator ref, root player, node count, start time, budget, stopped flag, PV table, board context (Stage 8).

## Public API

| Item | Signature | Notes |
|---|---|---|
| `Searcher` trait | `search(&mut self, &GameState, SearchBudget) -> SearchResult` | Permanent, frozen |
| `SearchBudget::{ max_depth, max_nodes, max_time_ms }` | All `Option<_>` | None = no limit |
| `SearchResult::{ best_move, score, depth, nodes, pv }` | — | score in [-30000, 30000] |
| `BrsSearcher::new(evaluator)` | `Box<dyn Evaluator> -> BrsSearcher` | No info callback |
| `BrsSearcher::with_info_callback(evaluator, cb)` | `Box<dyn Evaluator>, Box<dyn FnMut(String)> -> BrsSearcher` | Callback receives info lines |
| `BrsSearcher: Searcher` | `search(&mut self, &GameState, SearchBudget) -> SearchResult` | Full BRS with ID |

## Constants (search/brs.rs)

| Constant | Value | Purpose |
|---|---|---|
| `MAX_DEPTH` | 64 | Hard cap on search depth |
| `ASPIRATION_WINDOW` | 50cp | Initial aspiration half-width |
| `NULL_MOVE_REDUCTION` | 2 | R factor for null move (searches at depth - 1 - R) |
| `MAX_QSEARCH_DEPTH` | 8 | Extra plies in quiescence |
| `LMR_MIN_DEPTH` | 3 | Minimum depth for LMR |
| `LMR_MOVE_THRESHOLD` | 3 | Moves tried at full depth before LMR kicks in |
| `MATE_SCORE` | 20_000 | Forced mate score (ply-adjusted) |
| `NEG_INF` | -30_000 | -infinity sentinel (avoids i16::MIN overflow) |
| `POS_INF` | 30_000 | +infinity sentinel |
| `TIME_CHECK_INTERVAL` | 1_024 | Check time/budget every N nodes |

## Internal Flow: search()

1. **Clone GameState** once at the top: `let mut working_gs = position.clone()`. Reused across all ID depths.
2. **Iterative deepening loop** (depth 1..=max_depth):
   a. Aspiration: for depth ≥ 2, search with window [prev_score ± ASPIRATION_WINDOW].
   b. On fail-low: re-search with alpha = NEG_INF, same beta.
   c. On fail-high: re-search with beta = POS_INF, same alpha.
   d. On completion: emit info line via `info_cb`, save PV and score.
   e. Stop if budget exhausted.

## Internal Flow: alphabeta(depth, alpha, beta, ply)

- **Terminal (depth == 0):** → `quiescence()`
- **Terminal (game over):** → eval_scalar at this node
- **MAX node (root player's turn):**
  - Optional null move: if depth ≥ 3, not in check, has non-pawn material → skip turn, search at depth - 1 - R, prune if >= beta.
  - PV move ordering: try prev-depth PV move first.
  - Iterate all legal moves. LMR: after move #3 at depth ≥ 3, non-captures get depth - 1. Re-search at full depth if score > alpha.
  - Standard alpha-beta: update alpha on improvement, prune when alpha >= beta.
- **MIN node (opponent's turn):**
  - Stage 8: Uses `select_hybrid_reply()` from [[Component-BoardScanner]] — classifies moves, applies progressive narrowing, scores with hybrid formula (harm × likelihood + strength × (1-likelihood)).
  - Falls back to plain BRS if no relevant moves found.
  - Play the selected reply and recurse once (no branching at MIN nodes).
  - If eliminated / no legal moves: skip via `set_side_to_move(next)` (Zobrist-safe, symmetric).

## Internal Flow: quiescence(alpha, beta, qs_depth)

- Stand-pat: eval_scalar. If >= beta: return beta (fail high). Else update alpha.
- Iterate only capture moves (generated legal, filtered).
- Recurse up to MAX_QSEARCH_DEPTH extra plies.

## BRS Turn Order (ADR-012)

Natural 4-player order R→B→Y→G. NOT the MASTERPLAN's alternating MAX-MIN-MAX-MIN model.

**Why:** `unmake_move` infers the previous player from `prev_player(side_to_move)` — it does NOT save `side_to_move` in `MoveUndo`. Manual `set_side_to_move()` between `make_move` and `unmake_move` corrupts restoration. The natural order is Zobrist-safe and alpha-beta still prunes effectively at MAX nodes.

**Consequence:** Null move and eliminated-player skip use `set_side_to_move`, but always restore symmetrically before calling `unmake_move`. Any future code that calls `set_side_to_move` inside search must maintain this symmetry.

## Score Perspective

All scores are from `root_player`'s perspective (not negamax). Positive = root player winning. `eval_scalar(gs, root_player)` is called at leaf nodes. Opponent replies are evaluated as `eval_scalar(gs, opponent)` to find their best move at MIN nodes (opponent's score, not root's).

## Info Line Format

Emitted by `info_cb` after each completed iterative deepening depth:
```
info depth <d> score cp <s> v1 <r> v2 <b> v3 <y> v4 <g> nodes <n> nps <nps> time <ms> pv <moves> phase brs
```
- `v1-v4`: per-player `eval_scalar` at root position (not search score) — for UI display.
- `phase brs`: distinguishes BRS info from future MCTS info.

## Connections

- Depends on: [[Component-Eval]] (eval_scalar at leaf nodes), [[Component-GameState]] (clone, is_game_over, legal_moves), [[Component-MoveGen]] (generate_legal, make_move, unmake_move, is_in_check), [[Component-BoardScanner]] (hybrid reply scoring at MIN nodes, Stage 8)
- Depended on by: [[Component-Protocol]] (wires BrsSearcher in handle_go)
- Communicates via: [[Connection-Eval-to-Search]], [[Connection-Search-to-Protocol]]

## Tracing Points

Potential `tracing` spans/events (Huginn was retired in Stage 8; see ADR-015):

| Span/Event | Location | Level |
|---|---|---|
| `alpha_beta_prune` | MAX node cutoff in alphabeta() | TRACE |
| `quiescence` | Entry/exit of quiescence() | TRACE |
| `iterative_deepening` | After each completed depth | DEBUG |
| `brs_reply_selection` | MIN node: opponent, candidates, selected move | TRACE |

## Gotchas

1. **`unmake_move` takes 3 arguments, not 2.** Signature: `unmake_move(board: &mut Board, mv: Move, undo: MoveUndo)`. The Stage 2 audit log incorrectly shows 2 arguments. Missing `mv` causes immediate compile error.
2. **No `set_side_to_move` between make and unmake without symmetric restore.** `unmake_move` uses `prev_player(current_side)` — manual changes corrupt restoration.
3. **TIME_CHECK_INTERVAL = 1024.** Node budgets can overrun by up to 1024. Test assertions should use `<= budget + 1024` as ceiling.
4. **Non-board GameState fields are stale during search.** The cloned GameState used in BRS has a snapshot of `player_status`, `scores`, `halfmove_clock` from the moment `go` was called. These are never updated during make/unmake. Bootstrap eval only reads the Board, so safe for Stage 7. Stage 8+ evaluators must not rely on non-board fields during search.
5. **One clone per `go` command.** `GameState::clone()` includes `piece_lists: [Vec<...>; 4]` — heap allocation. Cost is acceptable at one clone/second. MCTS (Stage 10) must NOT clone per simulation — see [[Issue-Vec-Clone-Cost-Pre-MCTS]].
6. **NEG_INF = -30_000, not i16::MIN.** Negating i16::MIN overflows. Never use i16::MIN as alpha sentinel.
7. **Bootstrap eval lead-penalty causes tactical mismatch.** The evaluator penalizes large material leads, which can make BRS prefer check-giving moves over immediate captures. This is eval behavior, not a BRS bug. See [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]].

## Performance Notes (debug build, starting position)

| Depth | Nodes | Elapsed (ms) | Best Move |
|---|---|---|---|
| 1 | 40 | 2 | e1f3 |
| 2 | 100 | 14 | e1f3 |
| 3 | 164 | 28 | e1f3 |
| 4 | 356 | 80 | e1f3 |
| 5 | 1,425 | 221 | e2e3 |
| 6 | 10,916 | 1,547 | j1i3 |

- Debug depth 6: 1,547ms — within 5s AC4 limit.
- Effective branching factor: ~7x per ply (debug).
- For CI: cap at depth 4 (80ms). For release verification: depth 6 (<300ms estimated).
- Release estimate: 5-10x faster than debug.

## Known Issues

- [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]] (INFO) — lead-penalty causes check preference over captures in some positions.
- [[Issue-Vec-Clone-Cost-Pre-MCTS]] (WARNING) — Vec clone cost will be critical for Stage 10 MCTS.
- No `Stop` command cancellation during search (INFO) — deferred to Stage 8/11.

## Build History

- [[Session-2026-02-21-Stage07]] — initial implementation
- [[Session-2026-02-23-Stage08]] — hybrid reply scoring, board scanner, progressive narrowing

## Related

- [[stage_07_plain_brs]] — spec
- [[audit_log_stage_07]] — audit findings
- [[downstream_log_stage_07]] — API contracts, performance baselines
- [[Component-Eval]] — provides eval_scalar
- [[Connection-Search-to-Protocol]] — how protocol wires BrsSearcher
