# AGENT CONDUCT — AI/Agent Development Rules for Project Odin

**Version:** 1.0
**Created:** 2026-02-19
**Status:** Active

---

## 0. PREAMBLE

This document defines HOW AI agents behave while building Project Odin. It is one of three core reference documents:

| Document | Defines | Authority Over |
|----------|---------|---------------|
| `MASTERPLAN.md` ([[MASTERPLAN]]) | WHAT each stage builds | Stage specs, acceptance criteria, architecture, Huginn gate lists |
| `4PC_RULES_REFERENCE.md` ([[4PC_RULES_REFERENCE]]) | The game rules | Board layout, piece movement, scoring, game modes |
| `AGENT_CONDUCT.md` (this) | HOW agents work | Behavior rules, audit procedures, Huginn reporting, code standards |

**Every agent that touches the codebase must read this document before beginning any work.**

This document does not duplicate the masterplan. It references it. If this document says "see MASTERPLAN Section 4" it means go read that section there, not that the content is copied here.

---

## 1. AGENT BEHAVIOR RULES

---

### 1.1 Stage Entry Protocol

Before writing a single line of code for any stage, follow these steps in order. Skipping steps causes cascading problems that compound across stages.

**Step 0: Orient yourself.** Read `STATUS.md` ([[STATUS]]) to know where the project is. Read `HANDOFF.md` ([[HANDOFF]]) to know what the previous session was doing. Read `DECISIONS.md` ([[DECISIONS]]) if you're new to the project or working on a stage where architectural decisions were made. This takes 5 minutes and prevents you from duplicating work or re-arguing settled decisions.

**Step 1: Read the stage specification** in `MASTERPLAN.md` ([[MASTERPLAN]]) Section 4. Understand:
- What this stage builds (deliverables)
- Build order (sequential steps)
- Acceptance criteria (definition of done)
- Huginn gates (observation points to add)
- "What you DON'T need" (scope boundaries)

**Step 2: Read all upstream audit logs.** Trace the dependency chain from MASTERPLAN Appendix A. For every stage this one depends on (direct and transitive), read `audit_log_stage_XX.md`. Look for:
- BLOCKING findings that affect this stage
- WARNING findings that might be relevant
- Known limitations flagged by prior auditors

Example: Stage 8 depends on Stage 7, which depends on Stage 6, which depends on Stage 3, which depends on Stage 2, which depends on Stage 1, which depends on Stage 0. Read audit logs for stages 0, 1, 2, 3, 6, and 7.

**Step 3: Read all upstream downstream logs.** Same dependency chain. Read `downstream_log_stage_XX.md` for each. Look for:
- API contracts you must respect
- Known limitations you must work around
- Performance baselines you must not regress below
- Open questions that affect your stage

**Step 4: Build and test what exists.** Run:
```
cargo build
cargo test
cargo build --features huginn
```
If anything fails, STOP. Do not proceed with new work on a broken foundation. Record the failure in the pre-audit section of this stage's audit log.

**Step 5: Complete the pre-audit** section of `audit_log_stage_XX.md`. Record:
- Build state (compiles? tests pass?)
- Findings from upstream logs
- Risks identified for this stage

**Step 6: Begin implementation.**

---

### 1.2 The First Law: Do Not Break What Exists

Every commit must leave the project in a compilable, test-passing state. No exceptions. No "I'll fix it in the next commit."

**Permanent invariants** (once established, these must pass after every stage, forever):

The authoritative invariant table is in `MASTERPLAN.md` ([[MASTERPLAN]]) Section 4.1 (14 invariants, Stages 0-9). Consult that table for the full list. Key invariants for quick reference:

- **Stage 0:** Prior-stage tests never deleted. Huginn compiles to nothing when off.
- **Stage 2:** Perft values are forever. Zobrist make/unmake round-trip. Attack query API is the board boundary.
- **Stage 3:** Game playouts complete without crashes.
- **Stage 5:** UI owns zero game logic.
- **Stage 6:** Evaluator trait is the eval boundary.
- **Stage 7:** Searcher trait is the search boundary. Engine finds forced mates.

If a change in Stage N causes a test from Stage M (where M < N) to fail, that is a **blocking defect**. Fix it before proceeding.

If existing behavior genuinely needs to change (rare), the agent must:
1. Document exactly what changed and why in the audit log
2. Update all downstream consumers
3. Verify no test regressions
4. Flag it in the downstream log for future stages

---

### 1.3 Dependency Handling

**Consume APIs, do not peek at internals.** If Stage 7 needs evaluation, it calls `eval_scalar(position, player)` through the `Evaluator` trait. It does not reach into the eval module and read `piece_square_tables` directly.

**Respect the "What you DON'T need" sections.** Each stage spec explicitly says what it should NOT build. Do not build things listed there. Example: Stage 2 says "No move ordering." An agent working on Stage 2 must not add move ordering even if it seems easy.

**When a dependency is missing or insufficient:** If the current stage needs something from a prior stage that does not exist or does not work correctly, STOP. Document this in the audit log as a blocking issue. Do not implement a workaround that duplicates or contradicts prior-stage responsibilities.

**Stub contracts.** Some stages create stubs for later stages to fill (e.g., Stage 1 creates make/unmake stubs for Stage 2). The agent filling the stub must preserve the existing function signatures, parameter types, and return types. If the signature is wrong, document the needed change in the downstream log and get approval before modifying it.

---

### 1.4 Autonomy Boundaries

**Proceed autonomously when:**
- Implementing something explicitly described in the stage spec (build order items, key types, acceptance criteria)
- Writing tests for behavior described in the spec
- Fixing a bug that is clearly a defect (test fails, panic on valid input, wrong output for documented behavior)
- Adding Huginn observation points listed in the stage's Huginn gates section
- Refactoring internal implementation without changing the public API

**Stop and ask when:**
- The stage spec is ambiguous or contradicts another document
- A change would modify a public API established by a prior stage
- A decision has long-term architectural implications not covered in the spec
- Performance is significantly worse than what the spec implies
- You discover a bug in a prior stage that requires non-trivial changes
- You want to add functionality not mentioned in the spec, even if it seems helpful
- Any change to `Cargo.toml` dependencies beyond what the spec requires
- Any change to the Huginn macro interface or buffer structure

---

### 1.5 Code Standards

**Naming conventions are defined in MASTERPLAN Section 6. They are not optional.** Summary:

| Entity | Convention | Example |
|--------|-----------|---------|
| Rust modules | snake_case | `move_gen`, `board_repr` |
| Rust types | PascalCase | `GameState`, `MctsNode` |
| Rust functions | snake_case | `generate_legal_moves` |
| Rust constants | SCREAMING_SNAKE | `MAX_DEPTH`, `TACTICAL_MARGIN` |
| UI components | PascalCase | `BoardDisplay`, `DebugConsole` |
| Protocol commands | lowercase | `bestmove`, `isready` |

**No mixed conventions within a module.** If a module uses `square_index` as a parameter name, all functions in that module use `square_index`, not `sq_idx` in one place and `square` in another and `sqIndex` in a third.

**Term consistency across the entire codebase.** The project has a glossary (MASTERPLAN Section 7). Use those terms. If the glossary says "BRS," the code uses one Rust-appropriate form per context (type name: `BrsSearch`, function: `brs_search`, constant: `BRS_MAX_DEPTH`) and uses it everywhere. Not `best_reply_search` in one module and `brs` in another and `BestReply` in a third.

**Formatting and linting:**
- Run `cargo fmt` before every commit. No exceptions.
- Run `cargo clippy` and address all warnings before every commit. If a specific clippy lint is intentionally suppressed, add a comment explaining why.

**Visibility:**
- Default to private. Expose only what downstream stages need.
- Every `pub` item is a contract that future stages may depend on. Treat it as such.
- Document all `pub` items with a doc comment explaining what it does, not how.

**Constants over magic numbers.** If a number appears in logic code, give it a name:

| Literal | Named Constant |
|---------|---------------|
| `36` | `INVALID_CORNER_COUNT` |
| `196` | `TOTAL_SQUARES` |
| `160` | `VALID_SQUARES` |
| `14` | `BOARD_SIZE` |
| `150` | `TACTICAL_MARGIN` |
| `900` | `QUEEN_EVAL_VALUE` |

These should be defined once in the appropriate module and imported everywhere else.

---

### 1.6 Commit Discipline

