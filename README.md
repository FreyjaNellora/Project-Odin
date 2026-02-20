# Project Odin

A four-player chess engine for chess.com's 4PC format: 14x14 board (160 playable squares), four players, FFA/DKW/Terrain/Chess960 modes. Built in Rust with a TypeScript UI.

---

## Architecture

```
Position -> NNUE eval -> BRS/Paranoid filters losing moves -> Surviving moves -> MCTS evaluates strategy -> Best move
```

| Layer | Role |
|-------|------|
| **NNUE** | Fast neural evaluation (~1us incremental). Replaces handcrafted eval. |
| **BRS/Paranoid Hybrid** | Tactical search (depth 6-12). Finds captures, forks, mates. Alpha-beta. |
| **MCTS** | Strategic search. Long-term planning, alliance dynamics, king safety. |

The hybrid addresses what pure alpha-beta and pure MCTS each get wrong in 4-player chess: alpha-beta can't see deep strategy, MCTS can miss shallow tactics.

---

## Technology

| Component | Stack |
|-----------|-------|
| Engine | Rust |
| UI | TypeScript + React |
| NNUE Training | Python + PyTorch |
| Communication | Odin Protocol (UCI-like, stdin/stdout) |
| Telemetry | Huginn (compile-gated, zero-cost when off) |

---

## Project Status

See `masterplan/STATUS.md` for current stage and progress.

**Current state:** Pre-implementation (planning and documentation phase).

---

## Build Plan

20 stages in 6 tiers:

| Tier | Stages | What It Achieves |
|------|--------|-----------------|
| 1. Foundation | 0-5 | Board, moves, rules, protocol, basic UI |
| 2. Simple Search | 6-7 | Handcrafted eval, plain BRS. Engine plays chess. |
| 3. Strengthen Search | 8-11 | Hybrid scoring, TT, MCTS, integrated search |
| 4. Measurement | 12-13 | Self-play framework, time management |
| 5. Learn | 14-16 | NNUE design, training, integration |
| 6. Polish | 17-19 | Variant tuning, full UI, optimization |

Each stage produces a testable, runnable artifact. The engine is playable (weakly) after Stage 7.

---

## Documents

| File | What It Contains |
|------|-----------------|
| `masterplan/MASTERPLAN.md` | Full technical specification. 20 stages with deliverables, build order, acceptance criteria. |
| `masterplan/AGENT_CONDUCT.md` | How AI agents work: behavior rules, 26-category audit checklist, Huginn reporting spec. |
| `masterplan/4PC_RULES_REFERENCE.md` | Complete 4-player chess rules (board, pieces, scoring, modes). |
| `masterplan/DECISIONS.md` | Architectural decision records with reasoning. |
| `masterplan/STATUS.md` | Current stage, progress tracker, what to do next. |
| `masterplan/HANDOFF.md` | Mid-session continuity notes for context recovery. |
| `masterplan/stage_XX_*.md` | Per-stage detailed specifications. |
| `masterplan/audit_log_stage_XX.md` | Pre/post audit findings per stage. |
| `masterplan/downstream_log_stage_XX.md` | API contracts, limitations, baselines for downstream stages. |

---

## Key Design Decisions

- **Array-first board** with clean abstraction boundary (attack query API). Bitboards deferred to Stage 19 behind the same API.
- **BRS/Paranoid Hybrid** reply selection blending "objectively strongest" with "most harmful + likely."
- **Dual-head NNUE** (BRS scalar + MCTS 4-vector) behind an `Evaluator` trait.
- **Searcher trait** defined early (Stage 7). BRS and MCTS both implement it. Hybrid controller composes them.
- **Self-play at Stage 12** (not end of project). Can't improve what you can't measure.
- **Huginn from Stage 0.** You need the tracer while building, not after.

Full reasoning: see `masterplan/DECISIONS.md`.

---

## Quick Glossary

| Term | Meaning |
|------|---------|
| **BRS** | Best Reply Search. One opponent reply per ply. Alpha-beta compatible. |
| **MCTS** | Monte Carlo Tree Search. Statistics-based exploration. |
| **NNUE** | Efficiently Updatable Neural Network. Fast eval with incremental updates. |
| **Huginn** | Odin's telemetry. Compile-gated ghost observer. Zero cost when off. |
| **FFA** | Free-For-All. Every player for themselves. |
| **DKW** | Dead King Walking. Eliminated king makes random moves. |
| **Terrain** | Eliminated pieces become permanent walls. |
| **Odin Protocol** | UCI-like text protocol extended for 4 players. |

Full glossary: see `masterplan/MASTERPLAN.md` Section 7.
