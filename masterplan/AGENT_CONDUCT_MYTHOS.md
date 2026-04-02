# AGENT CONDUCT -- Project Odin (Mythos Edition)

**Derived from:** `AGENT_CONDUCT.md` v1.0
**For:** Mythos/frontier-class models
**Principle:** Less procedure. Same knowledge. Same boundaries.

> Standard models: use `AGENT_CONDUCT.md`.

Three core reference documents:

| Document | Defines | Authority Over |
|----------|---------|---------------|
| `MASTERPLAN.md` | WHAT each stage builds | Stage specs, acceptance criteria, architecture, tracing points |
| `4PC_RULES_REFERENCE.md` | The game rules | Board layout, piece movement, scoring, game modes |
| `AGENT_CONDUCT_MYTHOS.md` (this) | HOW agents work | Behavior rules, audit procedures, code standards |

---

## 1. BEHAVIOR RULES

---

### 1.1 Stage Entry

Before writing code: understand project state (STATUS.md + HANDOFF.md), read the stage spec in MASTERPLAN.md Section 4 (deliverables, build order, acceptance criteria, "What you DON'T need"), review upstream audit logs and downstream logs for blocking findings and API contracts, verify `cargo build && cargo test` pass, and research the work ahead -- especially 4PC-specific gotchas where direct literature is scarce (look to adjacent fields: multi-agent game theory, RTS AI, influence mapping). Record findings in the pre-audit section of `audit_log_stage_XX.md` before implementing.

> **SYSTEM_PROFILE.local.md** is a gitignored, machine-specific file in `masterplan/`. Contains CPU/RAM/GPU specs and their implications for build times, memory budgets, and feature feasibility. If missing, ask the user for their specs. Never commit it.

---

### 1.2 Search Depth Policy

**Only depths divisible by 4 are valid search depths.** In 4PC, each player takes one ply per round. A non-multiple-of-4 depth biases evaluation toward the last-moving player.

| Depth | Status |
|-------|--------|
| 4 | Minimum acceptable. One full round of play. |
| 8 | Maximum practical on current hardware. |
| 12+ | Not feasible. Do not target. |
| 1, 2, 3, 5, 6, 7 | **Never use** in production, tests, self-play, benchmarks, or documentation. |

The only exception: internal iterative deepening loops where intermediate depths are stepping stones toward a depth-4 or depth-8 target -- but intermediate results must never be treated as final.

This constraint applies everywhere: engine defaults, test assertions, self-play configs, observer baselines, training datagen, and documentation examples.

---

### 1.3 The First Law: Do Not Break What Exists

Every commit leaves the project compilable and test-passing. No exceptions.

**Permanent invariants** (must pass after every stage, forever -- full table in MASTERPLAN.md Section 4.1):

| Stage | Invariant |
|-------|-----------|
| 0 | Prior-stage tests never deleted |
| 2 | Perft values are forever. Zobrist make/unmake round-trip. Attack query API is the board boundary -- nothing above Stage 2 reads `board.squares[]` directly. |
| 3 | Game playouts complete without crashes |
| 5 | UI owns zero game logic |
| 6 | Evaluator trait is the eval boundary |
| 7 | Searcher trait is the search boundary. Engine finds forced mates. |

If a Stage N change causes a Stage M (M < N) test to fail: **blocking defect**. Fix before proceeding.

If existing behavior genuinely needs to change: document it in the audit log, update all downstream consumers, verify no test regressions, flag in the downstream log.

---

### 1.3 Dependency Handling

Consume APIs through traits. Do not reach into module internals. Respect the "What you DON'T need" sections in stage specs -- do not build things listed there. If a needed dependency is missing or broken: stop and document in the audit log as a blocking issue. Do not implement workarounds that duplicate prior-stage responsibilities.

Stub contracts: preserve existing function signatures, parameter types, and return types. Document needed signature changes in the downstream log before modifying.

---

### 1.4 Autonomy Boundaries

**Proceed autonomously when:**

| Situation |
|---|
| Implementing something explicitly described in the stage spec |
| Writing tests for behavior described in the spec |
| Fixing a clear defect (test fails, panic on valid input, wrong output for documented behavior) |
| Adding tracing instrumentation at key boundaries |
| Refactoring internal implementation without changing the public API |

