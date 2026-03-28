# Research: Project Odin Engine Parts & Architecture

**Date:** 2026-03-28
**Purpose:** Comprehensive inventory of Odin's engine components, their roles, current state, and the design rationale behind each part.

---

## 1. High-Level Architecture

Odin is a four-player chess engine with a three-layer hybrid search:

```
Position -> NNUE Eval -> BRS/Paranoid Prunes -> MCTS Selects -> Best Move
```

| Layer | Role | Latency |
|-------|------|---------|
| **NNUE** | Fast position scoring (dual-head: BRS scalar + MCTS 4-vec) | ~1.37 us/eval |
| **BRS/Paranoid Hybrid** | Tactical depth search (captures, forks, pins, mates) | ~25 ms @ depth 6 |
| **MCTS (Gumbel)** | Strategic exploration beyond BRS horizon | ~125 ms / 1000 sims |

**Data flow:** Board state -> NNUE evaluator scores leaf nodes -> BRS searches tactically with alpha-beta pruning -> surviving candidate moves fed to MCTS -> MCTS explores strategically -> hybrid controller picks the best move.

---

## 2. Component Inventory

### 2.1 Board Representation (Stage 1)

**Files:** `odin-engine/src/board/`
**Key structures:** `Board` (196-element array, 160 valid squares), `Piece`, `Color`, `Square`

- 14x14 cross-shaped board with 36 invalid corner squares
- Per-player piece lists + king square tracking for fast lookup
- Zobrist hashing (4,480 piece-square entries + castling + EP + side-to-move)
- FEN4 parser/serializer for position I/O

**Design decision (ADR-001):** Array-first with clean abstraction. Nothing above Stage 2 reads `board.squares[]` directly -- all access goes through the attack query API. Bitboards deferred unless profiling demands it.

---

### 2.2 Move Generation & Attack Queries (Stage 2)

**Files:** `odin-engine/src/movegen/`
**Key files:** `generate.rs` (24 KB), `moves.rs` (27 KB), `attacks.rs` (14 KB), `tables.rs` (13 KB)

- Pre-computed ray tables (8 directions), knight/king LUTs for all 160 squares
- `is_square_attacked_by(sq, player, board)` -- the foundational query used everywhere
- Pseudo-legal generation per piece type (4 directional pawn sets)
- Legal filtering via make-move + king-attack-check + unmake
- Compact u32 move encoding (from, to, piece, captured, promotion, flags)
- Make/unmake with undo struct (Zobrist hash integrity guaranteed)
- Perft validation at depths 1-4 for CI

---

### 2.3 Game State & Rules (Stage 3)

**Files:** `odin-engine/src/gamestate/`
**Key files:** `mod.rs`, `rules.rs`, `scoring.rs`

- Check, checkmate, stalemate detection (using attack queries from Stage 2)
- Player elimination logic (checkmate = eliminated, pieces become terrain or removed)
- FFA point scoring: captures earn points, checkmate bonuses
- Turn management: R->B->Y->G clockwise rotation, skip eliminated players
- Game termination: last player standing or all opponents eliminated

---

### 2.4 Odin Protocol (Stage 4)

**Files:** `odin-engine/src/protocol/`
**Key files:** `parser.rs`, `emitter.rs`, `types.rs`

UCI-inspired text protocol over stdin/stdout, extended for 4-player chess:
- Commands: `position`, `go`, `setoption`, `quit`, `isready`, `newgame`
- Responses: `bestmove`, `info` (depth, score, nodes, NPS, PV), `readyok`
- Supports FEN4 position strings and 4-player move notation

---

### 2.5 Evaluation System (Stages 6, 14-16)

**Files:** `odin-engine/src/eval/`

#### Bootstrap Evaluator (Stage 6 -- temporary scaffolding)
- Material counting (`material.rs`)
- Piece-square tables (`pst.rs`, 18 KB -- largest eval file)
- King safety (`king_safety.rs`, 11 KB)
- Pawn structure (`pawn_structure.rs`)
- Development scoring (`development.rs`)
- Multi-player adjustments (`multi_player.rs`)
- FFA strategy (`ffa_strategy.rs`), DKW mode (`dkw.rs`), Terrain mode (`terrain.rs`)

#### NNUE Evaluator (Stages 14-16)
**Files:** `odin-engine/src/eval/nnue/`

