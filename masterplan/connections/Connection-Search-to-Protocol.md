---
type: connection
tags:
  - stage/07
  - area/search
  - area/protocol
last_updated: 2026-02-21
---

# Connection: Search to Protocol

## What Connects

- [[Component-Search]] / `BrsSearcher` (provider)
- [[Component-Protocol]] / `OdinEngine::handle_go` (consumer)

## How They Communicate

`OdinEngine::handle_go` constructs a `BrsSearcher` per `go` command, wires a closure-based info callback, and calls `searcher.search()`. The engine does not hold a persistent searcher between commands — a fresh `BrsSearcher` is constructed each time.

```rust
// Inside handle_go (protocol/mod.rs):

// 1. Collect info lines into a buffer (Rc/RefCell for closure capture).
let info_buf: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
let buf_clone = Rc::clone(&info_buf);
let cb = Box::new(move |line: String| {
    buf_clone.borrow_mut().push(line);
});

// 2. Build searcher with callback.
let mut searcher = BrsSearcher::with_info_callback(
    Box::new(BootstrapEvaluator::new()),
    cb,
);

// 3. Convert protocol limits → search budget.
let budget = Self::limits_to_budget(limits);

// 4. Clone current game state (read-only reference passed to search).
let position = gs.clone();

// 5. Run search (blocks until budget exhausted).
let result = searcher.search(&position, budget);

// 6. Flush info lines then emit bestmove.
for line in info_buf.borrow().iter() {
    self.send(line);
}
self.send(&format_bestmove(&result.best_move.to_algebraic(), None));
```

## SearchLimits → SearchBudget Conversion

Priority order (first match wins):

| Protocol input | Budget produced |
|---|---|
| `infinite = true` | `{ max_depth: None, max_nodes: None, max_time_ms: None }` |
| `movetime = Some(ms)` | `{ max_depth: None, max_nodes: None, max_time_ms: Some(ms) }` |
| `depth = Some(d)` | `{ max_depth: Some(d as u8), max_nodes: None, max_time_ms: None }` |
| `nodes = Some(n)` | `{ max_depth: None, max_nodes: Some(n), max_time_ms: None }` |
| Own time control present | `{ max_time_ms: Some((own_time / 50).max(200)) }` |
| Nothing specified | `{ max_depth: Some(6), ... }` (depth 6 default) |

Own time control priority: `wtime` → `btime` → `ytime` → `gtime` (first non-None wins, player-independent in Stage 7 — protocol does not yet map players to time controls).

## Contract

1. **One BrsSearcher per `go` command.** No state is preserved between searches. History heuristic and TT (Stage 9) will require either persistent searcher storage or serialization — decide at Stage 9.
2. **Info lines are buffered, not streamed.** The `Rc<RefCell<Vec<String>>>` pattern collects all info lines during search, then flushes to stdout after `search()` returns. This means info lines appear all at once, not in real time, during a `go depth N` call. For interactive play this is acceptable; for UI live display of thinking it is not. Fix at Stage 11 if needed.
3. **`position` is a GameState clone, not the live engine state.** `handle_go` clones `self.game_state` before passing to search. The live engine state is not modified during search.
4. **`Command::Stop` is a no-op for search interruption.** BRS respects time budgets via `TIME_CHECK_INTERVAL` polling, but `Stop` does not interrupt an in-progress search. When `go infinite` is issued, the search runs until a budget fires. Proper stop via atomic flag deferred to Stage 8/11.
5. **Searcher trait is the only search interface.** Protocol imports `Searcher` trait + `SearchBudget` from `odin_engine::search`; imports `BrsSearcher` from `odin_engine::search::brs`. Never call `BrsSearcher::alphabeta` or other internals directly.

## Evolution

| Stage | Change |
|---|---|
| 7 (current) | BrsSearcher constructed per go command; info lines buffered; no stop |
| 8 | BRS with updated eval; same wiring pattern |
| 9 | TT + history heuristic may require persistent searcher state |
| 10 | MctsSearcher added; same Searcher trait, wired same way |
| 11 | HybridController composes BRS + MCTS through Searcher trait; stop signal added |