**Stop and ask when:**

| Situation |
|---|
| The stage spec is ambiguous or contradicts another document |
| A change would modify a public API established by a prior stage |
| A decision has long-term architectural implications not covered in the spec |
| Performance is significantly worse than what the spec implies |
| You discover a bug in a prior stage requiring non-trivial changes |
| You want to add functionality not mentioned in the spec, even if it seems helpful |
| Any change to `Cargo.toml` dependencies beyond what the spec requires |

---

### 1.5 Code Standards

**Naming conventions:**

| Entity | Convention | Example |
|--------|-----------|---------|
| Rust modules | snake_case | `move_gen`, `board_repr` |
| Rust types | PascalCase | `GameState`, `MctsNode` |
| Rust functions | snake_case | `generate_legal_moves` |
| Rust constants | SCREAMING_SNAKE | `MAX_DEPTH`, `TACTICAL_MARGIN` |
| UI components | PascalCase | `BoardDisplay`, `DebugConsole` |
| Protocol commands | lowercase | `bestmove`, `isready` |

No mixed conventions within a module. Glossary terms from MASTERPLAN Section 7 are authoritative -- use them everywhere in the appropriate Rust-casing form.

Run `cargo fmt` and `cargo clippy` (zero warnings) before every commit. Suppress a clippy lint only with a comment explaining why.

Default to private. Every `pub` item is a contract future stages may depend on. Document all `pub` items with what-not-how doc comments.

**Named constants over magic numbers:**

| Literal | Named Constant |
|---------|---------------|
| `36` | `INVALID_CORNER_COUNT` |
| `196` | `TOTAL_SQUARES` |
| `160` | `VALID_SQUARES` |
| `14` | `BOARD_SIZE` |
| `150` | `TACTICAL_MARGIN` |
| `900` | `QUEEN_EVAL_VALUE` |

---

### 1.6 Commit Discipline

- One commit per build-order item or logically atomic change.
- Commit messages reference the stage: `[Stage 02] Implement pseudo-legal pawn generation for all four directions`
- Never commit code that does not compile or has failing tests. Large changes that break things temporarily go on a feature branch (`stage-XX-feature-name`); merge when stable.

---

### 1.7 Test Rules

- Every acceptance criterion must have at least one test.
- Prior-stage tests are never deleted or modified to pass new code. If a prior test is genuinely wrong, document it in the audit log and fix as a separate commit.
- Unit tests: `#[cfg(test)] mod tests` in the module they test. Integration tests: `odin-engine/tests/`.
- Test names describe what they test: `test_checkmate_detected_when_no_legal_moves_and_in_check`.
- Perft tests run in CI and are never skipped.
- Every fixed bug gets a regression test. Never fix a bug without a test.

---

### 1.8 Decision Principles

When trade-offs arise, in priority order:

1. **Correctness before performance**, except where the spec defines explicit performance targets (those ARE correctness requirements).
2. **"Reasonable" means within 10x of the target** in MASTERPLAN or downstream logs. At 8s when spec says 5s: acceptable for now. At 50s: it's a bug.
3. **If self-play shows regression, revert and document.** Minimum 100 games at 1-minute time control before declaring regression or improvement. Use SPRT when available (Stage 12+).
4. **Under-engineer rather than over-engineer.** The spec is the scope. Three similar lines of code are better than a premature abstraction.
5. **When the spec is silent, prefer the simpler approach.**
6. **Record non-obvious decisions in `DECISIONS.md`.** If you chose A over B and someone might later wonder why, write it down.

---

### 1.9 Issue Lifecycle

**Staleness rule:** Every session, scan [[MOC-Active-Issues]]. Any open Blocking or Warning issue with `last_updated` older than 3 sessions: update it, resolve it, or document what's blocking it. Note-level issues are exempt until their relevant stage begins.

**Issue creation checklist:**
1. Create file in `issues/` using `_templates/issue.md`. Fill all fields, especially `severity`, `stage`, `last_updated`.
2. Add to [[MOC-Active-Issues]] under the correct severity heading.
3. Use existing [[wikilinks]] from [[Wikilink-Registry]]. If a new target is needed, add it to [[Wikilink-Registry]] immediately.

