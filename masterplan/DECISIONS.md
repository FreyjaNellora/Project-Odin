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

**Status: SUPERSEDED by ADR-015**
Huginn was retired in Stage 8. The `tracing` crate replaced it. See ADR-015.

**Date:** 2026-02-18
**Status:** Superseded by ADR-015
**Affects:** Stage 0 and every subsequent stage

**Decision:** Huginn exists from Stage 0, grows per stage, and is controlled by a single compile flag (`cfg(feature = "huginn")`). When off, the engine compiles as if Huginn does not exist. When on, it operates as a post-hoc reader with zero engine impact.

**Alternatives considered:**
- **Huginn as a late addition (original plan had it at Stage 11):** The user pointed out you need the tracer while building, not after. "How do you debug Stage 3 without observation points?"
- **Runtime toggle instead of compile-time:** Runtime checks introduce branches into hot paths. Even a single `if huginn_enabled` in the inner search loop costs measurable performance.
- **Logging framework (log4rs, tracing):** General-purpose logging frameworks allocate, format strings, and add branches. Huginn's macro-to-nothing approach is strictly zero-cost when off.

**Why this was chosen:** The "snitch at every gate" model means bugs are witnessed at the moment they happen, not reconstructed after the fact. Compile-gating means zero performance cost in production. Having it from Stage 0 means every stage gets observation points from day one.

---

### ADR-008: Huginn Reporting — JSONL with Ring Buffer

**Status: SUPERSEDED by ADR-015**
Huginn was retired in Stage 8. The `tracing` crate replaced it. See ADR-015.

**Date:** 2026-02-19
**Status:** Superseded by ADR-015
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

### ADR-013: Score-Aware Engine — Point Scoring as First-Class Eval Input

**Date:** 2026-02-21
**Status:** Active
**Affects:** Stage 8 ([[stage_08_brs_hybrid]]), Stage 10 ([[stage_10_mcts]]), Stage 12 ([[stage_12_self_play]]), Stage 16 ([[stage_16_nnue_integration]])

**Decision:** The engine must treat FFA point scoring as a first-class evaluation input, not just a side effect of captures. Banked points, point differential, capture opportunity density, and farm denial must be explicitly modeled. This is not a single-stage feature — it threads through multiple stages as each provides the mechanism for a different aspect.

**The problem:** BRS optimizes for "don't lose to the biggest threat" (defensive). Paranoid assumes all opponents cooperate against you (more defensive). MCTS explores broadly but inherits eval blindness. The bootstrap eval (Stage 6) scores material balance — pieces on the board — but not points already banked. An engine with a queen and 0 points evaluates identically to an engine with a queen and 30 points. In FFA, the player with the most points wins. The engine currently plays "good chess" but doesn't play to WIN THE SCORING GAME.

**Stage-by-stage implementation:**

| Stage | What Gets Added | Why |
|---|---|---|
| 6 (Bootstrap Eval) | Nothing — it's temporary scaffolding | NNUE replaces it. Not worth adding complexity. |
| 8 (Board Context) | **Scoring context** added to board scanner: point standings per player, lead/deficit magnitude, capture opportunity density, farm denial assessment. Context feeds into reply scoring — behind in points = weight aggressive moves higher; ahead = weight denial/defense higher. | Board context is pre-search (< 1ms). Adding scoring reads here is architecturally clean and directly influences move selection. |
| 10 (MCTS) | **Point-based terminal evaluation.** When playouts reach terminal states, score using actual game points, not just material eval. "Red won with 45 points" vs "Red won with 22 points" must produce different backpropagation values. | Makes MCTS naturally discover aggressive lines — more captures = more points = higher playout scores. |
| 11 (Hybrid) | **Phase allocation considers scoring context.** When behind in points, allocate more MCTS budget (explore aggressive/tactical lines). When ahead, lean on BRS (solid, defensive). | The BRS/MCTS split already exists; this makes the split score-aware. |
| 12 (Self-Play) | **Point differential as a metric** alongside win rate. Report average points scored, point spread, and capture efficiency per game. | "Did we win?" is insufficient. "Did we win AND outscore by a wide margin?" reveals whether the engine is passively squeaking by or actively dominating. |
| 16 (NNUE) | **Score-context features** in NNUE input: banked points per player, point differential, game phase (early/mid/late based on pieces remaining). Training signal includes point differential, not just win/loss. | The NNUE learns "this board + me 10 points behind = worse than same board + me 10 points ahead." Without score features, it's blind to the actual objective. |

