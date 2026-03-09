# Project Odin

A four-player chess engine for 14x14 boards (160 playable squares, R/B/Y/G). NNUE evaluation with AVX2 SIMD, BRS/Paranoid hybrid search, Gumbel MCTS strategic planning, and a React/Tauri desktop UI.

--- built with Claude. I don't care to nitpick about its silly naming at times. I got other things to worry about :)

---

## What It Does

- Plays four-player chess across multiple game modes: FFA, Last King Standing, Dead Kings Walking, Chess960, and Terrain variants
- Two-phase hybrid search: BRS tactical filter (depth 8, alpha-beta) into Gumbel MCTS strategic exploration
- NNUE neural network evaluation with SIMD-accelerated inference (~800ns incremental, 40x faster than v1)
- Multi-perspective opponent modeling — dynamic blend of paranoid, objective, and anti-leader scoring
- Adaptive time management with position classification (tactical/quiet/endgame/forced)
- Self-play infrastructure: match manager, Elo calculation, SPRT early stopping
- Desktop UI with per-slot player config, self-play dashboard, engine internals panel

---

## Performance

All measurements on release build with LTO.

| Metric | Value |
|--------|-------|
| NNUE incremental eval | 798 ns |
| NNUE forward pass | 1.37 us (40.8x improvement via AVX2 SIMD) |
| BRS depth 6 | 25.3 ms / ~400K NPS |
| BRS depth 8 | ~120 ms |
| MCTS 1000 simulations | 124.9 ms |
| Make/unmake move | 52.7 ns |
| Legal move generation | 4.5 us |
| Tests | 600 engine (573 unit/integration + 27 fuzz) + 63 UI |

---

## Architecture

```
Position -> BRS Phase 1 (tactical filter, depth 8) -> Survivor moves -> Gumbel MCTS Phase 2 (strategic search) -> Best move
```

| Layer | Role |
|-------|------|
| **NNUE** | HalfKP-4 feature set (4,480 features per perspective). Dual-head: BRS scalar centipawns + MCTS 4-player sigmoid. Quantized int16/int8 inference with AVX2 SIMD and scalar fallback. Incremental accumulator updates on make/unmake. |
| **BRS/Paranoid Hybrid** | Tactical search with alpha-beta pruning. Transposition table, killer/history/countermove heuristics, progressive narrowing. Multi-perspective opponent modeling with dynamic blend weights. Quiescence search extends captures past horizon. |
| **Gumbel MCTS** | Strategic search with Gumbel-Top-k and Sequential Halving at root. PUCT tree policy, 4-player MaxN backpropagation, progressive widening. BRS-informed priors via softmax over survivor scores. |
| **HybridController** | Two-phase orchestrator. BRS gets 10-30% of time budget (adaptive by position type), MCTS gets the rest. BRS history table and root scores warm-start MCTS priors. |

**Why hybrid?** Pure alpha-beta can't see deep strategy in a game where three opponents move between your turns. Pure MCTS misses shallow tactics — it needs thousands of simulations to "discover" that a queen is hanging. The hybrid gives BRS the first pass to eliminate tactical blunders, then lets MCTS explore the surviving candidates for strategic depth.

---

## Technology

| Component | Stack |
|-----------|-------|
| Engine | Rust (arrayvec only external dep in hot path) |
| NNUE Inference | AVX2 SIMD with runtime detection + scalar fallback |
| UI | TypeScript + React (Tauri desktop app) |
| NNUE Training | Python + PyTorch (Kaggle GPU pipeline) |
| Communication | Odin Protocol (UCI-like, extended for 4 players) |
| Observer | Node.js — self-play, match management, Elo/SPRT |

---

## Getting Started

When you clone this repo, you get source code (~10 MB). The engine must be compiled from source — the 600+ MB release binary and ~3 GB of build artifacts are not included in the repository.

### Prerequisites