**Issue resolution:**
1. Fill `## Resolution`. Set `status: pending-verification`. Do NOT set `resolved` yet.
2. **Never claim a bug is fixed or mark `resolved` until the user verifies through self-play or manual testing.** Passing tests is necessary but not sufficient.
3. After user confirms: set `status: resolved`, `date_resolved`, move entry to `## Recently Resolved` in [[MOC-Active-Issues]].

---

### 1.10 Blocking Issue Resolution

| Context | Action |
|---|---|
| Blocking found pre-audit (before starting a new stage) | Fix it first. Record fix in prior stage's audit log. Re-run all prior-stage tests. Update STATUS.md. |
| Blocking found post-audit (after completing a stage) | Fix before marking complete. Re-run full post-audit after fix. |
| Blocking found 3+ stages back | **Escalate to human oversight.** Document impact chain. Do not fix autonomously. |
| Auditor marks blocking, implementor disagrees | Blocking stands until human resolves it. Err on the side of caution. |

---

### 1.11 Version Control

Tag each completed stage after post-audit passes: `stage-00-complete` + `v1.0`, `stage-01-complete` + `v1.1`, etc. Tags are permanent -- never moved or deleted.

Version scheme: `v1.N` = Stage N complete. Major bump (`v2.0`) only if a rollback forces a rebuild from an earlier stage.

What gets versioned: all engine/UI/training code, all masterplan documents, README/STATUS/HANDOFF/DECISIONS, .gitignore. Not: build artifacts, node_modules, NNUE weight files >10MB.

---

### 1.12 Wikilink Discipline

1. **Reuse before creating.** Check [[Wikilink-Registry]] first. Use targets exactly as written (case-sensitive).
2. **New targets require registry updates.** Add immediately: target name, file path, category, one-line purpose.
3. **No orphan links.** Every `[[target]]` must resolve to an actual file.
4. **No duplicate targets for the same concept.** One concept = one canonical wikilink target.
5. **Log entries get wikilinks.** Audit logs, downstream logs, session notes, issue notes -- link to related specs, components, and decisions.
6. **Registry maintenance.** If a file is renamed or deleted, update [[Wikilink-Registry]] immediately.

---

### 1.13 Vault Note Protocol

Create vault notes as you work, not retroactively:

| Trigger | Note type | Folder | Example |
|---|---|---|---|
| Every WARNING or BLOCKING audit finding | Issue | `issues/` | `Issue-EP-Representation-4PC.md` |
| Every component implemented or substantially modified | Component | `components/` | `Component-Board.md` |
| Every cross-component interaction discovered or built | Connection | `connections/` | `Connection-Board-to-MoveGen.md` |
| Every non-obvious pattern or trick worth reusing | Pattern | `patterns/` | `Pattern-Pawn-Reverse-Lookup.md` |

Every vault note must: use the template from `_templates/`, link to relevant stage specs and logs via existing [[wikilinks]], be added to [[Wikilink-Registry]], and be added to the relevant MOC.

Tags in vault notes use frontmatter `tags:` for broad categories only (`stage/02`, `severity/warning`, `area/movegen`). Wikilinks connect concepts; tags group them.

---

### 1.14 Session End

Update HANDOFF.md (what was done, what wasn't, open issues, files modified, what's next) and STATUS.md (current stage, completion tracker, next-session priorities). Update DECISIONS.md if architectural decisions were made. Create vault notes per §1.13. Create a session note in `masterplan/sessions/`. Commit all management file updates as the last commit of the session.

---

### 1.15 Debugging

1. **Implement the user's diagnosis first.** Test it. Investigate alternatives only if it fails. Cost of trying: minutes. Cost of ignoring: the context window.
2. **Each analysis pass must surface something new.** Cite a specific new code location (file:line) or eliminate a prior hypothesis. If your hypothesis is identical to the last pass and nothing is new: stop analyzing and write a test.
3. **After two consecutive passes without narrowing: write a test.** A minimal reproduction is sufficient. Let the output guide the next pass.
4. **One bug, one focus.** When Bug B surfaces while investigating Bug A, write B down (code comment or issue note) and continue with A. Exception: Bug B actively blocks testing Bug A's fix.
5. **Scope lock after diagnosis.** Once you have a specific code location and a concrete fix plan, implement. Do not pre-analyze edge cases -- let tests reveal them.

