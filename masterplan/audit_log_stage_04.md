# Audit Log — Stage 04: Protocol

## Pre-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build` in 0.01s, `cargo build --features huginn` in 1.85s)
- Tests pass: Yes (164 total: 108 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03)
- Previous downstream flags reviewed: Yes — Stage 0, 1, 2, 3 downstream logs reviewed

### Findings

**From [[downstream_log_stage_03]]:**
1. `GameState::apply_move()` is the central game lifecycle method. Protocol must use this for game-level moves (not raw `make_move`).
2. `GameState::legal_moves()` requires `&mut self` (calls `board.set_side_to_move()` internally). Protocol handler needs mutable access.
3. `position_history` grows unbounded — not a concern for Stage 4 (no search), but noted.
4. DKW instant moves happen inside `apply_move()` — protocol sees them in `MoveResult::dkw_moves`.
5. `MoveResult` returns points, eliminations, DKW moves, game_ended — useful for info output.

**From [[downstream_log_stage_02]]:**
1. Attack query API is the board boundary (ADR-001). Protocol does not need attack queries directly.
2. `Move::to_algebraic()` returns move notation format: `d2d4`, `e7e8q` (promotion lowercase).
3. No `Move::from_algebraic()` exists — must match algebraic strings against legal move list.

**From [[downstream_log_stage_01]]:**
1. `Board::from_fen4(fen) -> Result<Board, Fen4Error>` for FEN4 parsing. `Fen4Error` has `Display` impl.
2. `Board::starting_position()` for standard setup.
3. `Fen4Error` is not currently re-exported from `board/mod.rs` — need to add re-export.

**From [[downstream_log_stage_00]]:**
1. `huginn_observe!` macro available. Stage 4 gates will be deferred per established pattern.

**From [[MOC-Active-Issues]]:**
- WARNING: [[Issue-Perft-Values-Unverified]] — not blocking Stage 4.
- NOTE: [[Issue-Huginn-Gates-Unwired]] — will accumulate 4 more gates this stage.
- NOTE: [[Issue-DKW-Halfmove-Clock]] — not relevant to protocol layer.

### Risks for This Stage

1. **FEN4 parsing edge cases (Section 2.23):** Malformed FEN4 input must produce descriptive errors, never panics. `Board::from_fen4()` already handles this via `Result`, but protocol must wrap errors gracefully.
2. **Move string matching correctness:** Matching user-supplied move strings against `to_algebraic()` output must handle all notation edge cases (double-digit ranks like `k14`, promotions). Relying on legal move list comparison avoids duplicating movegen logic.
3. **`GameState::apply_move()` panics on game over:** Protocol must guard against applying moves after game ends.
4. **Stdin/stdout blocking:** Protocol loop blocks on stdin. `stop` command cannot interrupt `go` in Stage 4 (no threading). Acceptable since `go` is instantaneous (random move).
5. **API surface creep (Section 2.24):** Protocol module should expose minimal public API — just `OdinEngine` and types needed by `main.rs`.


---

## Post-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| Command parser (odin, isready, setoption, position, go, stop, quit) | Done | `parser.rs` — `parse_command()` handles all variants. 23 unit tests. |
| Response emitter (id, readyok, bestmove, info) | Done | `emitter.rs` — pure formatting functions. 8 unit tests. |
| 4PC extensions (wtime/btime/ytime/gtime, v1-v4, phase, brs_surviving, mcts_sims) | Done | `SearchLimits` struct, `SearchInfo` struct with optional fields. |
| Position setting (FEN4 + startpos + move list) | Done | `handle_position_fen4`, `handle_position_startpos`, `apply_moves`. |
| `go` stub (random legal move) | Done | LCG-based random selection from legal moves. Sends info + bestmove. |
| Full command set | Done | All 8 commands handled. Unknown commands produce error. |
| Main loop (stdin reader) | Done | `OdinEngine::run()` with `BufRead`. EOF exits cleanly. |
| Acceptance: `odin` → id response | PASS | `test_odin_responds_with_id` |
| Acceptance: `isready` → readyok | PASS | `test_isready_responds_readyok` |
| Acceptance: Position set via FEN4 or startpos | PASS | `test_position_set_via_startpos`, `test_position_set_via_fen4` |
| Acceptance: `go` returns legal move | PASS | `test_go_returns_legal_move`, `test_go_bestmove_is_legal` |
| Acceptance: Malformed input handled gracefully | PASS | `test_malformed_input_no_crash` |
| Permanent invariant: Protocol round-trip | PASS | `test_protocol_roundtrip_startpos`, `_fen4`, `_with_moves` |
| Huginn gates | Deferred | Per established pattern — unwired until telemetry needed |

