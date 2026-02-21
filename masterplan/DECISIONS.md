# PROJECT ODIN — ARCHITECTURAL DECISION LOG

**Version:** 1.0
**Created:** 2026-02-19

---

## Purpose

This document records architectural decisions and their reasoning so future agents don't re-litigate settled questions. If you're about to argue for an approach that contradicts a decision here, read the reasoning first. If new information changes the calculus, record a new decision that supersedes the old one -- don't silently override it.

**Format:** Each decision has a number, date, the decision itself, what alternatives were considered, why this was chosen, and which stages are affected.

---

## Decisions

---

### ADR-001: Board Representation — Array-First with Clean Abstraction

**Date:** 2026-02-18
**Status:** Active
**Affects:** Stage 1 ([[stage_01_board]]), Stage 2 ([[stage_02_movegen]]), Stage 19 ([[stage_19_polish]])

**Decision:** The board uses a flat 196-element array (`[Option<Piece>; 196]`). Nothing above Stage 2 reads `board.squares[]` directly. All board queries go through the attack query API (`is_square_attacked_by`, `attackers_of`) defined in Stage 2.

**Alternatives considered:**
- **Bitboards from the start:** 160-square bitboards require 3x u64 or a custom u192 type. Non-trivial implementation with high bug potential for unproven performance gains.
- **Hybrid array + bitboards:** Additional complexity, two representations to keep in sync.

**Why this was chosen:** "Do it right the first time" means making the boundary swappable, not picking the fastest representation upfront. The attack query API is THE abstraction boundary. If profiling later shows array scanning is a bottleneck, bitboard operations can be added behind the API in Stage 19 without touching any code above Stage 2. Correctness first with the simpler representation, optimization deferred to when we have profiling data.

**Key constraint:** Nothing above Stage 2 reads `board.squares[]` directly. This is enforced by code review, not by the type system.

---

### ADR-002: Search Architecture — BRS/Paranoid Hybrid, Not Pure MaxN or Pure Paranoid

**Date:** 2026-02-18
**Status:** Active
**Affects:** Stage 7 ([[stage_07_plain_brs]]), Stage 8 ([[stage_08_brs_hybrid]]), Stage 11 ([[stage_11_hybrid_integration]])

**Decision:** The tactical search uses BRS tree structure (one opponent reply per ply, alpha-beta compatible) with reply selection driven by a pre-search board read that blends "objectively strongest" with "most harmful to me" weighted by likelihood given the board's incentive structure.