---

### 1.16 Deferred-Debt Escalation

If any work item has been deferred for 2 or more consecutive stages:

1. Flag in HANDOFF.md under `## Deferred Debt`: what it is, how many stages deferred, WHY it's stuck (the actual blocker), what would unblock it, whether the design may be flawed.
2. Promote issue severity: NOTE → WARNING after 2 stages; BLOCKING or explicit abandonment decision in DECISIONS.md after 3.
3. Tell the user directly.

Intentional dependency ordering per the build order is NOT deferral. "Will wire in the next stage" applied repeatedly to the same item IS.

---

### 1.17 Task Files

For any work involving investigation or regression risk, create a task file in `masterplan/tasks/` from `_templates/task.md`. Name it `Task-Short-Name_in_progress.md`; rename to `Task-Short-Name_complete.md` when done. Add to [[MOC-Tasks]].

**Before writing code**, fill Section 1 (Understanding Check): files read, problem statement, constraints, prior attempts. If you cannot fill this with specific file paths and constraints, read more first.

| Situation | Task File? |
|---|---|
| Multi-file code change with investigation | YES |
| Bug fix requiring root cause analysis | YES |
| Eval tuning or search parameter change | YES |
| Performance retrofit | YES |
| One-line typo fix | NO |
| Adding a single test | NO |
| Documentation-only update | NO |

---

### 1.18 Diagnostic Gameplay Observer

**ONLY the top-level orchestrating agent** may start the engine, run `cargo build`, or run diagnostic games. Subagents must not independently start/stop the engine, run builds, or modify engine state while another agent is working. Confirm no other agent is actively compiling before building.

Engine protocol logging: `setoption name LogFile value observer/logs/<name>.log`. Format: incoming `> ...`, engine responses `< ...`. Zero overhead when disabled.

Max Rounds slider (UI, 0--50): set before Full Auto for bounded diagnostic windows.

| Situation | Run diagnostic? |
|---|---|
| After eval tuning or search changes | YES |
| After completing a stage | YES |
| Investigating a reported behavioral bug | YES |
| Routine code cleanup or refactor | NO |

Log file naming: `observer/logs/ffa_standard_d6_20rounds_2026-02-27.log` (game mode, config, depth, rounds, date).

---

## 2. AUDIT CHECKLIST

An audit is adversarial review. The auditor assumes bugs exist and tries to find them. The checklist is a minimum -- if something looks wrong, flag it even if it doesn't fit a category.

**Severity levels:**
- **BLOCKING** -- Must fix before the next stage begins.
- **WARNING** -- Should fix. Will likely cause problems in a future stage.
- **NOTE** -- Observation for the record. No action required now.

---

### 2.1 Cascading Issues

A change in module A breaks module B which breaks module C.

**What to look for:**
- Did any function signatures change? Are all callers updated?
- Did any enum variants get added or removed? Is every `match` exhaustive?
- Did any struct fields change? Are all constructors and destructors consistent?
- Did return types change? Are all consumers expecting the new type?
- Did error types change? Are all `?` propagation chains still valid?
- Trace every changed public API downstream through the dependency map (MASTERPLAN Appendix A). For each consumer, verify it still compiles and behaves correctly.

**Odin-specific:** Changes to `Board`, `Piece`, `Player`, `Move`, `GameState`, or `MoveUndo` cascade to nearly everything. Any change to these types requires a full downstream trace.

---

### 2.2 Iterative Degradation

Something that gets worse each time it is touched.

**What to look for:**
- Functions that have grown beyond 50 lines through repeated additions.
- Structs that have accumulated fields beyond their original purpose.
- Comments that are stale (describe behavior from 3 commits ago).
- Workaround layers: code that patches around a prior workaround. More than one level of workaround is a red flag.
- Test bloat: test helper functions copied and slightly modified rather than generalized.
- "Temporary" code that has survived multiple stages. If something was marked temporary in Stage 2 and it is now Stage 7, it is technical debt. Either make it permanent (with proper design) or remove it.

