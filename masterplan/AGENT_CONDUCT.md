# AGENT CONDUCT — AI/Agent Development Rules for Project Odin

**Version:** 1.0
**Created:** 2026-02-19
**Status:** Active

---

## 0. PREAMBLE

This document defines HOW AI agents behave while building Project Odin. It is one of three core reference documents:

| Document | Defines | Authority Over |
|----------|---------|---------------|
| `MASTERPLAN.md` ([[MASTERPLAN]]) | WHAT each stage builds | Stage specs, acceptance criteria, architecture, tracing points |
| `4PC_RULES_REFERENCE.md` ([[4PC_RULES_REFERENCE]]) | The game rules | Board layout, piece movement, scoring, game modes |
| `AGENT_CONDUCT.md` (this) | HOW agents work | Behavior rules, audit procedures, code standards |

**Every agent that touches the codebase must read this document before beginning any work.**

This document does not duplicate the masterplan. It references it. If this document says "see MASTERPLAN Section 4" it means go read that section there, not that the content is copied here.

---

## 1. AGENT BEHAVIOR RULES

---

### 1.1 Stage Entry Protocol

Before writing a single line of code for any stage, follow these steps in order. Skipping steps causes cascading problems that compound across stages.

**Step 0: Orient yourself.** Read `STATUS.md` ([[STATUS]]) to know where the project is. Read `HANDOFF.md` ([[HANDOFF]]) to know what the previous session was doing. Read `DECISIONS.md` ([[DECISIONS]]) if you're new to the project or working on a stage where architectural decisions were made. Read `SYSTEM_PROFILE.local.md` to understand the hardware and software constraints of the current development machine. This takes 5 minutes and prevents you from duplicating work, re-arguing settled decisions, or making assumptions about available resources.

> **SYSTEM_PROFILE.local.md** is a gitignored, machine-specific file in the `masterplan/` directory. It contains CPU, RAM, GPU, and storage specs along with their implications for build times, memory budgets, parallelism, and feature feasibility. If this file does not exist, create it by asking the user for their system specs. Never commit it to version control.

**Step 1: Read the stage specification** in `MASTERPLAN.md` ([[MASTERPLAN]]) Section 4. Understand:
- What this stage builds (deliverables)
- Build order (sequential steps)
- Acceptance criteria (definition of done)
- Tracing points (observation points to add)
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

**Step 4: Research the work ahead.** Before writing any code, conduct online research on the techniques, algorithms, and concepts relevant to this stage. Understand what problems others have encountered and what solutions exist.

- Search broadly. There is virtually no published research on four-player chess from an engine/coding standpoint, so direct references will be scarce. When chess-specific information comes up lacking, look to adjacent fields: multi-agent game theory, multi-player game tree search, RTS/strategy game AI, swarm intelligence, influence mapping, or whatever domain is closest to the task at hand.
- Identify potential pitfalls, edge cases, and performance traps before they become bugs.
- Note any algorithms, papers, or implementations that could inform the design. Record useful findings in the pre-audit section of the audit log under a "Research Notes" heading.
- This is not optional. Every stage involves concepts that benefit from prior art review. Even well-known techniques (alpha-beta, MCTS, NNUE) have four-player-specific gotchas that only surface through broad reading.

**Step 5: Build and test what exists.** Run:
```
cargo build
cargo test
```
If anything fails, STOP. Do not proceed with new work on a broken foundation. Record the failure in the pre-audit section of this stage's audit log.

**Step 6: Complete the pre-audit** section of `audit_log_stage_XX.md`. Record:
- Build state (compiles? tests pass?)
- Findings from upstream logs
- Research notes (key findings from Step 4)
- Risks identified for this stage

**Step 7: Begin implementation.**

---

### 1.2 Search Depth Policy

**Only depths divisible by 4 are valid search depths.** In four-player chess, each player takes one ply per round. A depth that is not a multiple of 4 creates evaluation bias — the last-moving player gets an artificial advantage because opponents don't get to respond.