- Each commit should correspond to one item in the stage's build order, or one logically atomic change.
- Commit messages must reference the stage: `[Stage 02] Implement pseudo-legal pawn generation for all four directions`
- Never commit code that does not compile. Never commit code with failing tests.
- If a large change breaks things temporarily, use a feature branch and merge when stable.

---

### 1.7 Test Expectations

- Every acceptance criterion in the stage spec must have at least one corresponding test.
- Tests for prior stages must never be deleted or modified to make new code pass. If a prior test is genuinely wrong, document it in the audit log and fix it as a separate commit with clear explanation.
- Unit tests live in the module they test (`#[cfg(test)] mod tests`). Integration tests live in `odin-engine/tests/`.
- Test names describe what they test: `test_checkmate_detected_when_no_legal_moves_and_in_check`, not `test_1` or `test_checkmate`.
- Perft tests are integration tests that run in CI. They are never skipped.
- When a bug is found and fixed, add a regression test that reproduces the bug. Never fix a bug without a test.

---

### 1.8 Decision Principles

When trade-offs arise during implementation, these principles guide the decision. They are ordered by priority.

1. **Correctness before performance,** except where the spec defines explicit performance targets. If the acceptance criteria say "< 10us per position," that IS a correctness requirement.
2. **"Reasonable" means within 10x of the target** given in the MASTERPLAN or downstream logs. If the spec says "depth 6+ within 5 seconds" and you're at 50 seconds, that's a bug. If you're at 8 seconds, that's acceptable for now.
3. **If self-play shows regression, revert and document.** Minimum 100 games at 1-minute time control before declaring a change a regression or improvement. Use SPRT when available (Stage 12+).
4. **Under-engineer rather than over-engineer.** The spec is the scope. Do not build for hypothetical future requirements. Three similar lines of code are better than a premature abstraction.
5. **When the spec is silent, prefer the simpler approach.** If two approaches are equally valid and the spec doesn't specify, pick the one with fewer moving parts.
6. **Record non-obvious decisions in `DECISIONS.md`.** If you chose approach A over approach B and someone might later wonder why, write it down. It takes 2 minutes and saves 20 minutes of re-litigation.

---

### 1.9 Issue Lifecycle

**Issue staleness rule.** At the start of every session, scan [[MOC-Active-Issues]]. Any open **Blocking** or **Warning** issue whose `last_updated` field is older than 3 sessions without an update must be reviewed:
- If still relevant: update `last_updated`, add a status comment explaining current state.
- If no longer relevant: resolve it with a note explaining why, move to Recently Resolved in [[MOC-Active-Issues]].
- If blocked: add a comment explaining what's blocking it and link to the blocking dependency.

**Note**-level issues are exempt from the 3-session staleness check. These are observations logged for future reference (e.g., a limitation that only matters once a later stage is reached). They sit until their relevant stage begins, at which point the agent working that stage reviews them and either promotes them to Warning/Blocking or resolves them.

Agents must never create a Blocking or Warning issue and forget it. Every actionable issue gets touched or resolved.

**Issue creation checklist:**
1. Create the file in `issues/` using the template (`_templates/issue.md`).
2. Fill all fields — especially `severity`, `stage`, and `last_updated`.
3. Add the issue to [[MOC-Active-Issues]] under the correct severity heading.
4. Use existing [[wikilinks]] from the [[Wikilink-Registry]] for all cross-references. Only create a new wikilink target if nothing in the registry covers the concept.
5. If a new wikilink target is needed, add it to [[Wikilink-Registry]] immediately.

**Issue resolution checklist:**
1. Fill in the `## Resolution` section describing what was done.
2. Set `status: resolved` and `date_resolved` in frontmatter.
3. Move the entry from its severity section to `## Recently Resolved` in [[MOC-Active-Issues]].
4. Update `last_updated`.

---

### 1.10 Blocking Issue Resolution

When an audit finds a BLOCKING issue, follow this procedure.

**BLOCKING found during pre-audit (before starting a new stage):**
1. The blocking issue is in a prior stage's code.
2. Fix it before starting the current stage.
3. Record the fix in the prior stage's `audit_log_stage_XX.md` as an addendum.
4. Re-run all prior-stage tests to verify the fix didn't break anything else.
5. Update `STATUS.md` to note the fix.

**BLOCKING found during post-audit (after completing a stage):**
1. The blocking issue is in the current stage's code.
2. Fix it before marking the stage as complete.
3. Do NOT update `STATUS.md` to "complete" until the blocking issue is resolved.
4. Re-run the full post-audit after the fix.

**BLOCKING found in a stage 3+ levels back:**
1. This is serious -- it means multiple stages were built on a broken foundation.
2. Escalate to human oversight. Do not attempt a fix autonomously.
3. Document the impact chain: which stages are affected, what behavior is wrong, what the fix would require.
4. Record in `STATUS.md` as a blocking issue with full context.

**BLOCKING disagreement:**
If the auditor marks something BLOCKING but the implementor disagrees, the BLOCKING stands until a human resolves the disagreement. Err on the side of caution.

---

### 1.11 Version Control

**Branching strategy:** Keep it simple.

- **Main branch** for all stable work. Every commit on main must compile and pass tests.
- **Feature branches** only when a large change breaks things temporarily (per Section 1.6). Name them `stage-XX-feature-name` (e.g., `stage-07-quiescence-search`). Merge back to main when stable.
- **No long-lived branches.** Feature branches should live hours to days, not weeks.

**Tagging and versioning:**
- Tag each completed stage with both a stage tag and a version tag: `stage-00-complete` + `v1.0`, `stage-01-complete` + `v1.1`, etc.
- Tags are created AFTER the post-audit passes (not before).
- Tags are never moved or deleted. They are permanent markers.

**Version scheme:**
- `v1.0` = Stage 0 complete. `v1.1` = Stage 1 complete. `v1.2` = Stage 2 complete. Through `v1.19` = Stage 19 complete.
- **Major version bump** (`v2.0`): only if a rollback forces a rebuild from an earlier stage. Example: a critical flaw is found in Stage 3 while working on Stage 8, requiring a revert to `v1.3` and rebuild. The rebuild starts a new major version: `v2.3` → `v2.4` → `v2.5` → etc.
- Every stage-complete tag is a clean rollback point. `git checkout v1.3` restores the exact project state at the end of Stage 3.
- Record the current version in `STATUS.md` alongside the current stage.

**What gets versioned:**
- All engine code (`odin-engine/`)
- All UI code (`odin-ui/`)
- All training code (`odin-nnue/`)
- All masterplan documents (`masterplan/`)
- `README.md`, `STATUS.md`, `HANDOFF.md`, `DECISIONS.md`
- `.gitignore` (exclude: build artifacts, node_modules, NNUE weight files > 10MB, Huginn trace files)

**What does NOT get versioned:**
- Build artifacts (`target/`, `dist/`, `node_modules/`)
- NNUE weight files (`.onnue`) -- these are large binaries. Store separately or use Git LFS.
- Huginn trace files (`.jsonl`) -- these are debug data, not source.

---

### 1.12 Wikilink Discipline

**The [[Wikilink-Registry]] is the single source of truth for all wikilink targets in the vault.** Before creating any link in any document, check the registry first.

**Rules:**

1. **Reuse before you create.** When linking to a concept, check [[Wikilink-Registry]] for an existing target. If one exists, use it exactly as written (case-sensitive). Do not invent a synonym (`[[stage_02_movegen]]` exists — do not create `[[movegen]]` or `[[MoveGen]]` or `[[Stage-2-Movegen]]`).

2. **New targets require registry updates.** If you genuinely need a new wikilink target (new component note, new issue, new session, new pattern), add it to [[Wikilink-Registry]] immediately after creating the file. Include: target name, file path, category, and one-line purpose.

3. **No orphan links.** Every `[[target]]` in the vault must resolve to an actual file. If you reference something that doesn't exist yet, either create the file or use plain text instead of a wikilink.

4. **No duplicate targets for the same concept.** One concept = one canonical wikilink target. The registry is the arbiter.

5. **Log entries get wikilinks.** When writing audit logs, downstream logs, session notes, or issue notes, link to all relevant stage specs, component notes, decisions, and other notes using registry targets. A log entry that mentions Stage 2's move generation should contain `[[stage_02_movegen]]`, not just the words "Stage 2."