---

### 2.3 Code Bloat

More code than necessary for the same result.

**What to look for:**
- Verbose patterns that could be replaced with idiomatic Rust (manual loops vs. iterators, manual error checks vs. `?` operator).
- Duplicate logic that could be shared (two functions that do nearly the same thing for different piece types).
- Over-abstraction: trait hierarchies or generic parameters serving only one concrete type. If there is only one `impl`, you do not need the trait yet (exception: `Evaluator` and `Searcher` traits are defined before their second implementor, by design).
- Builder patterns, factory functions, or configuration objects for things that could be a simple constructor.
- Excessive logging or debug output.

---

### 2.4 Redundancy

Two or more pieces of code that accomplish the same thing.

**What to look for:**
- Multiple implementations of square validity checking.
- Multiple implementations of player turn ordering.
- Multiple definitions of piece values (one in eval, one in SEE, one in move ordering -- they should all reference one authoritative source).
- Multiple ways to convert between coordinate systems.
- Constants defined in more than one place.
- Utility functions that duplicate standard library functionality.

---

### 2.5 Dead Code

Code that exists but is never executed.

**What to look for:**
- Functions that are defined but never called (`cargo clippy` catches most of these).
- Enum variants that are never constructed.
- Match arms that can never be reached.
- `#[allow(dead_code)]` annotations. Each one is a question: why does this dead code exist?
- Imported modules or crates that are not used.
- Feature-gated code that is not behind the correct feature flag.
- Test helpers that are not used by any test.

---

### 2.6 Broken Code

Code that compiles but produces incorrect results.

**What to look for:**
- Logic errors that tests do not cover (the most dangerous category).
- **Off-by-one errors** in the 14x14 board with 36 invalid corners. The index formula `rank * 14 + file` maps to 196 total squares, but only 160 are valid. Every function that iterates over squares must handle this.
- **Boundary conditions:** rank 0, rank 13, file 0, file 13, the edges adjacent to invalid corners.
- **Pawn direction bugs:** Red goes +rank, Blue goes +file, Yellow goes -rank, Green goes -file. A single sign error means one player's pawns move backward.
- **Zobrist hash errors:** XOR is its own inverse, so forgetting to XOR out a piece before XOR-ing in a new piece at the same square corrupts the hash silently.
- **Integer overflow:** centipawn scores near `i16::MAX` or `i16::MIN`, especially during alpha-beta with mate scores. Negation of `i16::MIN` overflows.
- **Floating point comparison:** `==` on floats in MCTS value comparisons is unreliable.

---

### 2.7 Stale References

Code that references things that no longer exist or have changed shape.

**What to look for:**
- Function calls with wrong argument count or types (usually caught by the compiler, but generic code or macro-generated code can hide this).
- Struct field access on fields that were renamed or removed.
- Comments referencing functions, types, or behaviors that no longer exist.
- Import statements for modules or items that were deleted.
- Configuration or option names that no longer match the code that reads them.
- Protocol command handling that references deprecated command formats.

**Odin-specific:** When the bootstrap eval is replaced by NNUE (Stage 16), every reference to bootstrap eval behavior in comments and docs must be updated or removed. The function signatures stay the same (Evaluator trait), but documentation that says "handcrafted eval" becomes stale.

---

### 2.8 Naming Inconsistencies

Uppercase/lowercase mixing, different terms for the same concept, abbreviation inconsistency.

**What to look for:**
- Mixed casing within the same conceptual domain: `move_gen` in one module, `MoveGen` as a function name in another.
- Abbreviation inconsistency: `sq` vs. `square` vs. `sqr`. Pick one per context.
- Concept naming drift: `position` vs. `board` vs. `state` used interchangeably when they mean different things. `Board` is the piece layout. `GameState` is the full game. Define `position` once and use it consistently.
- Player naming: `Red`/`Blue`/`Yellow`/`Green` vs. numeric indices (0, 1, 2, 3). The enum should be `Player::Red`, etc. Numeric indices should only appear in array indexing, never in logic.

---

### 2.9 Conflicting Code

Two pieces of code that contradict each other.