| Tool | Version | What For |
|------|---------|----------|
| **Rust** | 1.75+ (stable) | Engine compilation. Install via [rustup.rs](https://rustup.rs) |
| **Node.js** | 18+ | UI frontend + observer/self-play tools |
| **npm** | (comes with Node) | Package management for UI |
| **Python 3** | 3.9+ | NNUE training pipeline only (optional) |
| **PyTorch** | 2.0+ | NNUE training only (optional, GPU recommended) |

### Build the Engine

```bash
# Clone the repo
git clone https://github.com/FreyjaNellora/Project-Odin.git
cd Project-Odin

# Build the engine (release mode with LTO — takes 2-5 minutes on first build)
cargo build --release

# The binary will be at: target/release/odin-engine (or odin-engine.exe on Windows)
```

The first build downloads dependencies and compiles with full LTO optimization. Subsequent builds are faster.

### Build the UI (Desktop App)

```bash
# Install UI dependencies
cd odin-ui && npm install

# Run in development mode (opens desktop window)
cargo tauri dev

# Or build a standalone installer
cargo tauri build
```

The Tauri app bundles the engine binary and React frontend into a single desktop application.

### Run Tests

```bash
# Engine tests (600+ tests)
cargo test --workspace

# UI tests
cd odin-ui && npm test

# Benchmarks
cargo bench -p odin-engine
```

### NNUE Weights (Optional)

The engine works without NNUE weights — it falls back to a built-in bootstrap evaluator (piece-square tables). For stronger play, you can train NNUE weights:

1. Generate training data via self-play: `cd observer && node match.mjs datagen_gen0_config.json`
2. Convert to binary: `cargo run --release -- --datagen --input <jsonl> --output <bin>`
3. Train with PyTorch: `cd odin-nnue && python train.py` (GPU recommended, or use `kaggle_train.ipynb` on Kaggle)
4. Load weights: set `nnue_file` option via the Odin Protocol or UI settings

`.onnue` weight files are gitignored (9+ MB each). They are generated locally through training.

### What’s NOT in the Repo

These are generated locally and excluded via `.gitignore`:

| Item | Size | How to Get It |
|------|------|---------------|
| `target/` (build artifacts) | ~3 GB | `cargo build --release` |
| `*.onnue` (NNUE weights) | ~9 MB each | Train via self-play pipeline |
| `*.jsonl` (training data) | 10-100 MB | Generate via datagen |
| `node_modules/` | ~200 MB | `npm install` in odin-ui/ |
| `observer/reports/` | varies | Generated by self-play matches |

The engine binary communicates via stdin/stdout using the Odin Protocol. Point any compatible frontend at it, or use the built-in Tauri UI.

---

## Project Status

**All 20 stages complete.** Currently in generational NNUE training (gen-1 in progress).

| Tier | Stages | What It Achieves | Status |
|------|--------|-----------------|--------|
| 1. Foundation | 0-5 | Board, moves, rules, protocol, basic UI | Complete |
| 2. Simple Search | 6-7 | Bootstrap eval, plain BRS with quiescence | Complete |
| 3. Strengthen Search | 8-11 | Hybrid scoring, TT + move ordering, Gumbel MCTS, two-phase integration | Complete |
| 4. Measurement | 12-13 | Self-play framework, regression tests, time management | Complete |
| 5. Learn | 14-16 | NNUE design, training pipeline, search integration | Complete |
| 6. Polish | 17-20 | Variant tuning, full UI, optimization + hardening, gen-0 NNUE training | Complete |

600 engine tests + 63 UI tests passing. Each stage produces a testable, runnable artifact. The engine has been playable since Stage 7.

---

## Game Modes

**Base modes:**

| Mode | Description |
|------|-------------|
| **FFA** (Free-For-All) | Score points by capturing pieces and checkmating opponents. Standard 4PC format. |
| **LKS** (Last King Standing) | Survive — last king alive wins. |

**Modifiers** (pair with either base mode):

| Modifier | Description |
|----------|-------------|
| **DKW** (Dead Kings Walking) | Eliminated players' pieces lock in place on the board. |
| **Terrain** | Eliminated players' pieces freeze where they stand. |
| **Chess960** | Randomized back rank with 4-player symmetric mirroring. |

---

## Key Design Decisions

- **Two-phase hybrid search**: BRS filters tactically, then Gumbel MCTS evaluates survivors strategically. BRS knowledge (history table, root scores) hands off as warm-start priors.
- **Gumbel-Top-k + Sequential Halving**: At the MCTS root, Gumbel noise drives exploration, Sequential Halving efficiently narrows candidates.
- **Multi-perspective opponent modeling**: BRS opponent nodes use a dynamic blend of paranoid, objective, and anti-leader scoring. Weights adapt to game state (material gaps, vulnerability, targeting).
- **Array-first board**: 14x14 with clean abstraction boundary (attack query API). Bitboard retrofit was evaluated in Stage 19 and skipped — profiling showed board scanning is not the bottleneck after SIMD + memory optimization.
- **Dual-head NNUE**: Single network, two output heads — BRS gets scalar centipawns, MCTS gets 4-player win probabilities. Both share the same accumulator stack.
- **Searcher trait frozen early** (Stage 7). BRS and MCTS both implement it. HybridController composes them. No trait changes across 12 stages of search development.
- **Self-play at Stage 12** (not end of project). Can't improve what you can't measure.

Full reasoning: see `masterplan/DECISIONS.md`.

---

## Documents

| File | What It Contains |
|------|-----------------|
| `masterplan/MASTERPLAN.md` | Full technical specification — 20 stages with deliverables and acceptance criteria. |
| `masterplan/AGENT_CONDUCT.md` | How AI agents work on this project: behavior rules, audit checklist, session protocols. |
| `masterplan/4PC_RULES_REFERENCE.md` | Complete 4-player chess rules (board geometry, scoring, elimination, modes). |
| `masterplan/DECISIONS.md` | Architectural decision records with reasoning. |
| `masterplan/STATUS.md` | Current stage, progress tracker, performance baselines. |
| `masterplan/HANDOFF.md` | Session continuity notes for context recovery across AI agent sessions. |
| `observer/match.mjs` | Match manager with seat rotation, Elo + SPRT, per-game JSON logging. |

---

## Glossary

| Term | Meaning |
|------|---------|
| **BRS** | Best Reply Search. Compresses 4-player tree into 2-player MAX/MIN alternation. One opponent reply per ply. Alpha-beta compatible. |
| **Gumbel MCTS** | Monte Carlo Tree Search with Gumbel noise for exploration at root. Sequential Halving allocates simulation budget efficiently. |
| **NNUE** | Efficiently Updatable Neural Network. Sub-microsecond incremental eval on make/unmake via accumulator stack. |
| **HalfKP-4** | NNUE feature set. 4,480 features per perspective (king square x piece square, four player perspectives). |
| **FFA** | Free-For-All. Standard 4PC scoring format. |
| **LKS** | Last King Standing. Elimination mode. |
| **DKW** | Dead Kings Walking. Modifier that locks eliminated players' pieces in place. |
| **Odin Protocol** | UCI-like text protocol extended for 4 players, game modes, eval profiles, and engine internals. |
| **HybridController** | Two-phase orchestrator: BRS Phase 1 (tactical) into MCTS Phase 2 (strategic). |