6. **Registry maintenance.** If a file is renamed or deleted, update [[Wikilink-Registry]] to reflect the change. Stale registry entries are as bad as stale issues.

---

### 1.13 Session-End Protocol

Before ending any work session, complete these steps. This takes 5 minutes and saves the next session 30 minutes.

**Step 1: Update `HANDOFF.md`.**
Clear the file and rewrite with:
- What stage and build-order step you're on
- What was completed this session
- What was NOT completed
- Any open issues or discoveries
- Files modified
- What the next session should do first

**Step 2: Update `STATUS.md`.**
- Update "Current Stage" and "Current Build-Order Step"
- Update stage completion tracker if any stages were completed
- Update "What the Next Session Should Do First"
- Add any new blocking issues or regressions

**Step 3: Update `DECISIONS.md`** (if any architectural decisions were made this session).

**Step 4: Commit management file updates.**
```
git add masterplan/STATUS.md masterplan/HANDOFF.md masterplan/DECISIONS.md
git commit -m "[Meta] Session-end status update"
```

This is the last commit of every session. No exceptions.

---

## 2. COMPREHENSIVE AUDIT CHECKLIST

---

### 2.0 Audit Philosophy

An audit is not a rubber stamp. It is adversarial review. The auditor assumes bugs exist and tries to find them.

The checklist below is a minimum, not an exhaustive list. If something looks wrong, flag it even if it does not fit a category.

**Severity levels:**
- **BLOCKING** — Must fix before the next stage begins. The codebase is in a broken, unsafe, or architecturally unsound state.
- **WARNING** — Should fix. Will likely cause problems in a future stage. Can proceed with acknowledgment.
- **NOTE** — Observation for the record. No action required now, but worth tracking.

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
- Functions that have grown beyond 50 lines through repeated additions. Is the function still coherent or has it become a grab bag?
- Structs that have accumulated fields beyond their original purpose. Is `Board` now carrying 20 fields when the spec shows 10?
- Comments that are stale (describe behavior from 3 commits ago).
- Workaround layers: code that patches around a prior workaround. More than one level of workaround is a red flag.
- Test bloat: test helper functions that have been copied and slightly modified rather than generalized.
- "Temporary" code that has survived multiple stages. If something was marked temporary in Stage 2 and it is now Stage 7, it is no longer temporary -- it is technical debt. Either make it permanent (with proper design) or remove it.

---

### 2.3 Code Bloat

More code than necessary for the same result.

**What to look for:**
- Verbose patterns that could be replaced with idiomatic Rust (manual loops vs. iterators, manual error checks vs. `?` operator).
- Duplicate logic that could be shared (two functions that do nearly the same thing for different piece types).
- Over-abstraction: trait hierarchies or generic parameters that serve only one concrete type. If there is only one `impl`, you do not need the trait yet (exception: `Evaluator` and `Searcher` traits are defined before their second implementor, by design).
- Builder patterns, factory functions, or configuration objects for things that could be a simple constructor.
- Excessive logging or debug output in non-Huginn code paths.

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
- Abbreviation inconsistency: `sq` vs. `square` vs. `sqr`. Pick one per context (variable names can abbreviate, type names should not).
- Concept naming drift: `position` vs. `board` vs. `state` used interchangeably when they mean different things in this project. `Board` is the piece layout. `GameState` is the full game. Define `position` once and use it consistently.
- Player naming: `Red`/`Blue`/`Yellow`/`Green` vs. numeric indices (0, 1, 2, 3). The enum should be `Player::Red`, etc. Numeric indices should only appear in array indexing, never in logic.
- Huginn naming: all Huginn-related identifiers should begin with `huginn_` or be in the `huginn::` module. No Huginn code should use names that look like engine code.

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

Comparing the state of the codebase before and after stage work to catch things that logs miss.

**Procedure:**

1. **Before work begins,** record:
   - Full `cargo test` output (test count, all pass)
   - Binary size: `cargo build --release`
   - Binary size with Huginn: `cargo build --release --features huginn`
   - Public API surface of the stage's module (all `pub fn`, `pub struct`, `pub enum`, `pub trait`)

2. **After work completes,** record the same metrics.

3. **Compare:**
   - Test count should have increased (new tests for new functionality). If it decreased, tests were deleted -- investigate why.
   - Binary size with Huginn enabled vs. disabled should differ only by Huginn code. If the gap grew significantly, Huginn code may be leaking into non-gated paths.
   - Public API should contain only what the stage spec requires. If extra public items appeared, justify each one.
   - All pre-existing tests still pass.

4. **Diff review:** Read the full diff between before and after. Look for unintentional changes to files outside the current stage's module.

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
- `.unwrap()` on `Option` or `Result`. Every unwrap is a potential panic. In the engine binary (not tests), replace with `.expect("descriptive context")` at minimum, or proper error handling.
- `.expect()` without meaningful context. `expect("failed")` is nearly as bad as `unwrap()`. Use `expect("TT probe: index {idx} out of bounds for table size {size}")`.
- Array indexing without bounds checks on user-derived indices (square indices from FEN4 parsing, move encoding/decoding).
- Division by zero (visit count = 0 in UCB1 calculation, time remaining = 0 in time management).
- Recursion without depth limits (BRS search, MCTS selection).

**Huginn must NEVER panic.** A Huginn bug should silently drop data, not crash the engine.

---

### 2.13 Test Coverage Gaps

Functionality that exists but has no tests.

**What to look for:**
- Every acceptance criterion in the stage spec must have at least one corresponding test.
- Edge cases specific to 4PC: 3 players simultaneously attacking one king, all 4 players with no legal moves, maximum number of pieces on board, empty board (all captured), terrain blocking every exit.
- Error paths: malformed FEN4 strings, illegal moves via protocol, corrupt Zobrist hash detection.
- Regression tests: when a bug is found and fixed, the test that reproduces the bug must exist. Never fix a bug without a test.
- Integration tests: do stages work together, not just in isolation? After Stage 3, there should be a test that goes Board + MoveGen + GameState and plays a 10-move game.

---

### 2.14 Performance Regressions

Something that was fast becoming slow.

**What to look for:**
- Perft nodes-per-second as a baseline after Stage 2. If NPS drops by more than 10% in any later stage, investigate.
- Eval calls per second as a baseline after Stage 6.
- BRS nodes per second as a baseline after Stage 7.
- MCTS simulations per second as a baseline after Stage 10.
- Allocations in hot paths: `Vec::push` in movegen (should pre-allocate), `Box::new` in MCTS node creation (should use arena after Stage 19), `String` creation in non-Huginn code during search.
- Unnecessary cloning of large structs.
- Hash table operations that degrade as the table fills up.

---

### 2.15 Memory Concerns

Memory leaks, unbounded growth, and allocation patterns.

**What to look for:**
- MCTS tree growth: if the tree is never pruned or reused between searches, memory usage grows without bound across a game. Verify that MCTS tree memory is bounded.
- Position history in `GameState.position_history: Vec<u64>`: grows every move, never shrinks. In long games (200+ moves), this could become large. Consider bounded storage.
- Transposition table: verify it is a fixed-size allocation, not growing.
- Huginn ring buffer: verify it wraps and drops old data silently, does not grow.
- Rust-specific: `Rc` cycles, `Arc` without weak references where cycles are possible, `Box<dyn Trait>` in collections that grow without bound.

---

### 2.16 Feature Flag Contamination

Huginn code leaking into non-Huginn builds.

**What to look for:**
- Any Huginn-related type, function, or import that appears outside of `#[cfg(feature = "huginn")]` blocks.
- The `huginn_observe!` macro compiles to nothing when the flag is off, but if the macro's arguments have side effects (function calls, allocations), those side effects still execute. **Macro arguments must be pure references or copies, never function calls that allocate.**
- Tests that only pass with `--features huginn` enabled. All non-Huginn tests must pass without the flag.
- Binary size comparison: track the difference between `cargo build --release` and `cargo build --release --features huginn` across stages. If it grows unexpectedly, Huginn code is pulling in unnecessary dependencies.
- The `huginn` module should not have `pub` items that non-Huginn code could import. Everything in `huginn::` should be feature-gated.

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
- `use module::*` glob imports. These pull in everything and make it unclear what is actually used.
- External crates pulled in for a single function that could be trivially implemented.
- `Cargo.toml` dependencies that are used in only one stage's code but affect the entire build.
- Feature flags on dependencies that pull in more than needed.