**What to look for:**
- Two different values for the same constant. Note: pawn value = 100cp in eval and pawn value = 1 point in scoring are intentionally different in 4PC, but they must be clearly named (`PAWN_EVAL_VALUE` vs. `PAWN_CAPTURE_POINTS`).
- Two functions that claim to compute the same thing but produce different results.
- A function's doc comment says it does X but the implementation does Y.
- A trait contract (documented invariants) that an implementor violates.
- Two modules with circular assumptions: A assumes B runs first, B assumes A runs first.
- Defensive code that clamps a value to a range that a producer guarantees it will never exceed (either the producer guarantee or the defensive clamp is unnecessary -- determine which).

---

### 2.10 Before-and-After Audit

Comparing the codebase before and after stage work to catch things that logs miss.

**Procedure:**
1. Before work begins, record: full `cargo test` output (test count, all pass), binary size from `cargo build --release`, public API surface of the stage's module (all `pub fn`, `pub struct`, `pub enum`, `pub trait`).
2. After work completes, record the same metrics.
3. Compare: test count should increase; public API should contain only what the spec requires; all pre-existing tests still pass.
4. Diff review: read the full diff. Look for unintentional changes to files outside the current stage's module.

---

### 2.11 Trait and Interface Contract Violations

Code that implements a trait but violates semantic contracts.

**What to look for:**
- `Clone` implementations that do not produce independent copies (shared mutable state through `Rc<RefCell<>>`). Critical for MCTS which clones `GameState` for simulations.
- `PartialEq`/`Eq` implementations inconsistent with `Hash` (two values that are `eq` must have the same hash).
- `Ord` implementations that are not total (`NaN` in float comparisons).
- Custom `Display` or `Debug` implementations that omit important fields.
- The `Evaluator` trait dual-output contract: `eval_scalar` and `eval_4vec` must be consistent. If `eval_scalar` says position is +200cp for Red, `eval_4vec` should show Red's component as relatively high.

---

### 2.12 Unsafe Unwraps and Panics

Code paths that can crash the engine.

**What to look for:**
- `.unwrap()` on `Option` or `Result` in production code. Replace with `.expect("descriptive context")` at minimum, or proper error handling.
- `.expect()` without meaningful context. `expect("failed")` is nearly as bad as `unwrap()`.
- Array indexing without bounds checks on user-derived indices (square indices from FEN4 parsing, move encoding/decoding).
- Division by zero (visit count = 0 in UCB1 calculation, time remaining = 0 in time management).
- Recursion without depth limits (BRS search, MCTS selection).

---

### 2.13 Test Coverage Gaps

Functionality that exists but has no tests.

**What to look for:**
- Every acceptance criterion in the stage spec must have at least one corresponding test.
- Edge cases specific to 4PC: 3 players simultaneously attacking one king, all 4 players with no legal moves, maximum number of pieces on board, empty board (all captured), terrain blocking every exit.
- Error paths: malformed FEN4 strings, illegal moves via protocol, corrupt Zobrist hash detection.
- Regression tests: when a bug is found and fixed, the test that reproduces the bug must exist. Never fix a bug without a test.
- Integration tests: do stages work together, not just in isolation?

---

### 2.14 Performance Regressions

Something that was fast becoming slow.

**What to look for:**
- Perft nodes-per-second as a baseline after Stage 2. If NPS drops by more than 10% in any later stage, investigate.
- Eval calls per second as a baseline after Stage 6.
- BRS nodes per second as a baseline after Stage 7.
- MCTS simulations per second as a baseline after Stage 10.
- Allocations in hot paths: `Vec::push` in movegen (should pre-allocate), `Box::new` in MCTS node creation (should use arena after Stage 19), `String` creation during search.
- Unnecessary cloning of large structs.
- Hash table operations that degrade as the table fills up.

---

### 2.15 Memory Concerns

Memory leaks, unbounded growth, and allocation patterns.

**What to look for:**
- MCTS tree growth: if the tree is never pruned or reused between searches, memory usage grows without bound across a game. Verify that MCTS tree memory is bounded.
- Position history in `GameState.position_history: Vec<u64>`: grows every move, never shrinks. In long games (200+ moves), consider bounded storage.
- Transposition table: must be a fixed-size allocation, not growing.
- Rust-specific: `Rc` cycles, `Arc` without weak references where cycles are possible, `Box<dyn Trait>` in collections that grow without bound.