**What this produces:** An engine that captures when tactically sound (BRS), pursues scoring when behind (board context modulates aggression), discovers high-scoring lines (MCTS terminal eval), and learns nuanced scoring patterns (NNUE). Defense and aggression are balanced by context, not hardcoded.

**Alternatives considered:**
- **Add point terms to bootstrap eval now:** Wasted effort — NNUE replaces it. The bootstrap just needs to not crash.
- **Let NNUE learn aggression purely from self-play:** Risky. If the training signal is win/loss only, NNUE might learn passive play that wins by not losing. Score-aware training signal is needed.
- **Hardcode aggression heuristics:** Fragile. "Always capture when possible" is dumb. "Capture when the scoring context says it's worth it" is smart. Context-driven > rule-driven.

**Key insight:** "The faster you establish a point lead and prevent others from farming pieces on the board, the more you guarantee victory." This is the FFA meta-strategy. The engine must understand it.

---

### ADR-014: UI Vision — Three-Column Desktop Application

**Date:** 2026-02-21
**Status:** Active
**Affects:** Stage 5 ([[stage_05_basic_ui]]), Stage 18 ([[stage_18_full_ui]])

**Decision:** The UI is a three-column desktop application: controls left, board center, information right. The design philosophy is "window into the engine's reasoning" — not just a chess viewer but a tool for understanding what the AI is thinking.

**Layout:**

```
┌──────────┬─────────────────────────┬─────────────────────┐
│ CONTROLS │                         │ SCORES (2×2 grid)   │
│          │                         │                     │
│ Mode     │                         ├─────────────────────┤
│ Play As  │                         │ ANALYSIS            │
│ Depth    │     14×14 BOARD         │ best move, eval,    │
│ Delay    │     (zoomable)          │ depth, nodes, NPS,  │
│          │                         │ time, PV line       │
│ [New]    │                         ├─────────────────────┤
│ [Terrain]│                         │ SEARCH TRACE        │
│ [Pause]  │                         │ depth-by-depth      │
│ [Step]   │                         │ iterative deepening │
│ [Round]  │                         │ watch the engine    │
│ [FEN]    │                         │ change its mind     │
│          │                         ├─────────────────────┤
│          │                         │ GAME LOG            │
│          │                         │ color-coded moves   │
│          │                         │ with eval + depth   │
└──────────┴─────────────────────────┴─────────────────────┘
│              COLLAPSIBLE DEBUG STRIP (raw protocol)       │
└───────────────────────────────────────────────────────────┘
```

**Left Panel — Controls:**
- Play mode: Manual (move everyone), Semi-Auto (play one color, AI plays rest), Full Auto (watch all AIs)
- Play as: choose color in Semi-Auto
- Depth: search depth slider (1–8+)
- Delay: AI move speed (0ms–2s)
- Buttons: New Game, New Game (Terrain), Pause/Resume, Step One Move, Step One Round, Show FEN

**Center Panel — Board:**
- Zoomable 14×14 cross-shaped board with coordinate labels (a–n files, 1–14 ranks)
- Color-coded pieces per player
- Eliminated players' pieces grayed out on board
- Last move highlighted, selected square highlighted

**Right Panel — Four Stacked Sections:**
1. **Scores:** 2×2 color-coded grid. Current capture points + checkmate bonuses. Active player highlighted.
2. **Analysis:** Final search result — best move, eval (cp), depth, nodes, NPS, time, principal variation.
3. **Search Trace:** Every completed iterative deepening depth — score, best line, nodes, time. Watch the engine's opinion evolve. Clears on next search start.
4. **Game Log:** Sequential color-coded moves. Format: `1. Red: f4  1. Blue: d9  1. Yellow: i11  1. Green: Nl10l11`. Each entry annotated with eval + depth at time of play. Scrollable.

**Bottom — Debug Strip:** Collapsible raw protocol output. Hidden by default.

**Design principle:** Fast. Board is the focus. Right panel tells you everything the engine is thinking without digging through logs. Search Trace is the differentiator — turns the app from a chess viewer into a reasoning observatory.

**Connection to ADR-013:** The Scores panel assumes points are central. When score-aware eval lands (Stage 8+), the Analysis panel may need to display scoring context alongside centipawn eval — or the centipawn value itself must incorporate scoring context so the single number is meaningful to the user.

---

### ADR-015: Retire Huginn, Adopt `tracing` Crate

**Date:** 2026-02-23
**Status:** Active
**Supersedes:** ADR-007, ADR-008
**Affects:** All stages (Stage 0 through 19)