---

### 2.21 Circular Dependencies

Modules that depend on each other, creating coupling.

**What to look for:**
- Module A imports from Module B and Module B imports from Module A. In Rust this is technically possible within a crate but indicates design problems.
- The architecture is layered: Board -> MoveGen -> GameState -> Search -> Eval. Each layer should depend only on layers below it. If eval imports from search, that is a circular dependency.
- Exception: Huginn can observe any module because it is a cross-cutting concern. But Huginn must never be a dependency OF any module -- no engine code imports from Huginn.

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
- Resource exhaustion: TT should replace entries not grow, MCTS should stop expanding not crash.

---

### 2.24 API Surface Area Creep

Public interfaces growing beyond what is necessary.

**What to look for:**
- Count the `pub` items in each module after each stage. If a stage that should add 5 public functions added 15, investigate.
- Helper functions that were made `pub` for testing convenience but are not part of the module's contract. Use `pub(crate)` instead.
- Internal types that leaked to `pub` because a `pub fn` returns them.
- Principle: each module should have a narrow, well-defined public API. Everything else is `pub(crate)` at most.

---

### 2.25 Documentation/Code Drift

Documentation that describes behavior the code no longer exhibits.

**What to look for:**
- Doc comments on functions that were correct when written but are stale after refactoring.
- MASTERPLAN references to code structures that have changed.
- Downstream log entries that describe API contracts that were subsequently modified.
- Inline comments explaining "why" that reference conditions that no longer exist.
- Huginn gate descriptions in the MASTERPLAN that do not match the actual observation points in code.

---

### 2.26 Semantic Correctness

Code that compiles and passes existing tests but is logically wrong in ways not yet tested.

**What to look for:**
- **Evaluation symmetry:** The same position evaluated from Red's perspective and from Yellow's perspective (with colors swapped) should produce symmetric results. Asymmetry indicates a perspective bug.
- **Move generation completeness:** Verify not just that generated moves are legal, but that ALL legal moves are generated. Perft catches aggregate errors; individual position tests also needed.
- **Score consistency:** If BRS evaluates a move at +300cp and MCTS evaluates the same move as terrible, either BRS or MCTS has a bug (or `eval_scalar` and `eval_4vec` are inconsistent).
- **Game rule edge cases from `4PC_RULES_REFERENCE.md`:** Check confirmed only at the affected player's turn, DKW king captures worth 0 points, promoted queen worth 1 point on capture but 900cp in eval. Each of these is a potential mismatch between rules reference and implementation.

---

## 3. HUGINN REPORTING SPECIFICATION

The MASTERPLAN (Section 2.1) defines WHAT Huginn observes -- the gates listed per stage. This section defines HOW Huginn reports, stores, organizes, and exposes that data. These are complementary. An agent adding Huginn gates reads the MASTERPLAN for what to observe and this section for how to format and store the observation.

---

### 3.1 Report Format: Structured JSON Lines (JSONL)

Each observation is a single JSON object on one line. One observation = one line. No multi-line JSON.

**Schema for every observation:**
```json
{
  "ts": 1423847291,
  "session_id": "a1b2c3d4",
  "trace_id": "e5f6g7h8",
  "gate": "alpha_beta_prune",
  "stage": 7,
  "phase": "brs",
  "level": "verbose",
  "data": { }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `ts` | u64 | Monotonic clock nanoseconds from engine start. Not wall clock. Allows correlation without timezone issues. |
| `session_id` | string | Generated once per engine process. All traces in one session share this. |
| `trace_id` | string | Generated per search invocation (from `go` to `bestmove`). All observations within one search share this. **Primary correlation key.** |
| `gate` | string | Observation point name, matching the MASTERPLAN's Huginn gate names exactly (e.g., `"board_mutation"`, `"zobrist_update"`, `"alpha_beta_prune"`). |
| `stage` | u8 | The stage number that defined this gate. For provenance. |
| `phase` | string | `"brs"`, `"mcts"`, `"eval"`, `"movegen"`, `"setup"`, or `"summary"`. Which engine phase emitted this. |
| `level` | string | `"minimal"`, `"normal"`, `"verbose"`, or `"everything"`. The verbosity level that emitted this. |
| `data` | object | Gate-specific payload. Defined per gate (see Section 3.7). |

---

### 3.2 Storage: Ring Buffer + Optional File Sink

**Primary storage: In-memory ring buffer.** The `HuginnBuffer` from Stage 0.
- Fixed-size, pre-allocated at engine startup.
- Default capacity: 65,536 entries (2^16).
- Each entry is a fixed-size slot (max 4 KB per serialized observation).
- When full, oldest entries are silently overwritten. No warning, no error, no panic.
- **No allocation during search.** The buffer is pre-allocated. Observations are written into existing slots.
- Configurable via `setoption name HuginnBufferSize value <N>`.

**Secondary storage: Optional file sink.**
- Enabled via `setoption name HuginnFile value <path>`.
- Observations are flushed from the ring buffer to a JSONL file during post-search processing (after `bestmove` is returned).
- Appends to the file. One line per observation.
- The file can be analyzed with standard tools: `grep`, `jq`, Python `json` module.

**Default: in-memory only.** File sink is opt-in. For development, the in-memory buffer is sufficient for the most recent search. For deep analysis or batch auditing, enable file sink.

---

### 3.3 Organization: 5-Level Trace Hierarchy

This is how Huginn data stays navigable instead of becoming a wall of noise.

```
Level 1: Session         — one engine process lifetime
  Level 2: Search/Trace  — one `go` to `bestmove` cycle (identified by trace_id)
    Level 3: Phase       — BRS or MCTS within a search
      Level 4: Path      — move sequence from root (BRS) or selection path (MCTS)
        Level 5: Gate    — individual observation point
```

**Querying pattern:** Filter by `trace_id` first (get everything from one search), then filter by `phase`, `gate`, or `level`. The JSONL format makes `grep` and `jq` usable for ad-hoc queries:

```bash
# All observations from search trace "e5f6g7h8"
grep '"trace_id":"e5f6g7h8"' huginn.jsonl

# Only alpha-beta prune events from that search
grep '"trace_id":"e5f6g7h8"' huginn.jsonl | grep '"gate":"alpha_beta_prune"'

