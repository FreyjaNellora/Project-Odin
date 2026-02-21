# Downstream Log — Stage 04: Protocol

## Notes for Future Stages

### Must-Know

1. **OdinEngine owns the GameState.** The protocol handler creates a `GameState` on `position` commands and stores it as `Option<GameState>`. It is replaced (not modified) on each new `position` command.
2. **Move strings are matched against legal moves.** There is no `Move::from_algebraic()`. The protocol generates all legal moves via `GameState::legal_moves()`, then finds the one whose `to_algebraic()` matches the input string. This is the standard UCI approach.
3. **`go` currently returns a random legal move.** Stage 7 should replace the random move selection in `handle_go()` with actual search through the `Searcher` trait. The handler structure is ready.
4. **All responses go to stdout via `send()`.** The `send()` method both prints to stdout and records in `output_buffer`. Tests use `take_output()` to verify responses without actual I/O.
5. **`stop` is a no-op.** No search runs in a separate thread. Stage 7+ should implement `stop` to interrupt an ongoing search.
6. **Error responses use `info string Error: <msg>`.** Following UCI convention. No output goes to stderr.

### API Contracts

| Item | Signature / Format | Notes |
|---|---|---|
| `OdinEngine::new()` | `-> Self` | Creates engine with default settings, no position |
| `OdinEngine::run()` | `(&mut self)` | Stdin read loop, exits on `quit` or EOF |
| `OdinEngine::handle_command()` | `(&mut self, Command) -> bool` | Returns true on Quit. Used by tests. |
| `OdinEngine::take_output()` | `(&mut self) -> Vec<String>` | Takes and clears output buffer. For testing. |
| `OdinEngine::game_state()` | `(&self) -> Option<&GameState>` | Inspect current game state. For testing. |
| `parse_command()` | `(&str) -> Command` | Parse raw input line to Command enum |
| `Command` enum | `Odin, IsReady, SetOption, PositionFen4, PositionStartpos, Go, Stop, Quit, Unknown` | All protocol commands |
| `SearchLimits` | struct | `wtime/btime/ytime/gtime: Option<u64>`, `depth: Option<u32>`, `nodes: Option<u64>`, `movetime: Option<u64>`, `infinite: bool` |

**Protocol format (stdin → engine):**
```
odin
isready
setoption name <name> value <value>
position startpos [moves <move_list>]
position fen4 <fen_string> [moves <move_list>]
go [wtime <ms>] [btime <ms>] [ytime <ms>] [gtime <ms>] [depth <N>] [nodes <N>] [movetime <ms>] [infinite]
stop
quit
```

**Protocol format (engine → stdout):**
```
id name Odin v0.4.0
id author Project Odin
odinok
readyok
bestmove <move>
info [depth <N>] [seldepth <N>] [score cp <N>] [v1 <N> v2 <N> v3 <N> v4 <N>] [nodes <N>] [nps <N>] [time <ms>] [pv <moves>] [phase <brs|mcts>] [brs_surviving <N>] [mcts_sims <N>]
info string Error: <message>
```

### Known Limitations

1. **No threading.** The `go` command blocks the main loop. `stop` cannot interrupt it. Stage 7+ should add threading for search.
2. **No pondering.** `bestmove` never includes a ponder move.
3. **No `ucinewgame` command.** A new `position` command replaces the game state.
4. **Move parsing depends on legal move generation.** Every move in a `position ... moves` list triggers a full legal move generation to find the matching move. For long move lists, this is O(n * m) where n=moves and m=legal moves per position. Not a concern for protocol (not in hot path).
5. **`setoption` only recognizes `Debug` and `Terrain`.** All other options are silently accepted and ignored.
6. **Huginn gates not wired.** 4 gates defined (command_receive, response_send, position_set, search_request) but not implemented.

### Performance Baselines

| Metric | Value | Notes |
|---|---|---|
| Test count (no huginn) | 229 | 156 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 + 17 stage-04 |
| Perft values | 20/395/7800/152050 | Unchanged from Stage 2 |

### Open Questions

1. Should `go` run in a separate thread from Stage 7 onward? The current design is synchronous. Threading would require shared state or message passing for `stop` handling.
2. Should the protocol support `display` or `d` commands for debug board display? Not in spec, but common in UCI engines for debugging.

### Reasoning

1. **Why match algebraic against legal moves instead of parsing:** Creating a `Move` from algebraic notation requires knowing piece type, captured piece, flags (en passant, castling, double push), and promotion. All that context lives in the board state. Matching against `to_algebraic()` of legal moves is the universal UCI approach and avoids duplicating move generation logic.
2. **Why `output_buffer` instead of a trait for I/O:** The buffer is the simplest testable approach. A trait abstraction (e.g., `impl Write`) would be over-engineering for Stage 4 per AGENT_CONDUCT 1.8. If future stages need I/O abstraction for testing, the buffer can be replaced.
3. **Why LCG for random move:** Consistent with the DKW and integration test RNG. Zero external dependencies. The random move is a stub that will be replaced by actual search.



---

## Related

- Stage spec: [[stage_04_protocol]]
- Audit log: [[audit_log_stage_04]]