**Decision:** The custom Huginn telemetry system (compile-gated ghost observer with `huginn_observe!` macro, `HuginnBuffer` ring buffer, JSONL trace format) is retired entirely. All Huginn code has been deleted from the engine. The `tracing` crate (v0.1) replaces it as the observability solution. Diagnostic output uses `tracing::debug!`, `tracing::info!`, and `tracing::trace!` macros at key engine boundaries.

**What was removed:**
- `odin-engine/src/huginn/` directory (mod.rs + buffer.rs, ~350 lines)
- `huginn` feature flag from Cargo.toml
- `huginn_observe!` macro (both on/off branches)
- All `#[cfg(feature = "huginn")]` blocks across search/board_scanner modules
- Huginn-specific integration tests (3 tests from stage_00_proof_of_life.rs)

**What was added:**
- `tracing = "0.1"` dependency in Cargo.toml
- Tracing instrumentation to be added incrementally at former gate points

**Alternatives considered:**
- **Keep Huginn, fix the plumbing:** The fundamental problem was that `HuginnBuffer` had no global instance. Threading `&mut HuginnBuffer` through every API signature (Board, MoveGen, GameState, Eval, Search) would pollute the entire API surface. A global/static instance would require unsafe code or a mutex in hot paths. Eight stages of deferred wiring proved this was impractical.
- **Runtime-toggled logging (log4rs):** Adds branches to hot paths. The `tracing` crate's subscriber model avoids this — unsubscribed spans/events compile to near-zero overhead.
- **Keep Huginn as dead code, wire later:** Carrying dead code for 8 stages with no plan to resolve the plumbing problem is deferred debt. The user explicitly requested removal after identifying the pattern.

**Why `tracing` was chosen:**
1. **Works out of the box.** `tracing::debug!("msg", field = value)` — no buffer plumbing, no feature gates, no macro gymnastics.
2. **Zero-cost when filtered.** With no subscriber attached (production), tracing macros compile to no-ops. With a subscriber (development), output goes to stderr, files, or structured formats.
3. **Industry standard.** Tokio ecosystem, well-maintained, familiar to Rust developers.
4. **Incremental adoption.** Can add `tracing::debug!` calls one at a time without touching API signatures. No Big Bang wiring required.

**Lesson learned:** If a system requires plumbing through every API surface but that plumbing is deferred at every stage, the design is fundamentally flawed. This should have been caught by Stage 2 at latest. A new deferred-debt escalation rule has been added to AGENT_CONDUCT to prevent similar accumulation.

---

### ADR-016: Gumbel MCTS over UCB1

**Date:** 2026-02-26
**Status:** Active
**Affects:** Stage 10 ([[stage_10_mcts]]), Stage 11 ([[stage_11_hybrid_integration]]), Stage 13 ([[stage_13_time_management]])

**Decision:** Replace UCB1 selection at the MCTS root with Gumbel-Top-k sampling + Sequential Halving. Non-root nodes use an improved policy formula with prior and progressive history terms. The prior policy before NNUE is derived from move ordering scores: `pi(a) = softmax(ordering_score(a) / T)` where `T` is a temperature parameter (default 50).

**Alternatives considered:**
- **Standard UCB1:** Optimizes cumulative regret (overall exploration quality), but the engine only plays one move — what matters is the quality of the final choice (simple regret). UCB1 needs 16+ simulations to reliably converge; Gumbel MCTS works with as few as 2.
- **PUCT (AlphaZero-style):** Designed for single-player MCTS with a learned policy network. Degenerates without a good policy prior. Gumbel is more robust with weak priors.
- **Thompson Sampling:** Bayesian approach that works well but lacks the theoretical guarantees of Gumbel's policy improvement.

**Why this was chosen:** In the BRS→MCTS hybrid, MCTS gets a limited simulation budget (Phase 2 residual time). Gumbel-Top-k at the root provides provable policy improvement even with 2 simulations — critical when the budget is tight. Sequential Halving at the root progressively eliminates weaker moves, concentrating simulations on the best candidates. The Gumbel noise (sampled from Gumbel(0,1) distribution) provides principled exploration at the root without the over-exploration that UCB1 exhibits at low simulation counts.

**Key mechanism:** At root: sample `g(a) ~ Gumbel(0,1)` for each action, compute `g(a) + log(pi(a))` to select Top-k candidates, then use Sequential Halving to eliminate candidates by comparing `sigma(g(a) + log(pi(a)) - Q(a))` (where `sigma` = logistic function). At non-root: standard tree policy with prior + progressive history.

**Pre-NNUE prior:** `ordering_score(a)` combines TT hint bonus (10000), MVV-LVA capture scores, killer bonuses, countermove bonuses, and history heuristic values — all already computed by BRS's move ordering pipeline. Softmax with temperature T=50 converts these to a probability distribution. This is a weak prior, but Gumbel noise provides exploration and Sequential Halving corrects via Q-values.