# Only anomalies (minimal-level events are summaries and anomalies)
grep '"level":"minimal"' huginn.jsonl
```

**Per-search summary:** After each search completes, Huginn automatically generates a summary observation:

```json
{
  "gate": "search_summary",
  "level": "minimal",
  "data": {
    "best_move": "d2d4",
    "score_cp": 150,
    "depth": 8,
    "total_nodes": 523847,
    "brs_time_ms": 234,
    "mcts_time_ms": 1766,
    "mcts_sims": 4823,
    "surviving_moves": 5,
    "observations_by_gate": { "alpha_beta_prune": 12847, "eval_call": 48293, ... },
    "observations_by_level": { "minimal": 1, "normal": 34, "verbose": 891 },
    "anomalies": []
  }
}
```

This summary is always emitted at `minimal` level. It is the headline for the search.

---

### 3.4 Verbosity Level Contract

What goes in each level. The goal: Minimal shows the answer, Normal shows the reasoning, Verbose shows every step, Everything shows every detail.

---

#### Minimal (target: fewer than 10 observations per search)

- **Search summary** (one per search): best move, score, depth, node count, phase times, surviving move count.
- **Anomalies only:** Zobrist mismatch detected, hash collision detected, score exceeds sane bounds, assertion-like failures.
- Gate firing: only gates that detected a problem.

Use Minimal for production monitoring and quick sanity checks.

---

#### Normal (target: 20-50 observations per search)

Everything in Minimal, plus:

- **Board context** (Stage 8): the full `BoardContext` output at search start.
- **Phase transition:** BRS result with surviving moves and their scores, handoff to MCTS.
- **Iterative deepening progression:** depth, best move, score, node count, time for each completed depth.
- **MCTS root summary:** visit distribution for top 5 moves.
- **Eval call summary:** count, average, min, max for this search.

Use Normal for routine development auditing. Run 5-10 test positions at Normal, check the summaries.

---

#### Verbose (target: 200-2,000 observations per search)

Everything in Normal, plus:

- Individual alpha-beta cutoffs with depth, alpha/beta values, cutoff move.
- Move ordering sequence at root node.
- Killer and history heuristic state.
- Each MCTS simulation's selection path and leaf evaluation.
- Cheap filter classification at opponent nodes (interacting vs. background).
- Hybrid reply scoring for each scored move.
- Progressive narrowing: which moves were cut at each depth.
- Accumulator update details (features added/removed per move).

Use Verbose when investigating a specific position where the engine plays poorly.

---

#### Everything (target: 10,000+ observations per search)

Everything in Verbose, plus:

- Every eval call with full component breakdown.
- Every TT probe (hit/miss/store) at every node.
- Every move generation call with full move list.
- SEE computation per capture.
- Quantization comparison (float vs. quantized) at every NNUE layer.

Use Everything only for debugging specific issues. Will fill the ring buffer quickly. Always enable file sink when using this level.

---

### 3.5 How Agents Use Huginn Data During Audits

**Routine audit (every stage):**
1. Run the engine with `--features huginn` at Normal verbosity on 5-10 test positions.
2. For each search, check the search summary for anomalies (the `anomalies` array in the summary should be empty).
3. Verify the phase transition: did BRS produce a reasonable number of surviving moves? Did MCTS agree with BRS on the best move, or disagree?
4. Check eval summary: are min/max evals in reasonable ranges (not near overflow)?
5. If anything looks off, re-run at Verbose on that specific position to get the detailed trace.

**Debugging a specific bug:**
1. Enable Everything verbosity.
2. Enable file sink (`setoption name HuginnFile value /tmp/huginn_debug.jsonl`).
3. Run the failing position.
4. Filter the trace by the relevant gate. Example: Zobrist mismatch detected -> filter by `"zobrist_update"` gate -> find the exact operation where the hash diverged.

**Verifying a new Huginn gate:**
1. Run a known position at the gate's verbosity level.
2. Verify the gate fires (its observations appear in the trace).
3. Verify the payload contains the expected fields (see Section 3.7).
4. Verify the data values match manual calculation.

**Cross-search comparison:**
For positions where the engine plays poorly, collect traces at Normal and compare against positions where it plays well. Look for structural differences: different surviving move counts, BRS-MCTS disagreement, anomalous eval values, different board context readings.

---

### 3.6 Trace Correlation: Following a Decision Through the Pipeline

Within a trace, moves are encoded in the same notation format (`d2d4`, `e1g1`). Filtering by a specific move string across all gates shows every point where that move was considered, scored, pruned, or selected.

**Example: trace a move that survives to selection**

| Step | Gate | What you see |
|------|------|-------------|
| 1 | `move_generation` | Move appears in legal move list |
| 2 | `move_ordering` | Move's ordering score and position in sequence |
| 3 | `alpha_beta_prune` (absence) | Move was NOT pruned (not in prune gate = survived) |
| 4 | `brs_surviving` | Move appears with its BRS score |
| 5 | `phase_transition` | Move passes threshold into MCTS |
| 6 | `mcts_expansion` | Move expanded as child of root |
| 7 | `mcts_simulation` (multiple) | Move appears in selection paths |
| 8 | `mcts_root_summary` | Move's visit count and value |
| 9 | `search_summary` | Move selected as bestmove |

**Example: trace a move that was pruned**

| Step | Gate | What you see |
|------|------|-------------|
| 1 | `move_generation` | Move appears in legal move list |
| 2 | `alpha_beta_prune` | Move pruned at depth X with alpha/beta values |
| 3 | (nothing) | Move does not appear in any subsequent gate |

**Procedure:** Filter the JSONL by the move string:
```bash
grep '"d2d4"' huginn.jsonl | grep '"trace_id":"<target_trace>"'
```
This gives the complete lifecycle of that move through the pipeline.

---

### 3.7 Gate Payload Schemas

For each gate defined in the MASTERPLAN, the `data` object must contain these fields. This section is populated incrementally as stages are built. Representative examples:

---

**Stage 1 gates:**

`board_mutation`:
```json
{
  "square": 42,
  "previous": { "type": "Pawn", "owner": "Red" },
  "new": null,
  "hash_before": "0x1a2b3c4d5e6f7890",
  "hash_after": "0x9f8e7d6c5b4a3210"
}
```

`zobrist_update`:
```json
{
  "operation": "xor_out",
  "key_index": 1247,
  "key_value": "0xabcdef0123456789",
  "hash_before": "0x1a2b3c4d5e6f7890",
  "hash_after": "0x9f8e7d6c5b4a3210"
}
```

`piece_list_sync`:
```json
{
  "player": "Red",
  "array_count": 16,
  "list_count": 16,
  "match": true
}
```

---

**Stage 2 gates:**

`move_generation`:
```json
{
  "position_hash": "0x1a2b3c4d5e6f7890",
  "player": "Red",
  "pseudo_legal_count": 42,
  "legal_count": 35,
  "moves": ["d2d3", "d2d4", "e2e3", ...]
}
```

`perft`:
```json
{
  "depth": 3,
  "expected_nodes": 12345,
  "actual_nodes": 12345,
  "match": true
}
```

---

**Stage 7 gates:**

`alpha_beta_prune`:
```json
{
  "depth": 5,
  "alpha": -150,
  "beta": 200,
  "score": 210,
  "move": "e4e5",
  "node_type": "min",
  "cutoff": true
}
```

`iterative_deepening`:
```json
{
  "depth": 6,
  "best_move": "d2d4",
  "score_cp": 150,
  "nodes": 234567,
  "time_ms": 423,
  "pv": ["d2d4", "c7c5", "e2e4"]
}
```

---

**Stage 8 gates:**

`board_context`:
```json
{
  "weakest_player": "Green",
  "root_danger_level": 0.72,
  "per_opponent": [
    { "player": "Blue", "aggression_toward_root": 0.85, "best_target": "Red" },
    { "player": "Yellow", "aggression_toward_root": 0.23, "best_target": "Green" },
    { "player": "Green", "aggression_toward_root": 0.41, "best_target": "Yellow" }
  ]
}
```

`reply_scoring`:
```json
{
  "opponent": "Blue",
  "move": "f6f5",
  "objective_strength": 120,
  "harm_to_root": 0.73,
  "likelihood": 0.82,
  "final_score": 0.69
}
```

---

**Stage 10 gates:**

`mcts_simulation`:
```json
{
  "sim_number": 847,
  "selection_path": ["d2d4", "c7c5", "e2e4", "f7f5"],
  "leaf_eval": [0.62, 0.15, 0.08, 0.15],
  "leaf_depth": 4
}
```

---

**Stage 11 gates:**

`phase_transition`:
```json
{
  "brs_time_ms": 234,
  "surviving_moves": [
    { "move": "d2d4", "score_cp": 150 },
    { "move": "e2e4", "score_cp": 130 },
    { "move": "c2c4", "score_cp": 95 }
  ],
  "eliminated_count": 27,
  "threshold_cp": 0,
  "mcts_budget_ms": 1766
}
```

---

New gates are defined following this pattern as each stage is implemented. The gate name, stage number, and payload schema must be documented here when added.

---

### 3.8 Huginn Anti-Patterns

Things Huginn must never do. Violations of these are BLOCKING audit findings.

1. **Never format strings during search.** Observations are raw data (integers, enum variants as u8, hash values as u64). JSON serialization happens during post-search processing only. During search, the `huginn_observe!` macro copies raw values into the ring buffer slot.

2. **Never allocate during search.** The ring buffer is pre-allocated. If an observation exceeds the slot size, truncate it. Do not allocate a larger buffer.

3. **Never branch on Huginn data.** The engine must never read from the Huginn buffer. Data flows one way: engine -> Huginn. Never Huginn -> engine. No `if huginn_buffer.last_score > 200 { ... }`.

4. **Never introduce conditional compilation beyond the feature gate.** Do not add `#[cfg(debug_assertions)]` or other conditional compilation to Huginn code. Either `cfg(feature = "huginn")` is on and all Huginn code is active, or it is off and none of it exists.

5. **Never panic.** If Huginn encounters an error (buffer full, serialization failure, malformed input), it silently drops the observation and continues. A Huginn bug must never crash the engine.

6. **Never cause side effects from macro arguments.** `huginn_observe!(board.expensive_clone())` is wrong -- the clone executes even when the macro body is empty. Arguments must be pure references: `huginn_observe!(&board, square, hash)`.

---