---

### 2.16 Feature Flag Contamination

Feature-gated code leaking into default builds.

**What to look for:**
- Any feature-gated type, function, or import appearing outside its `#[cfg(feature = "...")]` blocks.
- Tests that only pass with a specific feature flag enabled. All default tests must pass without optional flags.
- Binary size comparison: track `cargo build --release` across stages. Unexpected growth may indicate unnecessary dependencies.

---

### 2.17 Board Geometry Errors

Errors specific to the non-standard 14x14 board with 36 invalid corners.

**What to look for:**
- Off-by-one in corner exclusion. The 4 corners of 9 squares each (see `4PC_RULES_REFERENCE.md` for exact coordinates). Verify the exact sets.
- Ray generation for sliding pieces: rays must stop at board edges AND at corner boundaries. A bishop on d4 sliding toward a1 hits the invalid corner zone and must stop.
- Knight moves from squares adjacent to corners: some destination squares that would be valid on a standard 14x14 board are invalid.
- Pawn double-step: for players on the wing sides (Blue, Green), the double-step path may pass through or land on invalid squares near corners.
- FEN4 parsing: the serialized format must handle invalid squares correctly. Verify round-trip correctness.

---

### 2.18 Zobrist Hash Correctness

The integrity of incremental Zobrist hashing after any code change.

**What to look for:**
- **Make/unmake round-trip:** For any move, `hash_before == hash_after_make_then_unmake`. Test exhaustively.
- **Incremental vs. full recomputation:** `compute_full_hash(board) == board.zobrist` at every observation point.
- **Castling rights:** 8 bits (2 per player), each with a unique Zobrist key. When castling rights change (king move, rook move, rook capture), old rights must be XOR'd out, new rights XOR'd in.
- **En passant:** When set, its Zobrist key is XOR'd in. When cleared (next move), XOR'd out. If a new en passant square is set, old one must be cleared first.
- **Side to move:** XOR'd on every turn transition. With 4 players, the key depends on which player's turn it is, not just a toggle.

---

### 2.19 Thread Safety Preparation

Preparing for future multithreading even in single-threaded stages.

**What to look for:**
- Global mutable state (`static mut`, `lazy_static` with interior mutability). These prevent future parallelization.
- `Rc` usage (not `Send`). MCTS nodes should use owned data or `Arc` if references are needed.
- The transposition table should be designed with eventual concurrent access in mind. Don't design it in a way that prevents lockless reads later.
- Random number generators: use per-thread RNG, not a shared global.

---

### 2.20 Import and Dependency Bloat

Unnecessary crates or overly broad imports.

**What to look for:**
- `use module::*` glob imports.
- External crates pulled in for a single function that could be trivially implemented.
- `Cargo.toml` dependencies used in only one stage's code but affecting the entire build.
- Feature flags on dependencies that pull in more than needed.

---

### 2.21 Circular Dependencies

Modules that depend on each other, creating coupling.

**What to look for:**
- Module A imports from Module B and Module B imports from Module A.
- The architecture is layered: Board -> MoveGen -> GameState -> Search -> Eval. Each layer should depend only on layers below it. If eval imports from search, that is a circular dependency.
- Exception: tracing instrumentation can observe any module. But tracing configuration must never be a dependency OF any module -- no engine logic imports from tracing configuration.

---

### 2.22 Magic Numbers

Literal numeric values with unclear meaning.

**What to look for:**
- Any numeric literal in logic code that is not 0 or 1 needs a named constant.
- Score values in search: `i16::MAX - depth` for mate scores should use `MATE_SCORE`.
- Array sizes: `[Option<Piece>; 196]` should be `[Option<Piece>; TOTAL_SQUARES]`.
- Timing constants: `100` ms minimum time should be `MIN_MOVE_TIME_MS`.

---

### 2.23 Error Handling Gaps

Where the engine can fail ungracefully.