---

### ADR-017: Progressive History — BRS History Table Shared with MCTS

**Date:** 2026-02-26
**Status:** Active
**Affects:** Stage 10 ([[stage_10_mcts]]), Stage 11 ([[stage_11_hybrid_integration]])

**Decision:** MCTS non-root selection incorporates BRS history heuristic scores via the Progressive History formula: `PH(a) = H(a) / (N(a) + 1)` where `H(a)` is the history score from BRS's history table and `N(a)` is the MCTS visit count of the action. This gives MCTS a warm start using knowledge BRS already computed during Phase 1.

**Alternatives considered:**
- **No warm start (cold MCTS):** MCTS starts from scratch every search. With limited Phase 2 budget, many simulations are wasted on moves BRS already identified as poor.
- **Persistent MCTS tree between moves:** Complex lifecycle management, stale data risk when the board changes. Deferred to Stage 13 measurement.
- **Direct injection of BRS scores as MCTS priors:** Conflates tactical scores with strategic exploration. History heuristic is a softer signal — it reflects which moves produced cutoffs, not absolute scores.

**Why this was chosen:** The `1/(N(a)+1)` decay is elegant — progressive history dominates when MCTS visits are low (warm start) and fades as MCTS accumulates its own data (MCTS takes over). BRS already computes history tables during Phase 1; sharing them costs nothing. The hybrid controller extracts the history table after BRS completes and passes it to MctsSearcher before Phase 2 begins.

**Key constraint:** History table is NOT persistent across moves. Extracted from BRS after Phase 1, consumed by MCTS in Phase 2, discarded. Full inter-move persistence deferred to Stage 13 measurement to determine if stale history helps or hurts.

---

### ADR-018: Dual Governance Files -- Standard vs. Mythos

**Date:** 2026-04-01
**Status:** Active
**Affects:** All sessions (governance infrastructure)

**Decision:** Maintain two parallel sets of agent governance files:

| File | For | Strategy |
|---|---|---|
| `CLAUDE.md` | All models (default) | Full procedural orientation with step-by-step instructions |
| `CLAUDE_MYTHOS.md` | Mythos/frontier-class models | Goal-oriented orientation, same constraints |
| `masterplan/AGENT_CONDUCT.md` | All models (default) | Full procedural rules, anti-spiral guardrails, multi-step lifecycles |
| `masterplan/AGENT_CONDUCT_MYTHOS.md` | Mythos/frontier-class models | Condensed rules, same domain knowledge, same hard constraints |

`CLAUDE.md` contains a model-tier router at the top directing frontier models to the `_MYTHOS` variants.

**What the Mythos files KEEP verbatim:**
- All hard constraints (search depth policy, First Law invariants, autonomy boundary tables)
- All domain knowledge (naming conventions, named constants, pitfall tables in Section 2)
- All approval gates (blocking issue escalation, stop-and-ask triggers)
- All of Section 2 (audit checklist, 26 categories)
- All of Section 4 (what tracing cannot catch)

**What the Mythos files REMOVE or CONDENSE:**
- Procedural step-by-step instructions (Stage Entry 7-step → one paragraph)
- Prescribed file reading orders
- Timing and sequencing ceremony
- Anti-spiral debugging rules and flowcharts (condensed to 5 core principles)
- Compensating guardrails that frontier models do not need

**Alternatives considered:**
- **Single file with model-conditional sections:** Harder to maintain, harder to read. Model-tier routing to separate files is cleaner.
- **Remove procedural content from the main files:** Would regress coverage for weaker models. Dual-file preserves both audiences.
- **No Mythos variant:** Frontier models can follow the standard files, but the procedure-heavy format adds unnecessary friction and can prime over-literal behavior.

**Why this was chosen:** Procedural scaffolding compensates for known failure modes in weaker models (losing track of steps, skipping prep). Frontier models have those failure modes less often but can still be primed toward over-literal step-counting by procedure-heavy prompts. The Mythos files let the model reason from goals and constraints rather than execute a checklist. The domain knowledge (pitfall tables, constants, invariants) is preserved because that is information the model cannot derive from first principles.

**Maintenance rule:** When AGENT_CONDUCT.md is updated with new domain knowledge (a new pitfall table, a new named constant, a new hard constraint), the corresponding content must be added to AGENT_CONDUCT_MYTHOS.md. When AGENT_CONDUCT.md adds new procedural scaffolding, evaluate whether it represents new knowledge (add to Mythos) or compensating procedure (omit from Mythos).

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