## 4. WHAT HUGINN CANNOT CATCH

Huginn sees data. It does not understand design. These are the categories of problems that require human or agent judgment and cannot be detected by automated observation alone. When auditing, actively look for these -- passing all Huginn gates and tests does NOT mean the code is correct.

---

### 4.1 Architectural Drift

The codebase slowly deviates from the intended architecture in the MASTERPLAN.

**Examples:**
- The UI starts computing legal moves locally instead of asking the engine (violates "UI owns ZERO game logic").
- Eval starts importing from the search module, creating a circular dependency.
- GameState accumulates non-rule concerns (tracking search statistics, storing eval caches).

**How to catch it:** At every stage boundary, compare the actual module dependency graph against the MASTERPLAN architecture diagram (Section 2). Does the real code still match the intended layers?

---

### 4.2 Wrong Abstractions

An abstraction makes some things easy but makes the right things hard.

**Examples:**
- Abstracting all four players into a generic "opponent iterator" that makes it hard to ask "which specific opponent is attacking me?" (critical for hybrid scoring in Stage 8).
- Using a trait for evaluation that prevents inlining of NNUE inference (performance-critical path).
- Over-generalizing move generation to handle hypothetical piece types that will never exist.

**How to catch it:** When implementing a downstream stage feels like fighting the abstractions from an upstream stage, the abstraction may be wrong. Document this in the downstream log.

---

### 4.3 Over-Engineering

Building for hypothetical future needs that are not in the spec.

**Examples:**
- Adding a plugin system for custom piece types when the game only has 7 types.
- Building a generic N-player search framework when the game is always 4 players.
- Implementing SIMD during Stage 7 when Stage 19 is designated for optimization.
- Adding network play infrastructure when the spec says local only.

**How to catch it:** For every abstraction or generalization, ask: "Does the MASTERPLAN require this?" If no, do not build it.

---

### 4.4 Under-Engineering

Taking shortcuts that will cost significantly more to fix later than to do correctly now.

**Examples:**
- Using `Vec<Piece>` for piece lists when a fixed-size array is specified and critical for `GameState` clone performance.
- Using `String` for move representation in hot paths when a compact `u32` encoding is specified.
- Not implementing Zobrist hashing from the start, planning to "add it later."

**How to catch it:** Cross-reference the stage spec's key types and design notes. If the implementation diverges from the specified types, flag it.

---

### 4.5 Algorithmic Correctness

Huginn can see that the BRS search explored 50,000 nodes and returned move d2d4 with score +150. It cannot see whether the alpha-beta algorithm is correctly implemented or whether the hybrid scoring formula is mathematically sound.

**Examples:**
- BRS negamax with the sign convention wrong for one player (correct for Red and Yellow, wrong for Blue and Green).
- MCTS UCB1 using `ln` instead of `log2` (both "work" but with different exploration rates).
- Hybrid reply scoring formula giving negative likelihood values.
- Progressive widening exponent too aggressive (misses good moves) or too conservative (tree too wide).

**How to catch it:** Manual algorithm review. Compare the implementation against the MASTERPLAN specification line by line. Write test cases with hand-computed expected values.

---

### 4.6 Performance Pathology

Correct but slow. The algorithm produces the right answer but takes 100x longer than necessary.

**Examples:**
- Move generation that recomputes attack tables from scratch every call instead of using pre-computed tables.
- MCTS that clones `GameState` for every simulation instead of using make/unmake.
- NNUE that does full recomputation on every move instead of incremental updates.
- Board context scanner that checks every piece against every square (O(n^2)) instead of targeting relevant areas.

**How to catch it:** Profiling with `cargo flamegraph` or `criterion` benchmarks. Performance baselines in downstream logs. Huginn can sometimes reveal these (e.g., `accumulator_update` gate showing full recompute when incremental was expected) but cannot detect all of them.

---

### 4.7 Design Intent Violations

Code that works and is correct but does not serve the project's goals.

**Examples:**
- An eval function that is very accurate but takes 100us (the target is < 10us for bootstrap, < 2us for NNUE).
- BRS that finds the best move in trivial positions but times out in complex positions because progressive narrowing is too conservative.
- MCTS that converges but takes 100,000 simulations when the time budget only allows 5,000.

**How to catch it:** Test against the acceptance criteria's quantitative targets, not just correctness. "Works" is not enough -- it must work within the specified constraints.

---

### 4.8 Readability and Maintainability

Code that is correct and fast but that the next agent cannot understand.

**Examples:**
- 200-line functions with no internal comments.
- Variable names like `x`, `tmp`, `val2` in algorithmic code.
- Control flow with 5+ levels of nesting.
- Excessive use of closures, iterator chains, or trait objects where simpler imperative code would be clearer.

**How to catch it:** The "read it cold" test. If an agent reading this code for the first time, with only the MASTERPLAN and this conduct document, cannot understand what the code does within 60 seconds per function, the code is not readable enough.

---

## 5. TIER-SPECIFIC CONDUCT NOTES

These are behavioral notes organized by tier (not per-stage, to keep it manageable). They supplement, not replace, the per-stage specs in the MASTERPLAN.

---

### 5.1 Tier 1: Foundation (Stages 0-5)

Everything downstream depends on these stages being rock solid.

- **Favor correctness over speed, clarity over cleverness.** The MASTERPLAN explicitly says "correctness is the only goal" for Stage 2.
- **Establish invariants with exhaustive tests.** Board validity, move legality, Zobrist correctness, game rule compliance. Every invariant must have tests that will catch regressions forever.
- **Do not optimize.** If perft is slow, that is fine. Correct first.
- The **attack query API** (`is_square_attacked_by`, `attackers_of`) from Stage 2 is reused by Stages 3, 7, 8, and castling. It must be correct, well-tested, and have a stable API.
- **GameState** (Stage 3) must be cheaply cloneable. Use fixed-size arrays over `Vec` where the spec indicates. This is a design note in the MASTERPLAN.
- The **Odin Protocol** (Stage 4) is the contract between engine and UI. Changes affect both sides. Treat it as a stable interface.
- The **UI** (Stage 5) owns ZERO game logic. Resist the temptation to "just add" a quick legality check client-side.

---

### 5.2 Tier 2: Simple Search (Stages 6-7)

Getting the engine to play chess for the first time.

- The `Evaluator` trait (Stage 6) and `Searcher` trait (Stage 7) are **interface contracts that persist through the entire project**. `eval_scalar` + `eval_4vec` will be called by BRS, MCTS, and the hybrid controller. The signatures must not change. Bootstrap eval quality matters less than interface stability.
- **Keep the bootstrap eval simple.** The spec says "this is temporary." Do not spend time perfecting piece-square tables. NNUE replaces it.
- **Plain BRS must be fully working before Stage 8.** The hybrid layer in Stage 8 is layered on top. If BRS is broken, the hybrid amplifies the brokenness.

---

### 5.3 Tier 3: Strengthen Search (Stages 8-11)

These are the most complex stages. The BRS hybrid alone has 8 build-order items.

- **Build order is sequential and testable.** Complete step 1 and verify it works before starting step 2. The MASTERPLAN explicitly says "If the hybrid scoring makes things worse at any step, roll back to plain BRS."
- **Stage 11** (Hybrid Integration) must not change BRS or MCTS internals. It is a controller layer that orchestrates them through the `Searcher` trait.
- Track the **TACTICAL_MARGIN** constant (150cp default). It controls how many moves survive BRS. Measure its effect empirically.
- **MCTS** (Stage 10) implements the same `Searcher` trait as BRS. It must work standalone before integration.

---

### 5.4 Tier 4: Measurement (Stages 12-13)

You can now measure whether changes help or hurt.

- **Self-play** (Stage 12) is infrastructure, not a feature. Every subsequent stage uses it for validation.
- **Time management** (Stage 13) uses self-play for parameter tuning. The tuning process should be documented: which parameter, what range tested, what the self-play result showed.
- From this point forward, significant changes should be validated by self-play win rate and/or SPRT.

---

### 5.5 Tier 5: Learn (Stages 14-16)

Neural network integration.