- **Depth 4:** Minimum acceptable search depth. One full round of play.
- **Depth 8:** Maximum practical depth given current hardware constraints (see `SYSTEM_PROFILE.local.md`).
- **Depth 12+:** Not feasible on current hardware. Do not target.
- **Depth 1, 2, 3, 5, 6, 7:** Never use as a search depth in production code, tests, self-play, benchmarks, or documentation. The only exception is internal iterative deepening loops where intermediate depths are stepping stones to a depth-4 or depth-8 target — but these intermediate results must never be treated as final.

This applies everywhere: engine defaults, test assertions, self-play configurations, observer baselines, training data generation, and documentation examples.

---

### 1.3 The First Law: Do Not Break What Exists

Every commit must leave the project in a compilable, test-passing state. No exceptions. No "I'll fix it in the next commit."

**Permanent invariants** (once established, these must pass after every stage, forever):

The authoritative invariant table is in `MASTERPLAN.md` ([[MASTERPLAN]]) Section 4.1 (14 invariants, Stages 0-9). Consult that table for the full list. Key invariants for quick reference:

- **Stage 0:** Prior-stage tests never deleted.
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
- Adding tracing instrumentation at key boundaries
- Refactoring internal implementation without changing the public API

**Stop and ask when:**
- The stage spec is ambiguous or contradicts another document
- A change would modify a public API established by a prior stage
- A decision has long-term architectural implications not covered in the spec
- Performance is significantly worse than what the spec implies
- You discover a bug in a prior stage that requires non-trivial changes
- You want to add functionality not mentioned in the spec, even if it seems helpful
- Any change to `Cargo.toml` dependencies beyond what the spec requires

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
1. Fill in the `## Resolution` section describing what the fix is and what files were changed.
2. Set `status: pending-verification` in frontmatter. Do NOT set `resolved` yet.
3. Update `last_updated`.

**Verification gate.** An agent must NEVER claim a bug is fixed or mark an issue as `resolved` until the user has verified the fix through self-play or manual testing. Passing `npm test` or a clean compile is necessary but not sufficient — runtime behavior must be confirmed by a human.

- After implementing a fix, tell the user what was changed and ask them to verify.
- Only after the user confirms the fix works: set `status: resolved`, set `date_resolved`, and move the entry from its severity section to `## Recently Resolved` in [[MOC-Active-Issues]].
- If the user reports the fix doesn't work: update the `## Resolution` section with what was tried, set `status: open`, and continue investigating.

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
- `.gitignore` (exclude: build artifacts, node_modules, NNUE weight files > 10MB)

**What does NOT get versioned:**
- Build artifacts (`target/`, `dist/`, `node_modules/`)
- NNUE weight files (`.onnue`) -- these are large binaries. Store separately or use Git LFS.

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

### 1.13 Vault Note Protocol

**Agents must create Obsidian vault notes to surface knowledge that would otherwise be buried inside audit logs, downstream logs, and session notes.** The graph only works if concepts exist as their own nodes.

**Mandatory vault notes — create these as you work, not retroactively:**

| Trigger | Note type | Folder | Example |
|---|---|---|---|
| Every WARNING or BLOCKING audit finding | Issue | `issues/` | `Issue-EP-Representation-4PC.md` |
| Every component you implement or substantially modify | Component | `components/` | `Component-Board.md` |
| Every cross-component interaction you discover or build | Connection | `connections/` | `Connection-Board-to-MoveGen.md` |
| Every non-obvious pattern or trick worth reusing | Pattern | `patterns/` | `Pattern-Pawn-Reverse-Lookup.md` |

**NOTE-level audit findings** do not require issue notes unless they affect future stages. Use judgment.

**Resolved issues stay as notes.** When an issue is resolved, update its `status` to `resolved` and fill the Resolution section. Move it from the active section to Recently Resolved in [[MOC-Active-Issues]]. Do not delete the file — the graph link history is valuable.