**Alternatives considered:**
- **Pure BRS (standard):** Picks the objectively strongest opponent move. Problem: that might be irrelevant to the root player (Yellow capturing Green's queen).
- **Pure Paranoid:** Assumes all opponents minimize root player's score. Problem: in 4-player FFA this IS realistic (opponents target the leader), but it doesn't account for opponents having their own goals. Blue might sacrifice their queen to hurt you -- but they won't actually do that because it leaves them exposed.
- **Pure MaxN:** Each player maximizes their own score. Problem: no alpha-beta pruning possible. Tree search becomes prohibitively expensive.

**Why this was chosen:** The hybrid captures the insight that FFA opponents ARE somewhat paranoid toward the leader but also have their own interests. The board scanner (< 1ms) provides the context that makes reply selection realistic. The BRS tree structure preserves alpha-beta compatibility for efficient search. The hybrid scoring formula (`score = harm_to_root * likelihood + objective_strength * (1 - likelihood)`) is hand-tunable and measurable via self-play.

**Key insight from planning:** "Standard Paranoid IS realistic in FFA -- the weakness isn't that opponents DON'T target you, it's that Paranoid doesn't account for opponents having their own goals that might override targeting you."

---

### ADR-003: Evaluation Architecture — Dual-Head NNUE with Evaluator Trait

**Date:** 2026-02-18
**Status:** Active
**Affects:** Stage 6 ([[stage_06_bootstrap_eval]]), Stage 14 ([[stage_14_nnue_design]]), Stage 16 ([[stage_16_nnue_integration]])

**Decision:** The NNUE has two output heads: a BRS scalar head (centipawn value from one player's perspective) and an MCTS value head (4-player value vector with softmax). Both outputs are accessed through a common `Evaluator` trait (`eval_scalar` + `eval_4vec`) defined in Stage 6. Bootstrap handcrafted eval implements the trait first. NNUE implements it later. Search never knows which implementation is behind the trait.

**Alternatives considered:**
- **Separate networks for BRS and MCTS:** Doubles inference cost, no shared representation learning.
- **Single scalar head only:** MCTS needs 4-player value vectors for MaxN backpropagation. Converting scalar to 4-vec loses information.
- **No trait, direct function calls:** Makes the NNUE swap in Stage 16 a surgical rewrite instead of a clean implementation swap.

**Why this was chosen:** Dual-head shares the feature transformer and hidden layers (most of the computation) while providing the right output shape for each search type. The trait makes the bootstrap -> NNUE swap a single implementation change. BRS and MCTS both call through the trait without knowing or caring what's behind it.

---

### ADR-004: NNUE Feature Set — HalfKP-4

**Date:** 2026-02-18
**Status:** Active
**Affects:** Stage 14 ([[stage_14_nnue_design]])

**Decision:** Per perspective: `(piece_square, piece_type, relative_owner)`. 160 squares x 7 types x 4 relative owners = 4,480 features per perspective (~30 active per position). 4 accumulators (one per player perspective). SCReLU activation.

**Alternatives considered:**
- **Simpler feature set (material only):** Too crude. The whole point of NNUE is capturing positional subtleties.
- **King bucketing from the start:** 20 buckets x 4,480 = 89,600 features. Too large for Phase 1. Deferred to Phase 2 within Stage 14.
- **Higher-order features (piece pairs, square control):** Explosion in feature count. Start simple, add complexity if needed.

**Why this was chosen:** HalfKP is proven in Stockfish. The 4-player adaptation (relative owner instead of binary color) is the minimal extension. 4,480 features with ~30 active is sparse enough for efficient incremental updates (add/subtract 1-3 columns per non-king move).

---

### ADR-005: Stage Ordering — 20 Stages in 6 Tiers

**Date:** 2026-02-19
**Status:** Active
**Affects:** All stages

**Decision:** Restructure from 19 stages (v2.0) to 20 stages organized into 6 tiers:

```
TIER 1 — FOUNDATION: 0-5 (Skeleton, Board, Movegen, Rules, Protocol, Basic UI)
TIER 2 — SIMPLE SEARCH: 6-7 (Bootstrap Eval + Evaluator trait, Plain BRS + Searcher trait)
TIER 3 — STRENGTHEN SEARCH: 8-11 (Hybrid Layer, TT, MCTS, Integration)
TIER 4 — MEASUREMENT: 12-13 (Self-Play, Time Management)
TIER 5 — LEARN: 14-16 (NNUE Design, Training, Integration)
TIER 6 — POLISH: 17-19 (Variant Tuning, Full UI, Optimization)
```

**Key changes from v2.0:**
- Split old Stage 7 (BRS + everything) into Stage 7 (Plain BRS) and Stage 8 (Hybrid Layer)
- Moved Self-Play from 17 (second to last) to 12 (right after search works)
- Moved Time Management from 16 to 13
- Moved NNUE stages to 14-16
- Removed NNUE and Full UI as dependencies for Variant Tuning (now Stage 17)
- Moved Full UI to 18 (blocks nothing)
- Added Stage 19 for optimization

**Why this was chosen:** The v2.0 ordering had the cart before the horse in multiple places:
1. Self-play at Stage 17 meant you couldn't measure whether changes helped until nearly the end.
2. BRS was overloaded -- standard search + hybrid innovation + pruning techniques all in one stage.
3. MCTS was bolted on late rather than designed in through the Searcher trait.
4. Game mode variants were blocked by NNUE and Full UI, neither of which they actually need.
5. No maintenance invariants -- nothing enforced "don't break what works."

The user's directive: "get your steps in order! figure out what needs to be built and functional first before everything else." Foundation first, reliable simple gameplay, then complexity.

---

### ADR-006: Bitboards vs. NNUE Clarification

**Date:** 2026-02-19
**Status:** Active
**Affects:** Understanding only (no code impact)

**Decision:** These are completely different things at different layers. This decision exists solely to prevent future confusion.

- **Bitboard** = board representation technique. Performance optimization for the board, movegen, and attack queries. Has nothing to do with evaluation or neural networks.
- **NNUE** = neural network that reads board state (regardless of storage format) and outputs "how good is this position?" Replaces the handcrafted eval. Does not contain or use bitboards.

**Context:** During planning, there was a misconception that bitboards existed inside NNUE. This was clarified and the distinction is now explicit.

---

### ADR-007: Huginn Telemetry — Compile-Gated Ghost Observer

**Date:** 2026-02-18
**Status:** Active
**Affects:** Stage 0 and every subsequent stage

**Decision:** Huginn exists from Stage 0, grows per stage, and is controlled by a single compile flag (`cfg(feature = "huginn")`). When off, the engine compiles as if Huginn does not exist. When on, it operates as a post-hoc reader with zero engine impact.

**Alternatives considered:**
- **Huginn as a late addition (original plan had it at Stage 11):** The user pointed out you need the tracer while building, not after. "How do you debug Stage 3 without observation points?"
- **Runtime toggle instead of compile-time:** Runtime checks introduce branches into hot paths. Even a single `if huginn_enabled` in the inner search loop costs measurable performance.
- **Logging framework (log4rs, tracing):** General-purpose logging frameworks allocate, format strings, and add branches. Huginn's macro-to-nothing approach is strictly zero-cost when off.

**Why this was chosen:** The "snitch at every gate" model means bugs are witnessed at the moment they happen, not reconstructed after the fact. Compile-gating means zero performance cost in production. Having it from Stage 0 means every stage gets observation points from day one.

---

### ADR-008: Huginn Reporting — JSONL with Ring Buffer

**Date:** 2026-02-19
**Status:** Active
**Affects:** Stage 0 ([[stage_00_skeleton]]), `AGENT_CONDUCT.md` ([[AGENT_CONDUCT]]) Section 3

**Decision:** Huginn observations are stored as JSON Lines (one JSON object per line) in a pre-allocated ring buffer (65,536 entries, oldest silently overwritten). Optional file sink for persistent logging. 5-level trace hierarchy (Session > Search > Phase > Path > Gate). 4 verbosity levels (Minimal < 10/search, Normal 20-50, Verbose 200-2000, Everything 10,000+).

**Why this was chosen:** JSONL is streamable, greppable, one observation per line, standard tooling (`jq`, `grep`, Python). Ring buffer satisfies "no allocation during search" and "zero engine impact." File sink is opt-in for deep analysis. Four verbosity levels prevent information overload at normal usage while allowing firehose when debugging.

**Specified in:** AGENT_CONDUCT.md Section 3 (not in MASTERPLAN, to keep MASTERPLAN focused on engine specs).

---

### ADR-009: Searcher Trait — Defined Early, Implemented Incrementally

**Date:** 2026-02-19
**Status:** Active
**Affects:** Stage 7 ([[stage_07_plain_brs]]), Stage 10 ([[stage_10_mcts]]), Stage 11 ([[stage_11_hybrid_integration]])

**Decision:** The `Searcher` trait (`fn search(&mut self, position, budget) -> SearchResult`) is defined in Stage 7. BRS implements it in Stage 7. MCTS implements it in Stage 10. The hybrid controller in Stage 11 composes two `Searcher`s. No glue code, no retrofit. The `&mut self` is significant: searchers maintain internal mutable state (TT, history heuristic, MCTS tree).

**Alternatives considered:**
- **No trait, direct function calls:** Makes the hybrid controller in Stage 11 a hardcoded orchestrator that knows the internals of both BRS and MCTS. Adding a third search method later requires rewriting the controller.
- **Trait defined later (in Stage 11):** Requires retrofitting BRS and MCTS to implement it. Changes to stable code.

**Why this was chosen:** Define the interface before the second implementation exists. When MCTS is built in Stage 10, it implements an existing contract rather than inventing a new one. The hybrid controller composes through the trait -- it doesn't need to know how BRS or MCTS work internally. This makes MCTS integration "organic and native rather than hardcoding it into the engine" (user's words).

---

### ADR-010: Self-Play Placement — Right After Search Works

**Date:** 2026-02-19
**Status:** Active
**Affects:** Stage 12 ([[stage_12_self_play]]), and every stage after it

**Decision:** Self-play framework moves from Stage 17 (v2.0, second to last) to Stage 12 (v3.0, right after hybrid integration).

**Why this was chosen:** "Can't improve what you can't measure." In v2.0, the engine couldn't measure whether BRS hybrid tuning, MCTS parameters, or NNUE training actually helped until nearly the end. Moving self-play to Stage 12 means:
- BRS hybrid heuristic tuning (Stage 8) gets measurement in the next session
- MCTS parameter tuning has measurement
- NNUE training (Stage 15) uses the full self-play framework for data generation (no more "minimal self-play loop" hack)
- Every subsequent stage can validate against regression

---

### ADR-011: Project Management — STATUS + HANDOFF + DECISIONS + README

**Date:** 2026-02-19
**Status:** Active
**Affects:** All sessions

**Decision:** Four management documents complement the technical documents:

| Document | Purpose | Update Frequency |
|----------|---------|-----------------|
| `STATUS.md` ([[STATUS]]) | Where is the project? What stage? What's done? | Every session end |
| `HANDOFF.md` ([[HANDOFF]]) | Mid-stage continuity. What was I doing? What's next? | Every session end |
| `DECISIONS.md` (this) | Why were architectural choices made? | When decisions are made |
| `README.md` | 5-minute project overview for fast orientation | Major milestones |

**Why:** AI agents lose context between sessions. Without these files, each new session spends 30+ minutes re-orienting by reading all audit logs, inferring project state, and potentially re-arguing settled decisions. These four files cost 5 minutes to update per session and save 30 minutes of orientation.

---

### ADR-012: BRS Turn Order — Natural Game Order, Not Alternating MAX-MIN

**Date:** 2026-02-21
**Status:** Active
**Affects:** Stage 7 ([[stage_07_plain_brs]]), Stage 8 ([[stage_08_brs_hybrid]]), Stage 10 ([[stage_10_mcts]])

**Decision:** BRS uses natural 4-player turn order (R→B→Y→G→R→...) rather than the MASTERPLAN's described alternating MAX-MIN-MAX-MIN model (Root, Opp1, Root, Opp2, Root, Opp3).

**Alternatives considered:**
- **MASTERPLAN alternating model (MAX-MIN-MAX-MIN):** Root player gets a MAX node, then each opponent gets a MIN node, then back to root. This requires manual `board.set_side_to_move()` between the root MAX node and each opponent MIN node.
- **Natural turn order (chosen):** Each player takes one turn in R→B→Y→G order. Root player = MAX node (full branching, alpha-beta). Each opponent = MIN node (single best reply). The Board naturally advances `side_to_move` via `make_move`.

**Why this was chosen:** `unmake_move(board, mv, undo)` infers the previous player via `prev_player(side_to_move)` — it does NOT save `side_to_move` in `MoveUndo`. If `set_side_to_move()` is called manually between `make_move` and `unmake_move`, it corrupts the restoration logic. The alternating model requires exactly this. The natural turn order avoids this entirely: `make_move` advances the player normally, `unmake_move` restores it normally. Alpha-beta still prunes effectively at MAX nodes; MIN nodes are single-branch and pass scores through without issue.

**Consequence:** Any future code that calls `set_side_to_move()` inside the search loop (e.g., for null move or eliminated-player skip) must restore symmetrically before calling `unmake_move`. See [[downstream_log_stage_07]] for the explicit requirement and [[Component-Search]] for implementation notes.

---

## How to Add a New Decision

Copy this template:

```
### ADR-NNN: [Short Title]

**Date:** YYYY-MM-DD
**Status:** Active | Superseded by ADR-XXX | Deprecated
**Affects:** Stage X, Stage Y

**Decision:** [What was decided]

**Alternatives considered:**
- [Alternative 1]: [Why not]
- [Alternative 2]: [Why not]

**Why this was chosen:** [Reasoning]
```

Number sequentially. Never delete or renumber existing decisions. If a decision is superseded, mark it as such and reference the new decision.

---

*End of Decision Log v1.0*