- **Stage 14 is architecture and inference only.** No training code. No trained weights. Random weights to verify the pipeline.
- **Stage 15** involves Python (PyTorch). Python code must follow PEP 8, use type hints, and have clear separation from the Rust codebase. The critical interface is the `.onnue` binary format that bridges Python and Rust.
- **Stage 16 is the most dangerous swap in the project.** The before-and-after audit (Section 2.10) is mandatory. Record everything: perft, NPS, eval values for benchmark positions, self-play results. The `Evaluator` trait makes the swap mechanically clean, but accumulator lifecycle bugs are subtle.

---

### 5.6 Tier 6: Polish (Stages 17-19)

Integration, refinement, and hardening.

- **Variant tuning** (Stage 17) modifies search/eval behavior per game mode. Changes must be isolated behind the `GameMode` enum, not scattered through search code with `if terrain_mode { ... }` everywhere.
- **Full UI** (Stage 18) still owns ZERO game logic. Move arrows, highlights, and self-play dashboards are driven by engine output, not local computation.
- **Optimization** (Stage 19) is profile-first. Never optimize without measurement. Run profiler, identify top 3 bottlenecks, address those, re-profile. The regression test suite (from Stage 12) validates every optimization doesn't break correctness.

---

## 6. AUDIT LOG AND DOWNSTREAM LOG PROCEDURES

The existing templates (`audit_log_stage_XX.md` and `downstream_log_stage_XX.md`) are skeletons. This section explains how to fill them in.

---

### 6.1 How to Fill the Audit Log

**Pre-Audit (completed BEFORE any code is written):**

| Section | What to write |
|---------|--------------|
| Build State | Does `cargo build`, `cargo test`, and `cargo build --features huginn` pass? Yes/No for each. If no, describe the failure. |
| Previous downstream flags reviewed | For each upstream dependency, list the stage number and any Must-Know, Known Limitations, or Open Questions that affect this stage. Quote the specific text. |
| Findings | Anything discovered during the review that might affect this stage's work. |
| Risks for This Stage | What could go wrong? What are the tricky parts? Reference specific audit checklist items (Section 2) that are most relevant. |

**Post-Audit (completed AFTER all code is written and tests pass):**

| Section | What to write |
|---------|--------------|
| Deliverables Check | For each item in the stage's "What you're building" list, state whether it is complete and how it was verified. |
| Code Quality: Uniformity | Are naming conventions consistent? Are patterns used consistently? Reference Section 2.8. |
| Code Quality: Bloat | Is there unnecessary code? Over-abstraction? Reference Section 2.3. |
| Code Quality: Efficiency | Are there obvious performance problems? Allocations in hot paths? Reference Section 2.14. |
| Code Quality: Dead Code | Are there unused functions, imports, or match arms? Reference Section 2.5. |
| Code Quality: Broken Code | Did you find logic errors, off-by-one bugs, or edge case failures? Reference Section 2.6. |
| Code Quality: Temporary Code | Is there code marked as temporary or TODO? What is the plan for it? Reference Section 2.2. |
| Search/Eval Integrity | (From Stage 6 onward) Do evaluations produce sane values? Does search find known best moves? Reference Section 2.26. Write "N/A" for stages before eval/search exist. |
| Future Conflict Analysis | Reference the dependency map (MASTERPLAN Appendix A). Which future stages depend on this one? What could go wrong for them? Flag specific concerns. |
| Unaccounted Concerns | Anything that doesn't fit other sections. Things that feel wrong but you can't prove. Gut instincts about fragile code. |
| Reasoning & Methods | HOW was the audit conducted? Which tools were used, which positions were tested, what manual reviews were done, what Huginn verbosity was used? This lets future agents reproduce the audit. |
| Issue Resolution | All **Blocking** issues for this stage must be resolved. All **Warning** issues must be either resolved or explicitly acknowledged with a documented reason to defer (including which stage will address it). **Note**-level issues may remain open. Reference [[MOC-Active-Issues]] and list the disposition of each open issue. |

**Specific observations are required.** Do not write "looks fine" or "no issues." Instead write: "Checked all 12 public functions for naming consistency per Section 2.8 -- all follow snake_case convention. Verified no mixed abbreviations (all use `square` not `sq`)." Even negative findings should be specific about what was checked.

---

### 6.2 How to Fill the Downstream Log

| Section | What to write |
|---------|--------------|
| Must-Know | Things that a future stage agent absolutely needs to understand. Not "everything about this stage" -- only things that would cause problems if missed. Example: "Pawn promotion creates `PieceType::PromotedQueen`, not `PieceType::Queen`. These are distinct in the piece list and Zobrist hash." |
| API Contracts | Every public function, struct, or type that downstream stages will use. Include: signature, semantics, constraints, and any non-obvious behavior. Example: "`eval_scalar(position, player) -> i16`: Returns centipawn evaluation from given player's perspective. Range: -30000 to +30000. Values outside this range indicate mate scores." |
| Known Limitations | What this stage does NOT do that someone might expect. Example: "Move generation does not pre-sort or order moves. Ordering is Stage 9's responsibility." |
| Performance Baselines | Timing and throughput numbers that future stages must not regress below. Example: "Perft(4) from starting position: 1,234,567 nodes in 2.3 seconds (537K NPS). Measured on [hardware description]." |
| Open Questions | Unresolved design questions that may affect future stages. Example: "Zobrist keys use u64. If collision rate is too high with 4 players on 160 squares, u128 may be needed (MASTERPLAN Appendix B: Risk Register mentions this)." |
| Reasoning | Why decisions were made. Not what was done (that is in the code), but why this approach was chosen over alternatives. |

---

## 7. APPENDICES

---

### Appendix A: Quick-Reference Card

One-page summary for fast agent onboarding.

**Before starting any stage (Section 1.1):**
0. Orient: read STATUS.md, HANDOFF.md, DECISIONS.md
1. Read stage spec in MASTERPLAN
2. Read upstream audit logs (trace dependency chain)
3. Read upstream downstream logs
4. `cargo build && cargo test && cargo build --features huginn` -- all must pass
5. Fill pre-audit section
6. Begin work

**Top 10 things to check in every audit:**
1. All prior-stage tests still pass (Section 1.2)
2. No cascading breakage from changed APIs (Section 2.1)
3. No dead code added (Section 2.5)
4. No naming inconsistencies introduced (Section 2.8)
5. No magic numbers (Section 2.22)
6. No `.unwrap()` in engine code (Section 2.12)
7. Huginn gates compile to nothing when flag is off (Section 2.16)
8. Zobrist make/unmake round-trip still works (Section 2.18)
9. Public API surface is minimal (Section 2.24)
10. Before/after metrics recorded (Section 2.10)

**Decision priority order (Section 1.8):**
1. Correctness > Performance > Elegance
2. Spec-defined > Agent-inferred > Unspecified
3. Measured improvement > Theoretical improvement
4. Simple + working > Clever + fragile
5. Existing patterns > Novel patterns (unless spec requires novelty)
6. Reversible > Irreversible (when uncertain)

**When blocked (Section 1.10):**
- Pre-audit blocking finding → fix before writing code, or escalate
- Post-audit blocking finding → fix before tagging stage complete
- Deep regression (prior stage) → document, branch, isolate fix, re-run full suite

**Huginn verbosity summary:**
- Minimal: headline only (< 10 observations)
- Normal: the reasoning (20-50 observations)
- Verbose: every step (200-2,000 observations)
- Everything: firehose (10,000+ observations)

**When to stop and ask (Section 1.4):** Spec ambiguity, changing prior-stage APIs, architectural decisions not in spec, adding unspecified functionality.

**Naming (from MASTERPLAN Section 6):** modules = snake_case, types = PascalCase, functions = snake_case, constants = SCREAMING_SNAKE.

**Before ending any session (Section 1.13):**
1. Update HANDOFF.md (what was done, what wasn't, what's next)
2. Update STATUS.md (stage progress, blocking issues)
3. Update DECISIONS.md (if any decisions were made)
4. Commit: `[Meta] Session-end status update`

---

### Appendix B: Huginn Gate Registry

Master table of all Huginn gates. Populated incrementally as stages are built. Start with the gates defined in the MASTERPLAN, add payload schemas from Section 3.7 as they are implemented.