**Every vault note must:**
1. Use the template from `_templates/` for its type
2. Link back to the relevant stage spec, audit log, and downstream log using existing [[wikilinks]] from [[Wikilink-Registry]]
3. Be added to [[Wikilink-Registry]] immediately after creation
4. Be added to the relevant MOC ([[MOC-Active-Issues]] for issues, [[MOC-Sessions]] for sessions)

**Tags in vault notes** use the frontmatter `tags:` field for broad categories only — not per-concept. Good: `stage/02`, `severity/warning`, `area/movegen`. Bad: `#en-passant-bug`, `#pawn-reverse-lookup`, `#zobrist-mismatch`. Tags group; wikilinks connect. If you need to find a concept, link to its note — don't create a tag.

---

### 1.14 Session-End Protocol

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

### 1.15 Debugging Discipline

When investigating a bug, agents must follow a structured process that prevents analysis paralysis while still allowing deep exploration. The goal: every analysis pass must either **narrow the hypothesis space** or **produce empirical evidence**. If it does neither, stop analyzing and start testing.

---

#### The Core Problem This Solves

An agent can spiral when it re-derives the same conclusion from different angles without gaining new information. This burns context window and produces no code. The spiral typically looks like: identify bug → second-guess identification → re-trace same logic → arrive at same answer → wonder about edge cases → re-examine same functions → repeat until context is exhausted.

Productive analysis looks different: identify bug → ask "why wasn't it caught earlier?" → discover a new code path → find a specific ordering issue → identify root cause. Each pass introduces something **new and citable**.

---

#### Rule 1: Maintain a Hypothesis Journal

When investigating a non-trivial bug (more than a one-line fix), track your reasoning explicitly. After each analysis pass, state:

1. **Current hypothesis:** What you think the bug is. Must cite specific code locations (`file.rs:line`).
2. **What's new:** The specific code, value, path, or contradiction you discovered in this pass that you did not know before. Must be citable — a line number, a variable name, an execution path.
3. **What's eliminated:** Which prior hypotheses this pass ruled out, and why.

**The spiral test:** If your hypothesis is identical to the previous pass and "what's new" is empty, you are spiraling. Stop analyzing and move to Rule 3.

**Rephrasing is not progress.** "The bug is in handle_go because it returns early" and "The issue is that handle_go has an early return path" are the same hypothesis. Do not count restatement as refinement.

---

#### Rule 2: Trust Hierarchy

When the user provides a diagnosis, implement their suggested fix FIRST. Test it. Only investigate alternatives if the fix fails or introduces new problems.

The trust order for bug diagnosis:

1. **User's explicit diagnosis** — Implement it. If you disagree, implement it anyway and let the test results settle the dispute. Cost of trying: minutes. Cost of ignoring and spiraling: the entire context window.
2. **Direct code reading** — What the code actually does, traced through with specific values.
3. **Speculation about what "might" happen** — This is the weakest form of evidence. Never spend more than one analysis pass on a "might" without converting it to empirical evidence (Rule 3).

**Corollary:** If the user says "X works correctly, the problem is Y" — do not spend analysis passes questioning whether X works correctly. Address Y first. If Y's fix reveals that X is actually broken, you'll discover that empirically.

---

#### Rule 3: Empirical Escalation

**After two consecutive analysis passes where the hypothesis did not narrow (no new code location discovered, no hypothesis eliminated), you must write a test.** Not "plan to write a test." Write it. Run it. Let the output guide the next pass.

The test does not need to be perfect. A minimal reproduction is sufficient:

```rust
#[test]
fn repro_checkmate_skipping_bug() {
    // Set up the position where the bug occurs
    let mut gs = GameState::from_fen4("...");
    // Attempt the operation that fails
    let result = gs.legal_moves();
    // Assert what you expect vs. what happens
    assert!(result.is_empty(), "Red should have no legal moves");
}
```

This test accomplishes two things:
- It grounds the analysis in concrete behavior, not speculation.
- It persists as a regression test after the fix (per Section 1.7).

**Exception:** If you cannot write a test because you don't understand the setup well enough, that itself is the signal — read more code to understand the setup, don't re-analyze the same function.

---

#### Rule 4: One Bug, One Focus