**What to look for:**
- Protocol input parsing: any invalid input from stdin must produce an error message, not a panic.
- FEN4 parsing: invalid FEN4 strings must produce descriptive errors, not panics.
- File I/O: NNUE weight file loading must handle missing files, corrupt files, version mismatches, and truncated files.
- Numeric overflow: centipawn scores during search can overflow i16 if not clamped.
- Resource exhaustion: TT should replace entries not grow; MCTS should stop expanding not crash.

---

### 2.24 API Surface Area Creep

Public interfaces growing beyond what is necessary.

**What to look for:**
- Count `pub` items in each module after each stage. If a stage that should add 5 public functions added 15, investigate.
- Helper functions made `pub` for testing convenience but not part of the module's contract. Use `pub(crate)` instead.
- Internal types that leaked to `pub` because a `pub fn` returns them.

---

### 2.25 Documentation/Code Drift

Documentation that describes behavior the code no longer exhibits.

**What to look for:**
- Doc comments on functions that were correct when written but are stale after refactoring.
- MASTERPLAN references to code structures that have changed.
- Downstream log entries that describe API contracts that were subsequently modified.
- Inline comments explaining "why" that reference conditions that no longer exist.
- Tracing instrumentation in the MASTERPLAN that does not match the actual observation points in code.

---

### 2.26 Semantic Correctness

Code that compiles and passes existing tests but is logically wrong in ways not yet tested.

**What to look for:**
- **Evaluation symmetry:** The same position evaluated from Red's perspective and from Yellow's perspective (with colors swapped) should produce symmetric results. Asymmetry indicates a perspective bug.
- **Move generation completeness:** Verify not just that generated moves are legal, but that ALL legal moves are generated. Perft catches aggregate errors; individual position tests also needed.
- **Score consistency:** If BRS evaluates a move at +300cp and MCTS evaluates the same move as terrible, either BRS or MCTS has a bug (or `eval_scalar` and `eval_4vec` are inconsistent).
- **Game rule edge cases from `4PC_RULES_REFERENCE.md`:** Check confirmed only at the affected player's turn, DKW king captures worth 0 points, promoted queen worth 1 point on capture but 900cp in eval. Each of these is a potential mismatch between rules reference and implementation.

---

## 3. OBSERVABILITY

The engine uses the `tracing` crate for structured logging and diagnostics. This replaced a custom compile-gated telemetry system (Huginn) retired in Stage 8 (see ADR-015).

- `tracing::debug!` -- search/eval diagnostic output
- `tracing::info!` -- high-level events (search start/complete, position set)
- `tracing::trace!` -- verbose per-node data (development only)
- All calls are zero-cost when filtered out at runtime

```
RUST_LOG=odin_engine=debug    # Development
RUST_LOG=odin_engine=info     # Normal operation
RUST_LOG=odin_engine=trace    # Verbose debugging
```

---

## 4. WHAT AUTOMATED TRACING CANNOT CATCH

Tracing records data. It does not understand design. These require human or agent judgment. Passing all tracing checks and tests does NOT mean the code is correct.

---

### 4.1 Architectural Drift

The codebase slowly deviates from the intended architecture in the MASTERPLAN.

**Examples:**
- The UI starts computing legal moves locally instead of asking the engine (violates "UI owns ZERO game logic").
- Eval starts importing from the search module, creating a circular dependency.
- GameState accumulates non-rule concerns (tracking search statistics, storing eval caches).

**How to catch it:** At every stage boundary, compare the actual module dependency graph against the MASTERPLAN architecture diagram (Section 2).

---

### 4.2 Wrong Abstractions

An abstraction makes some things easy but makes the right things hard.

**Examples:**
- Abstracting all four players into a generic "opponent iterator" that makes it hard to ask "which specific opponent is attacking me?" (critical for hybrid scoring in Stage 8).
- Using a trait for evaluation that prevents inlining of NNUE inference (performance-critical path).
- Over-generalizing move generation to handle hypothetical piece types that will never exist.

**How to catch it:** When implementing a downstream stage feels like fighting the abstractions from an upstream stage, the abstraction may be wrong. Document in the downstream log.

---

### 4.3 Over-Engineering

Building for hypothetical future needs that are not in the spec. If the spec does not mention it, do not build it.

---

*Derived from AGENT_CONDUCT.md v1.0 -- Less procedure. Same knowledge. Same boundaries.*