| Gate Name | Stage | Default Level | Key Payload Fields | Purpose |
|-----------|-------|---------------|-------------------|---------|
| `board_mutation` | 1 | verbose | square, previous, new, hash_before/after | Track every piece placement/removal |
| `zobrist_update` | 1 | everything | operation, key_index, key_value, hash_before/after | Trace hash corruption to exact XOR |
| `fen4_roundtrip` | 1 | normal | input, parsed_hash, serialized, match | Catch parse/serialize mismatches |
| `piece_list_sync` | 1 | verbose | player, array_count, list_count, match | Catch array/list desync |
| `move_generation` | 2 | normal | position_hash, player, pseudo/legal counts, moves | Track movegen output |
| `make_unmake` | 2 | verbose | move, hash_before/after, captured, flags, hash_restored | Verify state restoration |
| `legality_filter` | 2 | verbose | rejected_move, reason, attacker | Why each move was rejected |
| `perft` | 2 | normal | depth, expected, actual, match | Verify move generation counts |
| `turn_transition` | 3 | normal | previous, next, skipped, reason | Track turn rotation |
| `check_detection` | 3 | verbose | king, attackers | Which king tested, what found |
| `checkmate_stalemate` | 3 | normal | determination, position | Check/stalemate rulings |
| `elimination` | 3 | normal | reason, points, terrain_conversions, dkw | Elimination events |
| `scoring` | 3 | normal | who, how_many, action, totals | Score changes |
| `dkw_move` | 3 | normal | king_pos, move, legal_set | DKW random moves |
| `game_over` | 3 | minimal | condition, scores, winner | Game termination |
| `command_receive` | 4 | normal | raw, parsed_type, errors | Protocol input |
| `response_send` | 4 | normal | full_string, trigger | Protocol output |
| `position_set` | 4 | normal | fen4/startpos, move_list, resulting_hash | Position setup tracking |
| `search_request` | 4 | normal | time_controls, depth_limits, options | Search go command parameters |
| `eval_call` | 6 | verbose | hash, player, score, components | Evaluation breakdown |
| `eval_comparison` | 6 | normal | hash, scores_per_player | 4-perspective comparison |
| `alpha_beta_prune` | 7 | verbose | depth, alpha, beta, score, move, node_type | Search cutoffs |
| `quiescence` | 7 | verbose | entry/exit, stand_pat, captures | Quiescence search |
| `iterative_deepening` | 7 | normal | depth, best_move, score, nodes, time, pv | ID progression |
| `brs_reply_selection` | 7 | verbose | opponent, candidates, selected, scores | Reply choice |
| `board_context` | 8 | normal | full BoardContext | Pre-search board read |
| `board_context_delta` | 8 | verbose | changes, updated, fallback | Delta refresh tracking |
| `cheap_filter` | 8 | verbose | passed, background, classifications | Move classification |
| `reply_scoring` | 8 | verbose | opponent, move, strength, harm, likelihood, score | Hybrid scoring |
| `progressive_narrowing` | 8 | verbose | depth, max_allowed, considered, truncated | Candidate narrowing |
| `tt_lookup` | 9 | everything | hash, hit/miss, stored_data | TT probes |
| `tt_store` | 9 | everything | stored, replaced, reason | TT writes |
| `move_ordering` | 9 | verbose | sequence, scores | Move order produced |
| `killer_see` | 9 | verbose | killer_moves, see_values, captures_reordered | Killer move and SEE tracking |
| `mcts_simulation` | 10 | verbose | sim_number, path, leaf_eval, depth | MCTS simulations |
| `mcts_selection` | 10 | everything | children_ucb1, selected, perspective | UCB1 selection |
| `mcts_expansion` | 10 | verbose | position, move, prior, widening_check | Node expansion |
| `mcts_root_summary` | 10 | normal | visit_distribution, selected, temperature | Root statistics |
| `phase_transition` | 11 | normal | brs_time, survivors, eliminated, threshold, mcts_budget | BRS->MCTS handoff |
| `surviving_comparison` | 11 | normal | brs_ranking, mcts_ranking | Agreement check |
| `time_allocation` | 11 | normal | tactical/quiet, planned_split, actual | Time budgeting |
| `search_controller` | 11 | normal | full lifecycle go->bestmove | Complete search trace |
| `regression_test` | 12 | normal | expected, actual, pass/fail | Regression checks |
| `self_play_anomaly` | 12 | minimal | bad_move, trace_context | Obvious blunders |
| `time_budget` | 13 | normal | remaining, complexity, budget, split | Time management |
| `time_overrun` | 13 | minimal | budget_exceeded, context, abort | Overrun detection |
| `panic_time` | 13 | minimal | trigger_condition, adjusted_behavior | Panic time activation |
| `accumulator_update` | 14 | verbose | features_added/removed, perspective, type | NNUE accumulator |
| `nnue_forward` | 14 | verbose | accumulator, per_layer, scalar, vector | NNUE inference |
| `quantization` | 14 | verbose | float_values, quantized_values, layer_boundary | Float vs. quantized comparison |
| `weight_load` | 14 | normal | architecture_hash, feature_set_id, param_count, checksum | NNUE weight file loading |
| `data_generation` | 15 | normal | position_hash, brs_target, mcts_target, game_result | Training data extraction |
| `training_sample_validation` | 15 | normal | position, brs_vs_mcts_disagreement, flagged | Training data QC |
| `eval_swap` | 16 | normal | nnue_vs_bootstrap, accumulator_state | Integration monitoring |
| `accumulator_lifecycle` | 16 | verbose | push/pop_through_search, state_leak_check | Accumulator state through search tree |
| `nnue_vs_bootstrap` | 16 | normal | position, nnue_score, bootstrap_score, disagreement_cp | Side-by-side eval comparison (temporary) |
| `dkw_random_move` | 17 | normal | king_pos, move_selected, interference_with_active | DKW move in variant context |
| `terrain_conversion` | 17 | normal | pieces_converted, positions, movegen_diff | Terrain mode piece conversion |
| `chess960_setup` | 17 | normal | arrangement, bishop_colors, king_between_rooks | Chess960 position validation |
| `scoring_anomaly` | 17 | minimal | score_change, expected_per_table, actual, flagged | Score changes not matching point table |
| `search_summary` | 0+ | minimal | best_move, score, depth, nodes, times, anomalies | Per-search headline |

This table grows with each stage. When a new gate is added, record it here with its default verbosity level and key fields.

---

### Appendix C: Common Odin-Specific Pitfalls

Bugs that are especially likely in a four-player chess engine on a non-standard board. Agents should actively check for these during audits.

| Pitfall | What Goes Wrong | Where It Matters |
|---------|----------------|-----------------|
| **Pawn direction reversal** | Red +rank, Blue +file, Yellow -rank, Green -file. A sign error means pawns move backward. | Stage 2 (movegen), Stage 6 (piece-square tables) |
| **Corner square validity** | 36 specific squares are invalid. Not a simple rectangle -- four 3x3 corners. | Stage 1 (board), Stage 2 (movegen, ray generation) |
| **Three-way check** | Must check attacks from 3 opponents, not 1. | Stage 2 (legal filtering), Stage 3 (check detection) |
| **Promoted queen dual value** | Worth 1 point on capture (scoring) but 900cp in eval (search). Two different systems. | Stage 3 (scoring), Stage 6 (eval) |
| **DKW timing** | Dead king moves happen instantly between turns, not as a full turn. | Stage 3 (DKW handler) |
| **Castling for 4 players** | 8 rights (2 per player), each player has their own back rank. Not the same as 2-player castling. | Stage 2 (castling), Stage 1 (Zobrist) |
| **En passant with 4 players** | Must clear on the very next move, even if that move is another player's turn. | Stage 2 (make/unmake), Stage 1 (Zobrist) |
| **Terrain piece inertness** | Terrain pieces block movement, cannot be captured, produce no check. They are walls, not pieces. | Stage 3 (terrain), Stage 2 (movegen near terrain) |
| **Turn skipping** | Eliminated players are skipped in turn rotation. Must not crash or infinite-loop. | Stage 3 (turn management) |
| **eval_scalar / eval_4vec consistency** | Same position, same perspective, results must be compatible. If scalar says +200 for Red, 4vec should show Red as relatively high. | Stage 6 (eval), Stage 16 (NNUE swap) |
| **Zobrist side-to-move** | 4 players, not 2. The hash key depends on which of 4 players moves next, not a binary toggle. | Stage 1 (Zobrist), Stage 2 (make/unmake) |
| **Stalemate scoring** | In FFA, stalemate awards 20 points. Not a draw. Not zero. | Stage 3 (stalemate handler), Stage 6 (eval) |

---

*End of Agent Conduct v1.0*
