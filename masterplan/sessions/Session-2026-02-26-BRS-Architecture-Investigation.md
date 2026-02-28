---
type: session
date: 2026-02-26
stage: post-stage-9 (gameplay + architecture investigation)
tags: [stage/09, area/search, area/eval, area/board-scanner]
---

# Session: 2026-02-26 -- Narrowing Fix, Hanging Penalty Experiment, BRS Architecture Investigation

## Goal

Three phases:
1. Fix three gameplay bugs from v0.4.2-pst testing (hanging knight on j5, Yellow queen exposure, Red hanging pawn on e4).
2. Experiment with eval-side hanging piece penalty (reverted).
3. Deep investigation into BRS architecture — why the engine can't exploit multi-move tactics.

## What Happened

### Phase 1: Progressive Narrowing Fix (v0.4.3-narrowing)

User reported three gameplay bugs. Root cause analysis traced the j5 hanging knight to progressive narrowing being too aggressive at depth 7+ (`NARROWING_DEEP = 3`), which pruned the opponent's capture move before the search explored it.

**Fix applied to `board_scanner.rs`:**
- Widened narrowing limits: `NARROWING_SHALLOW` 10 → 12, `NARROWING_MID` 6 → 8, `NARROWING_DEEP` 3 → 5
- Added **root-capture protection**: in `select_hybrid_reply`, moves that capture the root player's pieces are exempt from narrowing truncation. Implementation: partition moves into `root_captures` and `soft_threats`, only truncate `soft_threats`, then recombine.
- Test assertions in `stage_08_brs_hybrid.rs` updated for new limit ranges.
- `ENGINE_VERSION` bumped to `v0.4.3-narrowing`.

User tested: development improved significantly (e4, Ni3, Be2, k4, Bg4, Bh5 — solid mixed development). No more knight sacrifice regression.

### Phase 2: Hanging Piece Penalty (REVERTED)

Attempted to add an eval-side `hanging_piece_penalty` to `multi_player.rs` that penalized undefended pieces under attack at half piece value (capped at 500cp). Integrated into `eval_for_player` formula.

**Result: REGRESSION.** User tested v0.4.3-hanging and reported Red's Nf3 retreated to e1, then king walked Kh1→g2. Red's score dropped from ~4400 to ~3889 while opponents stayed ~4400. The penalty double-counted capture threats already handled by the search tree, making all forward deployment look dangerous.

**Fully reverted.** Only the narrowing fix (Phase 1) was kept. A comment documenting why was added to `eval/mod.rs`.

### Phase 3: BRS Architecture Investigation

User observed that even with v0.4.3-narrowing, Red didn't exploit Blue's exposed king after b6d6. User requested: "a deeper search into what is going on with turn order, TTs, BRS/paranoid, and how our diverging from how BRS is traditionally used might impact things."

**Findings:**

#### Finding 1: TT Not Player-Aware (Latent Bug)
TT hash includes `side_to_move` but NOT `root_player`. A TT entry stored when searching for Red could be probed when searching for Blue if the board + side_to_move match. Currently safe because protocol creates a fresh `BrsSearcher` per `go` command (line 237 `protocol/mod.rs`), so TT never persists between searches. **Becomes a real bug** when TT is persisted (easy perf win) or shared in MCTS.

#### Finding 2: Depth 7 Turn Order Asymmetry
At depth 7: Red(MAX,d7) → Blue(MIN,d6) → Yellow(MIN,d5) → Green(MIN,d4) → Red(MAX,d3) → Blue(MIN,d2) → Yellow(MIN,d1) → qsearch. Red gets 2 MAX moves 4 plies apart (~1.75 full rounds). The engine cannot see a 2-move tactical plan (e.g., f2f4 + queen attack on exposed king) because the second Red move is at depth 3 with very limited remaining search.

#### Finding 3: Paranoid Opponent Modeling Too Aggressive
The hybrid scoring formula in `board_scanner.rs` (lines 578-656) blends:
- `harm_to_root * likelihood` (paranoid component — ~80% weight)
- `objective_strength * (1 - likelihood)` (realistic component — ~20% weight)