- **HalfKP-4 features** (`features.rs`): 160 squares x 7 types x 4 relative owners = 4,480 features per perspective, ~30 active per position
- **4 accumulators** (`accumulator.rs`): one per player perspective, incrementally updated on make/unmake
- **Dual output heads**: BRS scalar (centipawn) + MCTS 4-vec (softmax win probabilities)
- **Quantized weights** (`weights.rs`): int16 + int8 for SIMD inference
- **AVX2 SIMD kernel** (`simd.rs`, 13 KB): with scalar fallback
- **SCReLU activation** throughout

**Design decision (ADR-003):** Dual-head NNUE shares feature transformer and hidden layers. Both BRS and MCTS call through the `Evaluator` trait without knowing the implementation.

**Design decision (ADR-004):** HalfKP-4 chosen as minimal 4-player extension of Stockfish's proven approach. King bucketing (20 buckets x 4,480 = 89,600 features) deferred to Phase 2.

**Current state:** Gen-0 weights trained (40K samples, depth 4 self-play). BRS head saturates at +/-30000 -- expected, resolves with gen-1+ training.

---

### 2.6 Search System (Stages 7-11)

**Files:** `odin-engine/src/search/`

#### BRS/Paranoid Hybrid (Stages 7-8)
**File:** `brs.rs`

- Best-Reply Search: one opponent reply per ply (alpha-beta compatible)
- Natural 4-player turn order R->B->Y->G (ADR-012)
- Iterative deepening with aspiration windows
- Quiescence search for tactical stability
- Board scanner (`board_scanner.rs`): pre-search threat detection, < 1ms, feeds reply scoring

**Hybrid scoring (ADR-002):**
```
score = harm_to_root * likelihood + objective_strength * (1 - likelihood)
```
Opponents are somewhat paranoid toward the leader but also pursue their own goals.

#### Transposition Table & Move Ordering (Stage 9)
**File:** `tt.rs`

- Hash table with replacement scheme
- Killer moves, history heuristic, countermove heuristic
- MVV-LVA capture ordering

#### MCTS -- Gumbel (Stage 10)
**File:** `mcts.rs`

- **Gumbel-Top-k** at root with Sequential Halving (ADR-016)
- Works with as few as 2 simulations (critical for tight Phase 2 budgets)
- Non-root: improved policy with prior + progressive history terms
- 4-player MaxN backpropagation
- Pre-NNUE prior from move ordering scores: `pi(a) = softmax(ordering_score(a) / T)`

**Design decision (ADR-016):** Gumbel over UCB1 because the engine only plays one move (simple regret, not cumulative regret). UCB1 needs 16+ sims to converge; Gumbel works with 2.

#### Hybrid Integration (Stage 11)
**File:** `hybrid.rs`

Two-phase orchestrator:
1. **Phase 1 (BRS):** Tactical search, produces candidate moves + history table
2. **Phase 2 (MCTS):** Strategic exploration of BRS survivors using residual time budget

**Progressive History (ADR-017):** BRS history table shared with MCTS. `PH(a) = H(a) / (N(a) + 1)` -- dominates early (warm start), fades as MCTS accumulates data.

---

### 2.7 Self-Play & Measurement (Stages 12-13)

#### Self-Play Framework (Stage 12)
**Files:** `odin-engine/src/datagen.rs`, `observer/match.mjs`

- Match orchestration with seat rotation
- Elo calculation + SPRT (Sequential Probability Ratio Test)
- Data generation for NNUE training (JSONL -> binary conversion)
- Gen-0 config: 1000 games @ depth 4, 40,243 training samples

#### Time Management (Stage 13)
**File:** `search/time_manager.rs`

- Adaptive budgeting by position type
- Phase 1/Phase 2 time split for hybrid search

---

### 2.8 Game Mode Variants (Stage 17)

**Files:** `odin-engine/src/variants/`

- **FFA** (Free For All): Standard 4-player, points from captures
- **DKW** (Dead Kings Walking): Eliminated players' pieces remain active
- **Last King Standing**: Last surviving king wins
- **Terrain Mode**: Eliminated pieces become board obstacles
- **Chess960**: Random back-rank positions

---

### 2.9 UI Layer (Stages 5, 18)

**Files:** `odin-ui/`
**Stack:** TypeScript + React 19.2 + Tauri 2.10