When investigating Bug A and you discover a potential Bug B, **write Bug B down and continue working on Bug A.** Do not chase Bug B mid-investigation.

How to write it down:
- If trivial: a code comment `// TODO: potential issue — DKW ordering may affect elimination detection`
- If non-trivial: create an issue note in `issues/` per Section 1.9

The temptation to chase Bug B is strong because it feels like progress. It is not. It is scope expansion that fragments attention and burns context.

**The only exception:** Bug B actively blocks the fix for Bug A (you literally cannot test Bug A's fix without fixing Bug B first). In that case, fix Bug B minimally, then return to Bug A.

---

#### Rule 5: Scope Lock After Diagnosis

Once you have a diagnosis with a specific code location and a concrete fix plan, **stop analyzing and start implementing.** Do not spend additional passes exploring what might go wrong with the fix, what edge cases the fix might miss, or what other bugs might exist nearby.

Implement → Test → Observe. If edge cases exist, the tests will reveal them. If the fix is incomplete, the failing test will tell you exactly what's still wrong. This is cheaper and more reliable than pre-analyzing every possible outcome.

**Anti-pattern:** "My fix handles case X, but what about case Y? And what if Z happens? Let me trace through the code one more time to make sure..." — This is the re-analysis spiral wearing a different mask. Implement the fix for X. Write a test for Y. Run it. If it fails, fix Y as a follow-up.

---

#### Recognizing the Spiral — Concrete Signals

You are spiraling if ANY of these are true:

| Signal | Example |
|---|---|
| **Re-reading a function you already analyzed** without a specific new question to answer | Reading `handle_go()` a third time "just to make sure" |
| **Questioning evidence you already accepted** without new contradictory evidence | "But does check_elimination_chain really work?" after already confirming it does |
| **Exploring hypothetical scenarios** without converting them to tests | "What might happen if the UI sends the same position twice?" — write a test instead |
| **Expanding scope** beyond the reported bug | "While I'm here, let me also check whether stalemate scoring works..." |
| **Restating your conclusion in different words** | "So the issue is really that..." for the third time |
| **Tracing execution paths you already traced** with the same starting conditions | Re-simulating the same move sequence through the same code path |

You are NOT spiraling if:

| Signal | Example |
|---|---|
| **Each pass cites a new code location** | "I found that `process_dkw_moves` at line 280 runs AFTER `check_elimination_chain` at line 260" — this is new, specific, and narrows the hypothesis |
| **Each pass eliminates a hypothesis** | "This rules out the UI sync theory because the UI doesn't call go until after position is sent" |
| **You discovered a contradiction** that changes the analysis | "The user said check_elimination_chain works, but I see it calls generate_legal which might return different results depending on DKW state" — this is a specific, citable new finding |
| **You're reading a NEW function** you haven't examined yet | Moving from `handle_go` to `process_dkw_moves` for the first time |

---

#### Summary: The Debugging Flowchart

```
Bug reported
    │
    ├─ User provided diagnosis? ──YES──→ Implement it. Test it. Done (or iterate).
    │
    NO
    │
    ▼
Read relevant code. Form hypothesis (cite file:line).
    │
    ▼
┌─ Analysis pass ──────────────────────────────────────────┐
│  Ask: "What is NEW in this pass?"                        │
│  • New code location discovered? → Record, continue.     │
│  • Hypothesis eliminated? → Record, continue.            │
│  • Nothing new? → STOP ANALYZING. Write a test. (Rule 3) │
└──────────────────────────────────────────────────────────┘
    │
    ▼
Hypothesis confirmed or narrowed?
    │
    ├─ YES → Implement fix. Test. Ship.
    │
    NO (hypothesis unchanged after test)
    │
    ▼
Escalate: write down what you know, what you tried,
and what didn't work. Ask the user.
```

---

### 1.16 Deferred-Debt Escalation Rule

**The problem this solves:** A system or feature can be deferred at Stage N with the note "will wire in Stage N+1." Then Stage N+1 defers to N+2. Then N+2 defers to N+3. After 8 stages of silent deferral, the project carries dead code that was never functional, and the user discovers it only when they try to use it. This happened with Huginn (see ADR-015).

**The rule:** If any work item, feature, integration, or plumbing task has been deferred for **2 or more consecutive stages**, it becomes a **mandatory escalation item**. The agent must:

1. **Flag it loudly in HANDOFF.md** under a dedicated `## Deferred Debt` section. Each item must include:
   - What it is
   - How many stages it has been deferred
   - WHY it is stuck (the actual blocker, not just "not needed yet")
   - What would unblock it (specific technical approach)
   - Whether the design itself might be flawed

2. **Promote the issue severity.** If the deferred item has a vault issue note, promote it from NOTE to WARNING after 2 stages of deferral. After 3 stages, promote to BLOCKING or explicitly record a decision (in DECISIONS.md) that the feature is being intentionally abandoned.

3. **Tell the user directly.** Do not silently carry deferred debt. If something has been pushed back 2+ times, the user must hear about it in plain language: "This has been deferred for N stages. Here's why it's stuck and what we should do about it."

**What counts as deferral:**
- "Will wire in the next stage"
- "Deferred per established pattern"
- "Not needed yet, will add when relevant"
- Any variant of "will do later" applied to the same item across multiple stages

**What does NOT count:**
- Intentional sequencing per the build order (e.g., NNUE training waiting for self-play infrastructure is not deferral — it's dependency ordering)
- Items whose prerequisite stage hasn't been reached yet

**The spirit of the rule:** If you're turning something off to make the problem go away temporarily, that is not a fix. Raise your voice and speak up. Silent deferral compounds into silent failure.

---

### 1.17 Task Tracking Protocol

**The problem this solves:** Agents jump into code changes without proving they understand the problem. Sessions produce code but leave no reasoning trail. When a fix causes a regression 3 sessions later, there's no record of *why* the original change was made or what alternatives were considered. The next agent re-derives everything from scratch.

**The solution:** Every non-trivial unit of work gets a **task file** in `masterplan/tasks/` that records understanding, investigation, plan, execution, and follow-up. The file name signals status at a glance.

---

#### File Naming

- **In progress:** `Task-Short-Name_in_progress.md`
- **Complete:** `Task-Short-Name_complete.md`

When a task finishes, rename the file (change `_in_progress` to `_complete`). Update [[MOC-Tasks]].

---

#### Task Lifecycle

**Step 1: Create the task file** from `_templates/task.md`. Name it `Task-Short-Name_in_progress.md`. Add it to [[MOC-Tasks]] under "In Progress."

**Step 2: Fill Section 1 (Understanding Check) BEFORE writing any code.** This is mandatory. The agent must:
- List every file read and what was learned from each
- State the problem in their own words
- List all constraints and invariants that must not be broken
- Note any prior attempts and what happened

**The understanding gate:** If the agent cannot fill Section 1 with specific file paths, specific constraints, and a clear problem statement, they are not ready to write code. Read more first.

**Step 3: Fill Section 2 (Investigation).** Record root cause analysis with confidence levels (HIGH/MEDIUM/LOW) and concrete evidence (code references, game logs, eval traces). This is the reasoning trail that future agents will read.

**Step 4: Fill Section 3 (Plan).** List specific changes with file, function, and rationale. Note risks and alternatives considered.

**Step 5: Execute and fill Section 4** as you work. Record actual changes, test results, and verification.

**Step 6: Fill Section 5 (References)** with wikilinks to all related sessions, issues, components, and decisions.

**Step 7: Fill Section 6 (Follow-Up)** if the task revealed new problems. Create issues/tasks for them.

**Step 8: Rename the file** from `_in_progress` to `_complete`. Update [[MOC-Tasks]] (move from "In Progress" to "Completed" with a reference note pointing to related sessions and issues).

---

#### When to Use Task Files

| Situation | Task File? |
|---|---|
| Multi-file code change with investigation | YES |
| Bug fix requiring root cause analysis | YES |
| Eval tuning or search parameter change | YES |
| Performance retrofit (Vec clone cost) | YES |
| One-line typo fix | NO |
| Adding a single test | NO |
| Documentation-only update | NO |

**Rule of thumb:** If the work involves investigation or could cause a regression, it gets a task file.

---

#### Why This Matters

1. **Proof of understanding** prevents agents from coding before they understand the problem
2. **Investigation logs** preserve reasoning that would otherwise be lost when context resets
3. **Plan + alternatives** prevent re-litigating the same design choices in future sessions
4. **The `_in_progress` / `_complete` rename** makes the backlog scannable at a glance
5. **Reference links** let any future agent trace from a task back to the full context
6. **Follow-up section** catches downstream issues early instead of discovering them 3 sessions later

---

### 1.18 Diagnostic Gameplay Observer Protocol

**The problem this solves:** Behavioral bugs (pawn-push preference, king walks, hanging piece blindness) only surface during live gameplay. Without automated observation, the only way to catch them is for a human to manually watch games. Agents need a structured way to run games, capture logs, and analyze engine thinking — without interfering with other agents or the user's workflow.

**The solution:** The engine has a built-in protocol logging toggle. The UI has a Max Rounds auto-stop. Together, an agent can run a diagnostic game, capture all protocol traffic, and analyze it afterward.

---

#### Who Runs Diagnostics

**ONLY the top-level orchestrating agent** may start the engine, build the project, or run diagnostic games. Subagents (Explore, Plan, Bash workers, etc.) MUST NOT independently:
- Start or stop the engine process
- Run `cargo build` or `cargo build --release`
- Spawn the engine binary
- Modify engine state while another agent is working

If a subagent needs diagnostic data, the top-level agent runs the game and passes the resulting log file path to the subagent for analysis.

**Before building or starting the engine:** The top-level agent must confirm no other agent (including Claude.T or any parallel session) is actively compiling or running the engine. Building while another agent is mid-compilation corrupts outputs.

---

#### Engine Protocol Logging

The engine supports a `LogFile` option via the standard `setoption` command:

```
setoption name LogFile value observer/logs/diagnostic_2026-02-27.log
```

- **Enable:** `setoption name LogFile value <path>` — opens a buffered file writer. Creates parent directories automatically.
- **Disable:** `setoption name LogFile value none` (or `off`, or empty) — flushes and closes the log file.
- **Format:** Incoming commands logged as `> ...`, engine responses as `< ...`.
- **Overhead:** Zero when disabled (single `if let` check). Negligible when enabled (buffered I/O, flushed on each response).

Log files persist in `observer/logs/` for historical comparison across sessions.

---

#### Max Rounds Auto-Stop

The UI provides a **Max Rounds** slider (0–50, where 0 = unlimited). When set:
- After all players complete the specified number of rounds (1 round = 4 ply), auto-play pauses automatically.
- The game state is preserved — the user or agent can resume, adjust settings, or start a new game.

For diagnostic use, set Max Rounds to the desired observation window (e.g., 10–20 rounds) before starting Full Auto.

---

#### Diagnostic Workflow

When the top-level agent needs to diagnose engine behavior:

1. **Build** (if needed): `cargo build --release` — confirm no other agent is compiling first.
2. **Start engine + UI**: Via Tauri dev or standalone.
3. **Configure settings**: Game Mode, Eval Profile, Terrain, Depth — as needed for the test.
4. **Enable logging**: Send `setoption name LogFile value observer/logs/<descriptive_name>.log` via the Communication Log input in the UI, or programmatically if using the engine directly.
5. **Set Max Rounds**: Use the UI slider to set the observation window.
6. **Set Full Auto**: Start the game in Full Auto mode.
7. **Wait for auto-stop**: The game pauses when the round limit is reached.
8. **Disable logging**: Send `setoption name LogFile value none`.
9. **Analyze**: Read the log file. Look for patterns: eval swings, suspicious moves, repetitive play, hanging piece blindness, pawn-push preference, king displacement.
10. **Record findings**: Create an issue in `masterplan/issues/` if a new behavioral bug is found, or update an existing issue with new evidence.

---

#### Log File Naming Convention

Use descriptive names that encode the test parameters:

```
observer/logs/ffa_standard_d6_20rounds_2026-02-27.log
observer/logs/lks_aggressive_d4_10rounds_2026-02-27.log
observer/logs/post-stage10-mcts-baseline_2026-03-01.log
```

---

#### When to Run Diagnostics

| Situation | Run diagnostic? |
|---|---|
| After eval tuning or search changes | YES — verify behavior improved |
| After completing a stage | YES — baseline before next stage |
| Investigating a reported behavioral bug | YES — reproduce and capture evidence |
| Routine code cleanup or refactor | NO — run tests instead |

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
- Abbreviation inconsistency: `sq` vs. `square` vs. `sqr`. Pick one per context (variable names can abbreviate, type names should not).
- Concept naming drift: `position` vs. `board` vs. `state` used interchangeably when they mean different things in this project. `Board` is the piece layout. `GameState` is the full game. Define `position` once and use it consistently.
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

Comparing the state of the codebase before and after stage work to catch things that logs miss.

**Procedure:**

1. **Before work begins,** record:
   - Full `cargo test` output (test count, all pass)
   - Binary size: `cargo build --release`
   - Public API surface of the stage's module (all `pub fn`, `pub struct`, `pub enum`, `pub trait`)

2. **After work completes,** record the same metrics.

3. **Compare:**
   - Test count should have increased (new tests for new functionality). If it decreased, tests were deleted -- investigate why.
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
- Allocations in hot paths: `Vec::push` in movegen (should pre-allocate), `Box::new` in MCTS node creation (should use arena after Stage 19), `String` creation during search.
- Unnecessary cloning of large structs.
- Hash table operations that degrade as the table fills up.

---

### 2.15 Memory Concerns

Memory leaks, unbounded growth, and allocation patterns.

**What to look for:**
- MCTS tree growth: if the tree is never pruned or reused between searches, memory usage grows without bound across a game. Verify that MCTS tree memory is bounded.
- Position history in `GameState.position_history: Vec<u64>`: grows every move, never shrinks. In long games (200+ moves), this could become large. Consider bounded storage.
- Transposition table: verify it is a fixed-size allocation, not growing.
- Rust-specific: `Rc` cycles, `Arc` without weak references where cycles are possible, `Box<dyn Trait>` in collections that grow without bound.

---

### 2.16 Feature Flag Contamination

Feature-gated code leaking into default builds.

**What to look for:**
- Any feature-gated type, function, or import that appears outside its `#[cfg(feature = "...")]` blocks.
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
- Exception: Tracing instrumentation can observe any module because it is a cross-cutting concern. But tracing configuration must never be a dependency OF any module -- no engine logic imports from tracing configuration.

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

The engine uses the `tracing` crate for structured logging and diagnostics. This replaced a custom compile-gated telemetry system (Huginn) that was retired in Stage 8 (see ADR-015).

### 3.1 Tracing Usage

- Use `tracing::debug!` for search/eval diagnostic output
- Use `tracing::info!` for high-level events (search start/complete, position set)
- Use `tracing::trace!` for verbose per-node data (only in development)
- All tracing calls are zero-cost when filtered out at runtime

### 3.2 Environment Configuration

```
RUST_LOG=odin_engine=debug    # Development
RUST_LOG=odin_engine=info     # Normal operation
RUST_LOG=odin_engine=trace    # Verbose debugging
```

---

## 4. WHAT AUTOMATED TRACING CANNOT CATCH

Tracing can record data. It does not understand design. These are the categories of problems that require human or agent judgment and cannot be detected by automated observation alone. When auditing, actively look for these -- passing all tracing checks and tests does NOT mean the code is correct.

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

Automated tracing can record that the BRS search explored 50,000 nodes and returned move d2d4 with score +150. It cannot see whether the alpha-beta algorithm is correctly implemented or whether the hybrid scoring formula is mathematically sound.

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

**How to catch it:** Profiling with `cargo flamegraph` or `criterion` benchmarks. Performance baselines in downstream logs. Tracing can sometimes reveal these (e.g., accumulator updates showing full recompute when incremental was expected) but cannot detect all of them.

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
| Build State | Does `cargo build` and `cargo test` pass? Yes/No for each. If no, describe the failure. |
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
| Reasoning & Methods | HOW was the audit conducted? Which tools were used, which positions were tested, what manual reviews were done, what tracing level was used? This lets future agents reproduce the audit. |
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

**When to retroactively update a downstream log vs. letting the next pre-audit catch it:**

Most content drift (stale test counts, outdated limitation descriptions, changed baselines) is handled by the next stage's pre-audit — that's what the pre-audit "findings from upstream logs" section is for. Do NOT routinely patch prior-stage logs after bugfixes or refactors.

**Retroactive updates ARE required when a bugfix or change introduces a new API contract or behavioral invariant that a future agent would need to build correctly.** The test: if a future agent reading the downstream log would make a *wrong decision* without this information (not just note a stale number), add it now.

Examples of retroactive-required changes:
- New public method added to a module (`handle_no_legal_moves()` on GameState)
- New ordering invariant (`process_dkw_moves` must run before `check_elimination_chain`)
- New protocol message format that parsers must handle
- Changed function signature that downstream callers must respect

Examples that the next pre-audit handles:
- Test counts changed (302 → 504)
- Known limitation is now outdated (turn tracking was simple, now it's complex)
- Performance baselines shifted
- Session notes with slightly wrong test counts (historical records — never patch)

---

## 7. APPENDICES

---

### Appendix A: Quick-Reference Card

One-page summary for fast agent onboarding.

**Before starting any stage (Section 1.1):**
0. Orient: read STATUS.md, HANDOFF.md, DECISIONS.md, SYSTEM_PROFILE.local.md
1. Read stage spec in MASTERPLAN
2. Read upstream audit logs (trace dependency chain)
3. Read upstream downstream logs
4. Research the work ahead (online, broad, adjacent fields)
5. `cargo build && cargo test` -- all must pass
6. Fill pre-audit section (include research notes)
7. Begin work

**Top 10 things to check in every audit:**
1. All prior-stage tests still pass (Section 1.2)
2. No cascading breakage from changed APIs (Section 2.1)
3. No dead code added (Section 2.5)
4. No naming inconsistencies introduced (Section 2.8)
5. No magic numbers (Section 2.22)
6. No `.unwrap()` in engine code (Section 2.12)
7. No feature flag contamination (Section 2.16)
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

**Tracing levels (Section 3):**
- `info` -- high-level events (search start/complete, position set)
- `debug` -- search/eval diagnostics
- `trace` -- verbose per-node data (development only)

**When to stop and ask (Section 1.4):** Spec ambiguity, changing prior-stage APIs, architectural decisions not in spec, adding unspecified functionality.

**When debugging (Section 1.15):**
1. User gave diagnosis? → Implement it first. Test. Investigate only if it fails.
2. Each analysis pass must cite something NEW (file:line, variable, path). No new citation = spiral.
3. Two passes without narrowing → write a test. Not "plan to." Write it.
4. One bug at a time. Discovering Bug B? Write it down. Keep fixing Bug A.
5. Have a fix plan? Stop analyzing. Implement → Test → Observe.

**Naming (from MASTERPLAN Section 6):** modules = snake_case, types = PascalCase, functions = snake_case, constants = SCREAMING_SNAKE.

**Before ending any session (Section 1.14):**
1. Update HANDOFF.md (what was done, what wasn't, what's next)
2. Update STATUS.md (stage progress, blocking issues)
3. Update DECISIONS.md (if any decisions were made)
4. Commit: `[Meta] Session-end status update`

---

### Appendix B: Observability Notes

The custom Huginn telemetry system (compile-gated ring buffer with JSONL output) was retired in Stage 8 and replaced with the `tracing` crate. See Section 3 for current tracing configuration. The original Huginn gate registry is preserved in git history for reference (pre-Stage 8 commits).

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