`LIKELIHOOD_BASE_TARGETS_ROOT = 0.7` means even when an opponent has a much better target (Blue's exposed king), the engine still assumes 70% chance they'll attack Red. This makes the engine play as if all 3 opponents are coordinating against it, which is unrealistic in FFA.

**Proposed fix:** Lower `LIKELIHOOD_BASE_TARGETS_ROOT` from 0.7 to 0.5, increase `LIKELIHOOD_EXPOSED_PENALTY` from 0.3 to 0.5, so opponents with obvious weaknesses are modeled more realistically.

#### Finding 4: Negamax Cannot Work in 4-Player
Standard BRS uses negamax (`score = -recursive_call`). This doesn't work in 4-player chess because the game is not zero-sum — one player's loss doesn't equal another's gain. Our implementation correctly uses explicit MAX/MIN nodes with `eval_scalar(position, root_player)` from root's perspective throughout. This is the right approach but limits us to depth-based search (no negamax optimizations). **Not fixable — structural.**

#### Finding 5: TT Fresh Per Search (Easy Perf Win)
Protocol creates a fresh `BrsSearcher` (and thus fresh TT) per `go` command. Iterative deepening within a single search reuses TT, but between moves all knowledge is lost. Fix: hoist `BrsSearcher` into protocol handler state, call `tt.increment_generation()` between searches. **Easy win, do before Stage 10.**

#### Finding 6: Human 4PC Strategy Research
Web research on human 4-player chess strategy confirmed several design assumptions and challenged others:
- Knights are defensive pieces (correct — our flattened gradient matches)
- Bishops > rooks in 4PC (our equal 500cp valuation may undervalue bishops)
- Don't castle (king safest behind pawns in center) — matches our king safety eval
- Queens out early is OK (we may over-penalize queen development)
- Avoid trades (non-zero-sum — trading benefits the non-traders) — our eval doesn't capture this
- Central control matters — our PSTs already reward this

## Components Touched

- [[Component-BoardScanner]] (`odin-engine/src/search/board_scanner.rs`) — narrowing limits widened, root-capture protection added
- [[Component-Eval]] (`odin-engine/src/eval/mod.rs`) — hanging penalty comment (function added then reverted)
- `odin-engine/src/eval/multi_player.rs` — linter formatting only (hanging penalty fully reverted)
- `odin-engine/src/protocol/emitter.rs` — ENGINE_VERSION bumped
- `odin-engine/tests/stage_08_brs_hybrid.rs` — test assertion ranges updated

## Discoveries

**Double-counting in eval vs search.** Adding eval-side penalties for tactical threats (hanging pieces) that the search tree already handles causes the engine to become excessively passive. The search sees the capture threat and avoids it; the eval ALSO penalizes the position, making the engine retreat pieces that were actually fine. **Rule: tactical threats belong in search (narrowing protection, move ordering), not eval.**

**The 80/20 paranoid blend is too paranoid for FFA.** With 3 opponents and `LIKELIHOOD_BASE_TARGETS_ROOT = 0.7`, the engine behaves as though opponents are coordinating. In FFA, opponents attack whoever is weakest/most exposed — not necessarily the root player. The hybrid scoring model needs to be more sensitive to "who is actually the best target."

**Depth asymmetry compounds with paranoid modeling.** Even if Red COULD see the f2f4 + queen attack plan, the paranoid model would assume Blue "defends" by attacking Red, not that Blue would make its own objective-best move (which might leave its king exposed).

## Issues Created/Resolved

**Created:**
- [[Issue-BRS-Paranoid-Opponent-Modeling]] — 80/20 blend too paranoid for FFA; proposed likelihood tuning
- [[Issue-TT-Not-Player-Aware]] — TT hash missing root_player; latent bug for persistence/MCTS
- [[Issue-TT-Fresh-Per-Search]] — TT discarded between moves; easy perf win
- [[Issue-Hanging-Piece-Eval-Double-Count]] — eval-side hanging penalty double-counts search; reverted and documented

**Not resolved this session:**
- [[Issue-Vec-Clone-Cost-Pre-MCTS]] — still open, still required before Stage 10