Three-column desktop layout (ADR-014):
- **Left:** Controls (mode, depth, delay, game actions)
- **Center:** 14x14 zoomable board with color-coded pieces
- **Right:** Scores (2x2 grid), Analysis panel, Search Trace, Game Log
- **Bottom:** Collapsible debug strip

Key components: `BoardDisplay.tsx`, `GameControls.tsx` (9.6 KB), `EngineInternals.tsx`, `SelfPlayDashboard.tsx`

63 Vitest tests passing.

---

### 2.10 NNUE Training Pipeline

**Files:** `odin-nnue/`
**Stack:** Python + PyTorch

- `model.py` -- PyTorch model definition (mirrors Rust inference architecture)
- `dataset.py` -- Binary training data loader (556 bytes/sample)
- `train.py` -- Local training script
- `kaggle_train.ipynb` -- GPU training on Kaggle (T4 x2)
- `export.py` -- Convert PyTorch weights to `.onnue` format for Rust

**Current state:** Gen-0 complete. Gen-1 requires self-play with NNUE weights to generate better training data.

---

## 3. Cross-Cutting Concerns

### Score-Aware Engine (ADR-013)

FFA point scoring threads through multiple components:
| Component | Score Awareness |
|-----------|----------------|
| Board Scanner (Stage 8) | Point standings, lead/deficit, capture density |
| MCTS Terminal Eval (Stage 10) | Actual game points in playout scoring |
| Hybrid Allocation (Stage 11) | Behind = more MCTS; ahead = lean on BRS |
| Self-Play Metrics (Stage 12) | Point differential alongside win rate |
| NNUE Features (Stage 16) | Banked points, differential, game phase |

### Observability

`tracing` crate (ADR-015) replaced custom Huginn system. Key instrumentation points at board mutations, search decisions, and eval boundaries. Zero-cost in production (no subscriber attached).

---

## 4. Performance Baselines (Post-Stage 19 Optimization)

| Metric | Value | Notes |
|--------|-------|-------|
| NNUE forward pass | 1.37 us | 40.8x improvement via AVX2 SIMD |
| NNUE incremental push | 798 ns | Per make/unmake |
| NNUE full init | 3.78 us | Cold start per position |
| BRS depth 4 | 3.18 ms | |
| BRS depth 6 | 25.3 ms | ~400K NPS implied |
| MCTS 1000 sims | 124.9 ms | |

---

## 5. Current State & Next Steps

**All 20 stages complete.** 600 engine tests + 63 UI tests passing. Gen-0 NNUE weights functional but crude.

### Immediate priorities:
1. **Gen-1 training cycle** -- Self-play with NNUE weights, generate better data, retrain
2. **Bootstrap eval removal** -- Make NNUE mandatory once gen-1 weights are viable
3. **README update** -- Currently stale (says Stage 19, wrong test counts)

### Deferred work:
- EP rule corner case (ep_sq cleared too eagerly)
- TT EP flag compression edge case
- Pondering support
- NPS stretch goals (1M NPS, 10K sims/sec -- requires tree parallelism)
- King bucketing for NNUE (Phase 2 feature expansion)

---

## 6. Key Architectural Decisions Summary

| ADR | Decision | Rationale |
|-----|----------|-----------|
| 001 | Array-first board (no bitboards) | Simpler, swappable via attack query API |
| 002 | BRS/Paranoid hybrid (not pure MaxN) | Captures FFA dynamics: opponents target leader but have own goals |
| 003 | Dual-head NNUE + Evaluator trait | Shared computation, clean swap from bootstrap |
| 004 | HalfKP-4 features | Proven Stockfish approach, minimal 4-player extension |
| 012 | Natural turn order in BRS | Avoids unmake_move corruption from manual side-to-move changes |
| 013 | Score-aware engine | Points are the win condition in FFA, not just material |
| 015 | tracing crate over custom Huginn | Works out of box, zero-cost when filtered |
| 016 | Gumbel MCTS over UCB1 | Better with limited sims (simple regret optimization) |
| 017 | Progressive History (BRS -> MCTS) | Warm start for MCTS using BRS knowledge |

---

*This document is a research snapshot. For authoritative stage specs, see `masterplan/MASTERPLAN.md`. For live project state, see `masterplan/STATUS.md`.*