### Code Quality
#### Uniformity
PASS. Protocol module follows the same patterns as board and movegen: private submodules (`parser.rs`, `emitter.rs`, `types.rs`) with explicit `pub use` re-exports in `mod.rs`. Naming is consistent: `parse_command`, `format_bestmove`, `handle_odin`, `handle_go`. All types use PascalCase (`Command`, `SearchLimits`, `OdinEngine`), functions use snake_case, constants use SCREAMING_SNAKE (`ENGINE_NAME`, `ENGINE_VERSION`).

#### Bloat
PASS. No unnecessary abstractions. No trait objects, no generics, no builder patterns. The `output_buffer: Vec<String>` is the simplest possible approach for test output capture. Total new code: ~400 lines of implementation + ~350 lines of tests across 4 files.

#### Efficiency
PASS. No performance targets for Stage 4. Move matching against legal moves is O(n) per move string — irrelevant since it runs once per `position` command. The protocol loop is blocking, appropriate for the current single-threaded design.

#### Dead Code
PASS. Zero clippy warnings. All public items are used either by `main.rs` or by tests. `Fen4Error` re-export added to `board/mod.rs` — not yet consumed externally but is the correct API surface.

#### Broken Code
PASS. All 229 tests pass (156 unit + 73 integration). No panics, no unwrap-on-None in production code. All `unwrap()` calls are in test code only. Error paths in the protocol use `format_error()` and continue.

#### Temporary Code
PASS. The `go` handler returns a random move — this is the specified behavior for Stage 4, not temporary code. It will be replaced by actual search in Stage 7 via the `Searcher` trait, but the protocol handler structure (`handle_go` calling a search method) will persist.

### Search/Eval Integrity
N/A for Stage 4. No search or evaluation code. The `go` command returns a random legal move per spec.

### Future Conflict Analysis
1. **Stage 5 (Basic UI):** Will spawn engine as child process, communicating via stdin/stdout with Odin Protocol. Protocol is ready — `OdinEngine::run()` reads stdin and writes stdout.
2. **Stage 7 (Plain BRS):** Will replace the random-move `go` stub with actual search. The `handle_go` method structure is ready — just replace `random_legal_move()` with a call through the `Searcher` trait.
3. **Stage 11 (Hybrid Integration):** May need to extend `info` strings with additional fields. `SearchInfo` is designed for this — all fields are optional, new fields can be added without breaking existing output.
4. **Stage 13 (Time Management):** `SearchLimits` already captures `wtime/btime/ytime/gtime`, `depth`, `nodes`, `movetime`, `infinite`. The time manager will consume these directly.
5. **Threading:** Stage 7+ may need `go` to run in a separate thread so `stop` can interrupt. Current `handle_go` is synchronous. The handler method is self-contained and could be dispatched to a thread. `stop` is currently a no-op.

### Unaccounted Concerns
1. **NOTE:** The `position` command with `moves` uses `GameState::apply_move()` which tracks scores, eliminations, etc. This means position setup via moves is fully game-aware. If a future stage needs lightweight position setup (no scoring/elimination tracking), it would need to use make_move directly instead.
2. **NOTE:** The protocol does not implement `ucinewgame` — sending a new `position` command effectively resets the game state. This is consistent with UCI convention where `ucinewgame` is optional.

### Reasoning & Methods
1. Built incrementally: types → parser → emitter → engine → main loop → integration tests
2. All code tested with `cargo test` (229 tests), `cargo clippy --all-targets` (0 warnings), `cargo fmt`
3. Verified `cargo build --features huginn` compiles (protocol code has no Huginn dependency)
4. Tested move matching by verifying bestmove output is in the legal move list
5. Tested error resilience by sending garbage inputs and verifying no panics
6. Verified prior invariants: perft values (20/395/7800), all prior-stage tests pass

### Issue Resolution
- WARNING [[Issue-Perft-Values-Unverified]]: Still open, not relevant to Stage 4. Perft values verified unchanged.
- NOTE [[Issue-Huginn-Gates-Unwired]]: Now accumulates 4 more gates from Stage 4 (command_receive, response_send, position_set, search_request). Still deferred.
- NOTE [[Issue-DKW-Halfmove-Clock]]: Unchanged, not relevant to protocol layer.


---

## Related

- Stage spec: [[stage_04_protocol]]
- Downstream log: [[downstream_log_stage_04]]
