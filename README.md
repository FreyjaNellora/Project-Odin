# Project Odin

A four-player chess engine for chess.com's 4PC format: 14x14 board (160 playable squares), four players (R/B/Y/G), FFA and Last King Standing modes. Built in Rust with a React/Tauri desktop UI.

Inspired by Stockfish's NNUE architecture, DeepMind's Gumbel MuZero planning approach, and BRS multi-player game theory — adapted for the unique dynamics of four-player chess where alliances shift, three opponents move between your turns, and the board is twice the size of standard chess.

--- built using Claude and I don't care to nitpick about its silly naming at times. I got other things to worry about :)

## Architecture

The engine uses a two-phase hybrid search that combines tactical precision with strategic exploration:

```
Position -> BRS Phase 1 (tactical filter, depth 8) -> Survivor moves -> Gumbel MCTS Phase 2 (strategic search) -> Best move
```

| Layer | Role | Status |
|-------|------|--------|
| **Bootstrap Eval** | Handcrafted evaluation: material, PSTs, king safety, multi-player threat modeling. Placeholder for NNUE. | Active |
| **BRS/Paranoid Hybrid** | Tactical search (depth 8, alpha-beta). Finds captures, forks, mates. Multi-perspective opponent modeling with dynamic blend weights. Quiescence search extends captures past horizon. | Active |
| **Gumbel MCTS** | Strategic search. Gumbel-Top-k with Sequential Halving at root, PUCT tree policy, 4-player MaxN backpropagation, progressive widening. BRS-informed priors via softmax over survivor scores. | Active |
| **NNUE** | Efficiently updatable neural network (~1us incremental eval). Replaces bootstrap eval. | Planned (Stages 14-16) |

**Why hybrid?** Pure alpha-beta can't see deep strategy in a game where three opponents move between your turns. Pure MCTS misses shallow tactics — it needs thousands of simulations to "discover" that a queen is hanging. The hybrid gives BRS the first pass to eliminate tactical blunders, then lets MCTS explore the surviving candidates for strategic depth.

---

## Technology

| Component | Stack |
|-----------|-------|
| Engine | Rust (zero dependencies beyond std) |
| UI | TypeScript + React (Tauri desktop app) |
| NNUE Training | Python + PyTorch (planned) |
| Communication | Odin Protocol (UCI-like, stdin/stdout, extended for 4 players) |
| Observer | Node.js self-play and analysis tooling |

---

## Project Status

See `masterplan/STATUS.md` for detailed progress.

**Current state:** Stage 12 complete (`v1.12`). The engine plays four-player chess with a BRS->Gumbel MCTS hybrid search, transposition tables, move ordering (TT/killers/history/countermoves), multi-perspective opponent modeling, adaptive time allocation, and a functional Tauri UI. Self-play infrastructure with match manager, Elo calculation, SPRT early stopping, and 9 regression test positions. 465 tests passing.

**Next:** Stage 13 (Time Management) — smart clock allocation, position complexity detection, parameter tuning.

---

## Build Plan

19 stages in 6 tiers:

| Tier | Stages | What It Achieves | Status |
|------|--------|-----------------|--------|
| 1. Foundation | 0-5 | Board, moves, rules, protocol, basic UI | Complete |
| 2. Simple Search | 6-7 | Bootstrap eval, plain BRS with quiescence. Engine plays chess. | Complete |
| 3. Strengthen Search | 8-11 | Hybrid scoring, TT + move ordering, Gumbel MCTS, integrated two-phase search | Complete |
| 4. Measurement | 12-13 | Self-play framework, regression tests, time management | In Progress |
| 5. Learn | 14-16 | NNUE design, training pipeline, integration | Planned |
| 6. Polish | 17-19 | Variant tuning, full UI, optimization | Planned |

Each stage produces a testable, runnable artifact. The engine has been playable since Stage 7.

---

## Documents

| File | What It Contains |
|------|-----------------|
| `masterplan/MASTERPLAN.md` | Full technical specification. 19 stages with deliverables, build order, acceptance criteria. |
| `masterplan/AGENT_CONDUCT.md` | How AI agents work on this project: behavior rules, 26-category audit checklist, session protocols. |
| `masterplan/4PC_RULES_REFERENCE.md` | Complete 4-player chess rules (board geometry, pieces, scoring, elimination, modes). |
| `masterplan/DECISIONS.md` | 17 architectural decision records with reasoning. |
| `masterplan/STATUS.md` | Current stage, progress tracker, performance baselines. |
| `masterplan/HANDOFF.md` | Session continuity notes for context recovery across AI agent sessions. |
| `masterplan/audit_log_stage_XX.md` | Pre/post audit findings per stage. |
| `masterplan/downstream_log_stage_XX.md` | API contracts, limitations, baselines for downstream stages. |
| `observer/baselines/` | Human game baselines from chess.com (1954-3438 Elo) for engine comparison. |
| `observer/match.mjs` | Two-engine match manager with seat rotation, Elo + SPRT, per-game JSON logging. |
| `observer/elo.mjs` + `sprt.mjs` | Elo difference calculation (95% CI) and Sequential Probability Ratio Test. |

---

## Key Design Decisions

- **Two-phase hybrid search**: BRS filters tactically (depth 8, ~10-30% of time budget), then Gumbel MCTS evaluates survivors strategically (remaining budget). BRS knowledge (history table, root scores) is handed off as warm-start priors.
- **Gumbel-Top-k + Sequential Halving**: At the MCTS root, Gumbel noise creates exploration, Sequential Halving efficiently narrows candidates. Avoids the "sample everything equally" problem of vanilla MCTS.
- **Multi-perspective opponent modeling**: BRS opponent nodes use a dynamic blend of paranoid (harm to root), objective (strongest move), and anti-leader (harm to leader) scoring. Weights adapt to game state.
- **Array-first board** with clean abstraction boundary (attack query API). Bitboards deferred to Stage 19 behind the same API.
- **Dual-head NNUE planned** (BRS scalar + MCTS 4-vector) behind a frozen `Evaluator` trait.
- **Searcher trait** defined early (Stage 7). BRS and MCTS both implement it. HybridController composes them.
- **Self-play at Stage 12** (not end of project). Can't improve what you can't measure.

Full reasoning: see `masterplan/DECISIONS.md`.

---

## Quick Glossary

| Term | Meaning |
|------|---------|
| **BRS** | Best Reply Search. Compresses 4-player tree into 2-player MAX/MIN alternation. One opponent reply per ply. Alpha-beta compatible. |
| **Gumbel MCTS** | Monte Carlo Tree Search with Gumbel noise for exploration at root. Uses Sequential Halving to efficiently allocate simulation budget across candidates. |
| **NNUE** | Efficiently Updatable Neural Network. Fast eval with incremental updates on make/unmake. |
| **FFA** | Free-For-All. Score points by capturing pieces and checkmating opponents. |
| **LKS** | Last King Standing. Survive — last king alive wins. |
| **Odin Protocol** | UCI-like text protocol extended for 4 players, game modes, and eval profiles. |
| **HybridController** | The two-phase orchestrator: BRS Phase 1 (tactical filter) -> MCTS Phase 2 (strategic search). |

Full glossary: see `masterplan/MASTERPLAN.md` Section 7.
