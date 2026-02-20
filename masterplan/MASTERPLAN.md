# PROJECT ODIN -- MASTERPLAN
## A Four-Player Chess Engine: NNUE + BRS/Paranoid Hybrid + MCTS

**Version:** 3.0
**Created:** 2026-02-18
**Last Updated:** 2026-02-19
**Status:** Active

---

## 1. VISION

Odin is a four-player chess engine for the game as played on chess.com: 14x14 board (160 playable squares), four players (Red, Blue, Yellow, Green), turn order clockwise. It supports FFA, DKW, Last King Standing, Terrain mode (eliminated pieces persist as obstacles), and Chess960 adaptation.

Full rules: see `4PC_RULES_REFERENCE.md` ([[4PC_RULES_REFERENCE]]).

### Why This Architecture

Two-player chess engines use alpha-beta + NNUE (Stockfish) or pure MCTS + deep NN (Leela). Four-player chess breaks both:

- **Alpha-beta alone:** The game tree branches explosively (4 players x ~30 moves each per round). Even with BRS compression, depth is limited.
- **Pure MCTS alone:** It builds a selective tree that can miss shallow tactical refutations -- critical when three opponents can all create threats.
- **The metagame is different.** Alliances shift. Survival matters more than material. The leader gets targeted. Sacrificing pieces to redirect aggression is valid strategy.

The hybrid:

| Layer | Role | What It Does |
|-------|------|-------------|
| **NNUE** | Fast eval | Scores positions using learned piece-square features. ~1us per incremental eval. |
| **BRS/Paranoid Hybrid** | Tactical search (depth 6-12) | Finds captures, forks, pins, mates. Eliminates blunder moves. Alpha-beta keeps it fast. |
| **MCTS** | Strategic search | Explores positions beyond BRS depth via simulation. 4-player MaxN backpropagation. Selects among BRS survivors. |

Data flow: `Position -> NNUE eval -> BRS/Paranoid prunes losing moves -> Surviving moves -> MCTS evaluates strategy -> Best move`

### Design Principles

1. **Correctness first.** A wrong answer fast is worse than a slow correct answer.
2. **Incremental build.** Each stage produces a testable, runnable artifact. Engine is playable (weakly) after Stage 7.
3. **Observability.** Huginn (the telemetry system) lets us see inside the engine at every step. No black boxes.
4. **No redundancy.** The UI is a display layer. It does not validate moves, check legality, or compute evaluations.
5. **Independent stages.** Each stage can be developed and audited using only its own spec, the audit/downstream logs from prior stages, and the code itself.

---

## 2. ARCHITECTURE

```
+---------------------------------------------------------+
|                     UI LAYER (Stage 5/18)                |
|  Board display, move input, debug console, self-play     |
|  NO game logic. Pure display + input + debug output.     |
+------------------------+--------------------------------+
                         | Odin Protocol (Stage 4)
+------------------------v--------------------------------+
|                   ENGINE CORE                            |
|                                                          |
|  Search Controller (Stage 11)                            |
|    |                      |                              |
|  BRS/Paranoid Hybrid --> MCTS Strategic Search           |
|  (Stage 7-8)             (Stage 10)                      |
|    |                      |                              |
|  NNUE Evaluation (Stage 14-16)                           |
|  [Bootstrap handcrafted eval before NNUE]                |
|    |                                                     |
|  Game State & Rules Engine (Stage 3)                     |
|    |                                                     |
|  Move Generation (Stage 2)                               |
|    |                                                     |
|  Board Representation (Stage 1)                          |
|                                                          |
|  Huginn Telemetry (Stage 0 core, grows per stage)        |
|  EXTERNAL OBSERVER -- zero engine impact, compile-gated  |
|                                                          |
|  Transposition Table & Move Ordering (Stage 9)           |
+---------------------------------------------------------+
```

### Build Plan: 20 Stages in 6 Tiers

```
TIER 1 — FOUNDATION (Stages 0-5)
  0: Skeleton + Huginn Core
  1: Board Representation
  2: Move Generation + Attack Query API
  3: Game State & Rules
  4: Odin Protocol
  5: Basic UI Shell

TIER 2 — SIMPLE SEARCH (Stages 6-7)
  6: Bootstrap Eval + Evaluator Trait
  7: Plain BRS + Searcher Trait

TIER 3 — STRENGTHEN SEARCH (Stages 8-11)
  8: BRS/Paranoid Hybrid Layer
  9: Transposition Table & Move Ordering
 10: MCTS Strategic Search
 11: Hybrid Integration (BRS -> MCTS)

TIER 4 — MEASUREMENT (Stages 12-13)
 12: Self-Play & Regression Testing
 13: Time Management

TIER 5 — LEARN (Stages 14-16)
 14: NNUE Feature Design & Architecture
 15: NNUE Training Pipeline
 16: NNUE Integration

TIER 6 — POLISH (Stages 17-19)
 17: Game Mode Variant Tuning
 18: Full UI
 19: Optimization & Hardening
```

Each stage produces a testable, runnable artifact. The engine is playable (weakly) after Stage 7.

---

## 2.1 HUGINN SPECIFICATION

Huginn (Odin's raven of thought) is the telemetry and tracer system. It exists solely for debugging and development.

### The Iron Rule: Huginn Must Be a Ghost

- Huginn **never writes** to any engine data structure.
- Huginn **never allocates** memory in any engine hot path.
- Huginn **never introduces branches** into engine code paths.
- When off (default for play/competition), the engine compiles as if Huginn **does not exist** -- literally absent from the binary.
- When on (dev builds), it operates as a **post-hoc reader**: snapshots engine state at observation points, copies data to its own external buffer, processes it after `bestmove` is returned.

### The Snitch Model

Huginn is a QC inspector standing at every gate. It doesn't decide what gets through, doesn't slow anyone down, doesn't touch anything. But it witnesses and accounts for everything that passes through its gate. Every time data crosses a boundary -- board to movegen, movegen to search, BRS to MCTS, NNUE input to output -- Huginn is standing there taking notes.

### Implementation

- **Compile flag:** `cfg(feature = "huginn")` in Cargo.toml, default off.
- **Observation macro:** `huginn_observe!(...)` -- compiles to absolutely nothing when off. When on, snapshots state into Huginn's external buffer.
- **Huginn-owned memory:** Ring buffer separate from all engine allocations. Silent drop on overflow.
- **Post-search processing:** Trace correlation and JSON serialization happen after `bestmove` is returned.
- **Verbosity levels:** Minimal, Normal, Verbose, Everything.

### Growth Model

Huginn is born in Stage 0 (macro + buffer). Every subsequent stage adds observation points for what that stage builds. By Stage 19, Huginn has gates at every boundary.

### Reporting Specification

Full Huginn reporting specification (JSONL format, ring buffer, trace hierarchy, verbosity contracts, agent usage guide) is defined in `AGENT_CONDUCT.md` ([[AGENT_CONDUCT]]) Section 3.

---

## 3. TECHNOLOGY

| Component | Technology | Why |
|-----------|-----------|-----|
| Engine | Rust | Performance-critical, memory-safe, zero-cost abstractions. |
| UI | TypeScript + React | WebSocket/subprocess communication with engine. |
| NNUE Training | Python + PyTorch | Industry standard. Weights exported to Rust for inference. |
| Communication | Odin Protocol (stdin/stdout) | Simple, debuggable, UCI-inspired. |
| Testing | cargo test + criterion | Built-in framework + statistical benchmarking. |

---

## 4. STAGE DEFINITIONS

---

### STAGE 0: Project Skeleton + Huginn Core

**Tier:** 1 — Foundation
**Dependencies:** None

**The problem:** You need a project structure, build system, and the telemetry macro available before writing any engine code. Every subsequent stage needs to place observation points from day one.

**What you're building:**

1. **Directory structure.** Rust workspace for the engine (`odin-engine/`), React app for the UI (`odin-ui/`), PyTorch scripts for NNUE training (`odin-nnue/`), masterplan docs (`masterplan/`).

2. **Build system.** Cargo for Rust, npm for UI. `cargo build` and `cargo test` work out of the box. CI config files.

3. **Huginn core.** Three things:
   - The `huginn` feature flag in `Cargo.toml` (default off)
   - The `huginn_observe!` macro that compiles to nothing when off
   - The `HuginnBuffer` ring buffer + `TraceEvent` struct

4. **Proof-of-life test.** Compile with and without `--features huginn`, verify identical behavior and that the feature-off binary contains zero Huginn symbols.

**Build order:**
1. Create directory structure
2. Initialize Cargo workspace + React project
3. Write the Huginn macro and buffer
4. Write the proof-of-life test
5. Set up linting, formatting, CI

**What you DON'T need:**
- Any engine code. This is pure scaffolding.
- Any observation points. Those come from each stage as it builds the thing being observed.

**Directory layout:**
```
Project_Odin/
+-- masterplan/           # This document + stage/audit/downstream files
+-- odin-engine/
|   +-- Cargo.toml
|   +-- src/
|   |   +-- lib.rs, main.rs
|   |   +-- board/        # Stage 1
|   |   +-- movegen/      # Stage 2
|   |   +-- gamestate/    # Stage 3
|   |   +-- protocol/     # Stage 4
|   |   +-- eval/         # Stage 6, 14-16
|   |   +-- search/       # Stage 7, 8, 9, 10, 11
|   |   +-- huginn/       # Stage 0 core, grows per stage
|   |   +-- variants/     # Stage 17
|   +-- tests/
+-- odin-ui/
|   +-- package.json
|   +-- src/
+-- odin-nnue/            # Stage 14-15
+-- tools/
```

**Huginn macro pattern:**
```rust
#[cfg(not(feature = "huginn"))]
macro_rules! huginn_observe {
    ($($args:tt)*) => {};
}
```

**Acceptance criteria:**
- `cargo build` succeeds, `cargo build --features huginn` succeeds
- `huginn_observe!` compiles to nothing without the feature (verified by binary inspection)
- UI initializes with `npm install && npm run dev`

---

### STAGE 1: Board Representation

**Tier:** 1 — Foundation
**Dependencies:** Stage 0 ([[stage_00_skeleton]])

**The problem:** You need to represent the 14x14 board (160 valid squares, 36 corner squares removed), pieces, players, and provide fast lookup, placement, and position hashing. Everything downstream depends on this being right.

**What you're building:**

1. **Square indexing.** `index = rank * 14 + file` (196 total, 36 invalid). A validity lookup table marks corner squares.

2. **Piece representation.** Each piece has a type (Pawn, Knight, Bishop, Rook, Queen, King, PromotedQueen), an owner (Red, Blue, Yellow, Green), and a status (Alive, Dead, Terrain).

3. **Board storage.** A 196-element array of `Option<Piece>`, plus per-player piece lists for fast iteration, plus per-player king square tracking.

4. **Zobrist hashing.** Random u64 per (square, piece_type, owner) combination (4,480 entries), plus castling (256 entries for 8-bit key), en passant (14 files), side to move (4 players). Fixed seed for reproducibility.

5. **FEN4 parser/serializer.** Position I/O for testing and the protocol.

6. **Make/unmake stubs.** The infrastructure for applying and reverting moves (Stage 2 fills in the logic).

**Build order:**
1. Square indexing + validity table
2. Piece and Player enums
3. Board struct with array + piece lists
4. Zobrist hash generation and accumulation
5. FEN4 parse/serialize
6. Make/unmake infrastructure (empty, awaiting Stage 2)

**What you DON'T need:**
- Move generation (Stage 2)
- Game rules like check or checkmate (Stage 3)
- Bitboards. Start with the array. If profiling later shows it's a bottleneck, add bitboards in Stage 19.

**Key types:**
```rust
struct Board {
    squares: [Option<Piece>; 196],
    piece_lists: [Vec<(PieceType, u8)>; 4],
    king_squares: [u8; 4],
    zobrist: u64,
    castling_rights: u8,     // 8 bits: 2 per player (kingside, queenside)
    en_passant: Option<u8>,
    side_to_move: Player,
    halfmove_clock: u16,
    fullmove_number: u16,
}
```

**Huginn gates (this stage):**
- Board mutation gate: every `set_piece`/`remove_piece` -- what changed, where, what was there before
- Zobrist update gate: every XOR -- old hash, key, new hash (traces hash corruption to the exact op)
- FEN4 gate: input string vs. resulting board state (catches parse/serialize mismatches)
- Piece list sync gate: array vs. piece list after every mutation (catches desync at the moment it happens)

**Acceptance criteria:**
- All 160 valid squares identified, all 36 corners rejected
- FEN4 round-trip: parse starting position -> serialize -> matches original
- Zobrist hash changes on piece placement/removal
- Piece lists stay synchronized with board array

---

### STAGE 2: Move Generation + Attack Query API

**Tier:** 1 — Foundation
**Dependencies:** Stage 1 ([[stage_01_board]])

**The problem:** You need to generate all legal moves for any position. This is the most critical stage -- a bug here poisons everything downstream. Move generation for 4PC is standard chess movement on a weird-shaped board with four sets of pawns going four directions.

**What you're building:**

1. **Pre-computed tables.** For each of the 160 valid squares: ray tables for sliding pieces (8 directions, stopping at edges and corners), knight destination tables, king adjacency tables.

2. **Attack query API.** A standalone `is_square_attacked_by(square, player, board) -> bool` function and its companion `attackers_of(square, player, board) -> Vec<(PieceType, Square)>`. These use the pre-computed tables to check if a given player attacks a given square. This is the foundation that everything downstream reuses: legal move filtering uses it here, check detection uses it in Stage 3, the cheap interaction filter uses it in Stage 8, and castling legality checks use it for "king does not pass through check."

3. **Pseudo-legal move generation.** For each piece type, generate all candidate moves (ignoring check). Pawns go in 4 different directions depending on player. Includes double-step, en passant, promotion (1-pt queen for FFA), and castling.

4. **Legal move filtering.** After generating pseudo-legal moves, apply each one and check if the moving player's king is attacked by ANY of the other 3 players (using the attack query API). Expensive but correct. Optimization (pin detection) can come later.

5. **Move encoding.** Compact u32:
   ```
   bits 0-7: from_square, bits 8-15: to_square, bits 16-19: piece_type,
   bits 20-23: captured_piece, bits 24-27: promotion, bits 28-30: flags
   ```

6. **Make/unmake.** Apply a move (update board, pieces, hash, castling, en passant), return an undo struct. Unmake restores the exact previous state. Zobrist hash must match after make->unmake.

   **Design note:** Make/unmake must expose what changed (which piece moved from where to where, what was captured, was it a king move) clearly enough that downstream consumers can react. Stage 8's delta refresh reads this to update board context. Stage 14's NNUE accumulator updates read this to know which features to add/remove. Don't bury the "what changed" information -- make it part of the return value or easily derivable from the Move + MoveUndo.

7. **Perft.** Recursive move count at given depths. Establish known-correct values for the starting position. Run as part of CI.

**Build order:**
1. Pre-compute attack tables (rays, knight, king)
2. Attack query API (`is_square_attacked_by`, `attackers_of`)
3. Pseudo-legal generation per piece type
4. Pawn movement (all 4 directions, double step, en passant, promotion)
5. Castling (all 4 players, using attack query to check king path)
6. Make/unmake with undo struct
7. Legal filtering (make, use attack query on own king, unmake)
8. Perft validation at depths 1-4

**What you DON'T need:**
- Move ordering (Stage 9). Just generate them in any order.
- Game-level check/checkmate/stalemate determination (Stage 3). You build the attack query infrastructure here; Stage 3 uses it to make game-level rulings.
- Performance optimization. Correctness is the only goal. If perft is slow, that's fine.

**Pawn directions:**
```
Red:    +rank    Blue:   +file    Yellow: -rank    Green:  -file
```

**Huginn gates (this stage):**
- Move generation gate: position hash, player, pseudo-legal count, legal count, full move list
- Make/unmake gate: move, hash before/after, captured piece, special flags, hash restoration check on unmake
- Legality filter gate: each rejected move + why (which opponent attacks the king, from where)
- Perft gate: node count at each depth (mismatches caught with full trace context)

**Acceptance criteria:**
- Perft at depths 1-4 matches independently verified values
- All special moves tested: castling for 4 players, en passant, promotions
- Make -> unmake returns board to identical state (Zobrist matches)
- Stress test: 1000+ random game playouts without crashes

---

### STAGE 3: Game State & Rules

**Tier:** 1 — Foundation
**Dependencies:** Stage 2 ([[stage_02_movegen]])

**The problem:** You have a board and moves. Now you need the full game lifecycle: turns, check/checkmate/stalemate detection, elimination, scoring, DKW, terrain conversion, game-over conditions.

**What you're building:**

1. **Turn management.** 4-player rotation, skipping eliminated players.

2. **Check detection.** For a given king, check if any piece of any other active player attacks it. Uses the `is_square_attacked_by` / `attackers_of` API built in Stage 2.

3. **Checkmate/stalemate detection.** When a player's turn arrives: generate their legal moves. Zero moves + in check = checkmate. Zero moves + not in check = stalemate. Critical: this is only checked when their turn comes, not when the check is delivered.

4. **Elimination and scoring.** On elimination: mark player, award points per the scoring table, handle terrain conversion if terrain mode is on, activate DKW if resignation or timeout (not checkmate or stalemate -- see `4PC_RULES_REFERENCE.md` ([[4PC_RULES_REFERENCE]])), check game-over conditions.

5. **DKW handler.** Dead king makes random legal moves instantly (not as a full turn). Processed after each active player's move.

6. **Terrain mode.** On elimination, convert all remaining pieces (except king) to Terrain status. Terrain pieces block movement, cannot be captured, are inert for check purposes.

7. **Position tracking.** Zobrist history for repetition detection. Half-move clock for 50-move rule.

**Build order:**
1. Turn rotation with elimination skip
2. Check detection using attack lookups
3. Checkmate/stalemate determination
4. Scoring system (all FFA point values)
5. Elimination pipeline
6. DKW random move logic
7. Terrain conversion
8. Game-over detection (three eliminated, claim win, draw conditions)
9. Position repetition tracking

**What you DON'T need:**
- Evaluation or search awareness of these rules (Stage 6-7). This stage is pure rule enforcement.
- UI display of game state (Stage 5). This stage just maintains the state.

**Design note: GameState must be cheaply cloneable.** MCTS (Stage 10) clones the GameState at the start of each simulation to traverse lines without corrupting the root. With thousands of simulations, this must be cheap. Derive `Clone`, keep allocations minimal (fixed-size arrays over Vecs where possible), and consider copy-on-write for the position history Vec.

**Key types:**
```rust
struct GameState {
    board: Board,
    player_status: [PlayerStatus; 4],
    scores: [i32; 4],
    current_turn_index: usize,
    elimination_order: Vec<Player>,
    position_history: Vec<u64>,
    game_mode: GameMode,
    terrain_mode: bool,
    game_over: bool,
    winner: Option<Player>,
}
```

**Huginn gates (this stage):**
- Turn transition gate: previous/next player, skipped players, reason
- Check detection gate: which king tested, attackers found, from where
- Checkmate/stalemate gate: determination, position, attacking pieces
- Elimination gate: reason, points awarded, terrain conversions, DKW activation
- Scoring gate: who earned, how many, from what action, running totals
- DKW gate: king position, move selected, legal move set
- Game-over gate: condition met, final scores, winner

**Acceptance criteria:**
- Correct turn rotation with eliminated player skipping
- Checkmate detected correctly in multi-player scenarios
- Stalemate awards 20 points in FFA
- DKW king makes random moves after resignation
- Terrain pieces block movement correctly
- Scoring matches chess.com rules for all capture types
- Game ends correctly under all termination conditions

---

### STAGE 4: Odin Protocol

**Tier:** 1 — Foundation
**Dependencies:** Stage 3 ([[stage_03_gamestate]])

**The problem:** The engine needs to talk to frontends. You need a text protocol -- UCI-like but extended for 4 players (4 time controls, per-player values, phase indicators).

**What you're building:**

1. **Command parser.** stdin reader that handles: `odin`, `isready`, `setoption`, `position`, `go`, `stop`, `quit`.

2. **Response emitter.** stdout writer for: `id`, `readyok`, `bestmove`, `info` strings.

3. **4PC extensions.** The `go` command takes 4 time controls (wtime/btime/ytime/gtime). The `info` string includes per-player values (v1-v4), search phase (brs/mcts), BRS surviving count, MCTS simulation count.

**Build order:**
1. Command parsing with error handling for malformed input
2. Position setting (FEN4 + move list)
3. `go` stub (returns random legal move)
4. `info` string formatting
5. `bestmove` output
6. Full command set

**What you DON'T need:**
- Actual search (Stage 7). The `go` command returns a random legal move for now.
- WebSocket/subprocess wrapper (Stage 5 handles that from the UI side).

**Protocol reference:**
```
# UI -> Engine
odin                    # Initialize
isready                 # Readiness check
setoption name X value Y
position fen4 <string> moves <movelist>
position startpos moves <movelist>
go wtime <ms> btime <ms> ytime <ms> gtime <ms> [depth N] [nodes N] [movetime N]
stop
quit

# Engine -> UI
id name Odin vX.Y.Z
readyok
bestmove <move> [ponder <move>]
info depth <N> seldepth <N> score cp <N> v1 <N> v2 <N> v3 <N> v4 <N> nodes <N> nps <N> time <ms> pv <moves> phase <brs|mcts> brs_surviving <N> mcts_sims <N>

# Move notation: d2d4, d7d8q (promotion), e1g1 (castling as king move)
```

**Huginn gates (this stage):**
- Command receive gate: raw string, parsed type, parse errors
- Response send gate: full string, what triggered it
- Position set gate: FEN4/startpos, move list, resulting hash
- Search request gate: time controls, depth limits, options

**Acceptance criteria:**
- Engine responds to `odin` with id, `isready` with `readyok`
- Position set via FEN4 or startpos + moves
- `go` returns a legal move
- Malformed input handled gracefully (no crash)

---

### STAGE 5: Basic UI Shell

**Tier:** 1 — Foundation
**Dependencies:** Stage 4 ([[stage_04_protocol]])

**The problem:** You need a minimal UI to see the board, make moves, and read engine debug output. This is scaffolding for the full UI in Stage 18.

**What you're building:**

1. **Board renderer.** SVG or Canvas, 14x14 grid, corners invisible, pieces colored per player. Click-to-move or text input.

2. **Engine communication.** Spawn engine as child process, send/receive via stdin/stdout using Odin Protocol.

3. **Debug console.** Panel showing raw engine output: best move, eval, depth, nodes, NPS, phase, surviving moves count, MCTS sims, timeout reason. Scrollable raw log.

4. **Basic controls.** Current turn, player scores, player status, "New Game" with mode selection.

**Build order:**
1. Board rendering with pieces in starting positions
2. Engine subprocess spawning and protocol communication
3. Click-to-move (send to engine, engine validates, UI updates)
4. Debug console with info string parsing
5. New game / mode selection

**What you DON'T need:**
- Any game logic. The UI sends moves to the engine, the engine validates. The UI never computes legal moves, detects check, or evaluates positions.
- Visual polish (Stage 18). This is functional scaffolding.
- Huginn trace viewer (Stage 18). But this stage does build the display surface -- when `info string huginn ...` arrives, the debug console renders it.

**The rule: UI owns ZERO game logic.** It does not validate moves. It does not compute legal moves. It does not detect check/checkmate. It sends intended moves to the engine, the engine responds with the new position or an error.

**Acceptance criteria:**
- Board displays correctly with all 160 squares and starting pieces
- Can spawn engine and exchange protocol messages
- Can make a move and see the board update
- Debug console shows engine output
- No game logic in UI code

---

### STAGE 6: Bootstrap Evaluation + Evaluator Trait

**Tier:** 2 — Simple Search
**Dependencies:** Stage 3 ([[stage_03_gamestate]])

**The problem:** Search needs an evaluation function to compare positions. NNUE isn't ready until Stage 16. You need a simple handcrafted eval that's good enough for BRS to find captures and avoid blunders. It will be replaced.

**What you're building:**

1. **The `Evaluator` trait.** This is the contract that persists through the entire project. Define it now so every search consumer codes against the trait, not against a specific implementation. When NNUE replaces the bootstrap in Stage 16, nothing above the trait changes.

```rust
trait Evaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16;
    fn eval_4vec(&self, position: &GameState) -> [f64; 4];
}
```

2. **Material counting.** Per player, using 4PC piece values: Pawn=100cp, Knight=300cp, Bishop=500cp, Rook=500cp, Queen=900cp. Promoted queen: moves as queen (900cp for search) but worth only 1 point on capture.

3. **Piece-square tables.** 160-entry position bonus per piece type per player. Center control premium, pawn advancement bonus, knight centralization, bishop on diagonals, rook on open files, king safety.

4. **Multi-player relative evaluation.** Not just "my material minus theirs." Considers: absolute score, army strength, relative standing (being the leader is slightly bad -- attracts aggression), threat level (how many opponents can attack my king), FFA points.

5. **Bootstrap implementation of `Evaluator`.** `BootstrapEvaluator` implements the trait. `eval_scalar` returns centipawn value from one player's perspective. `eval_4vec` returns evaluation from all 4 perspectives, normalized to [0,1] with sigmoid.

**Build order:**
1. Define `Evaluator` trait
2. Material counting with 4PC values
3. Piece-square tables (start simple, refine)
4. King safety heuristic
5. Multi-player relative eval (lead penalty, threat penalty)
6. Integration with FFA scoring system
7. Implement `BootstrapEvaluator` wrapping all components

**What you DON'T need:**
- Neural network anything (Stage 14-16). This is handcrafted arithmetic.
- Pawn structure analysis beyond basics. The bootstrap eval is temporary.
- Perfect accuracy. It just needs to tell search "this position is better/worse." NNUE will learn the subtleties.

**Eval sketch:**
```
eval_for_player(p) =
    material_score(p)
    + positional_score(p)      // piece-square tables
    + king_safety(p)
    - threat_penalty(p)        // per opponent attacking near my king
    + lead_penalty(p)          // being in the lead is slightly bad
    + ffa_points(p) * weight
```

**Huginn gates (this stage):**
- Eval call gate: position hash, player perspective, raw score, component breakdown (material, positional, king safety, threat, lead penalty, FFA points)
- Eval comparison gate: same position evaluated from all 4 perspectives side by side (catches perspective bugs)

**Acceptance criteria:**
- Evaluation returns different values for materially different positions
- Evaluation is perspective-dependent
- Eval is fast (< 10us per position)
- `Evaluator` trait compiles and `BootstrapEvaluator` implements it correctly
- `eval_scalar` and `eval_4vec` produce consistent results for the same position

---

### STAGE 7: Plain BRS + Searcher Trait

**Tier:** 2 — Simple Search
**Dependencies:** Stage 6 ([[stage_06_bootstrap_eval]])

**The problem:** You need a working search that can play legal chess and find basic tactics: captures, forks, mate-in-1. This stage builds standard BRS with alpha-beta -- no hybrid extensions yet. The hybrid layer comes in Stage 8.

**What you're building:**

1. **The `Searcher` trait.** This is the interface that both BRS and MCTS (Stage 10) implement. Define it now so the hybrid controller in Stage 11 composes through the trait without knowing implementation details.

```rust
trait Searcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult;
}

struct SearchResult {
    best_move: Move,
    score: i16,
    depth: u8,
    nodes: u64,
    pv: Vec<Move>,
}
```

2. **Standard BRS search.** Alternating MAX/MIN nodes. At opponent nodes, pick the objectively strongest reply (standard BRS behavior). Alpha-beta pruning. The engine plays legal chess after this step.

3. **Iterative deepening.** Search depth 1, then 2, then 3... Use the PV from the previous depth for move ordering at the root.

4. **Quiescence search.** At leaf nodes, extend with captures until quiet. Stand-pat pruning. Max 8 extra plies.

5. **Aspiration windows.** Start with a narrow window around the previous depth's score. Widen on fail-high/fail-low.

6. **Null move pruning.** Adapted for 4-player: skip the root player's turn, check if the position is still good. If yes, prune this branch.

7. **Late move reductions (basic).** Reduce search depth for moves that are unlikely to be good (late in the move list, not captures, not checks).

8. **PV tracking.** Maintain the principal variation through iterative deepening.

9. **BRS implementation of `Searcher`.** `BrsSearcher` implements the `Searcher` trait.

10. **Info output.** Emit `info` strings with depth, score, nodes, NPS, PV, `phase brs`.

**Build order:**
1. Standard BRS with alpha-beta (MAX/MIN alternation, pick objectively strongest opponent reply)
2. Iterative deepening
3. Quiescence search
4. Aspiration windows
5. Null move pruning
6. Late move reductions
7. PV tracking
8. Define `Searcher` trait, implement `BrsSearcher`
9. Info string output with `phase brs`
10. Wire into Odin Protocol `go` command (replace random move)

**What you DON'T need:**
- The board scanner, cheap filter, hybrid scoring, or progressive narrowing (Stage 8). This is plain BRS only.
- Transposition table (Stage 9). Move ordering is basic for now.
- MCTS (Stage 10). BRS works standalone.
- Move ordering beyond iterative deepening PV. Full ordering comes in Stage 9.

**Huginn gates (this stage):**
- Alpha-beta prune gate: depth, alpha, beta, cutoff score, cutoff move, node type
- Quiescence gate: entry/exit, stand-pat score, captures evaluated
- Iterative deepening gate: depth, best move, score, node count, time, PV
- BRS reply selection gate: opponent, candidates considered, selected move, score

**Acceptance criteria:**
- Engine plays legal moves via the protocol
- Finds mate-in-1
- Avoids hanging pieces (captures are searched)
- Iterative deepening reaches depth 6+ within 5 seconds
- `Searcher` trait compiles and `BrsSearcher` implements it correctly
- Info strings show depth, score, nodes, PV

---

### STAGE 8: BRS/Paranoid Hybrid Layer

**Tier:** 3 — Strengthen Search
**Dependencies:** Stage 7 ([[stage_07_plain_brs]])

**The problem:**

Standard BRS picks the "objectively strongest" opponent reply. Problem: that might be Yellow capturing Green's queen -- great for Yellow, irrelevant to you (Red). Standard Paranoid picks the reply that "hurts you the most." Problem: Blue might sacrifice their queen to checkmate you -- but Blue won't actually do that because it leaves them exposed.

The hybrid says: "Pick the reply that is both dangerous to you AND that the opponent would realistically play given what's happening on the board."

This stage adds the hybrid layer ON TOP of the working BRS from Stage 7. If the hybrid makes things worse, roll back to Stage 7. Nothing is thrown away.

**What you're building:**

1. **A board scanner** that runs once before search. Loops through pieces, checks lines of attack, compares scores, looks at king exposure. Produces a flat struct with ~15 fields. Takes under a millisecond. Not search -- pattern recognition. "Who's pointing guns at me, who's pointing guns at someone else, who's exposed."

2. **A move classifier** at each opponent node. For every candidate move: does it capture one of my pieces, check my king, or land next to my king? Yes = "relevant." No = "background." A handful of table lookups per move. Reuses the attack query API and pre-computed tables from Stage 2.

3. **A scoring function** on only the ~10-15 relevant moves. Three numbers multiplied together: how strong is this move objectively (quick eval), how much does it hurt me specifically, how likely is this opponent to actually play it (lookup into the board scanner's output). One float per move. Sort descending.

4. **A fallback** keeping the single strongest move from the ~75 background moves. Just a running max while the classifier runs. Zero extra cost.

5. **A depth schedule (progressive narrowing).** One array lookup: 8-10 candidates at shallow depth, 3 at deep depth.

6. **A delta updater** patching the board scanner every 2 plies. Make/unmake already tracks captures and king moves. Optional for v1.

**What you DON'T need:**
- No new search structure. The tree is still alternating MAX/MIN. Alpha-beta works unchanged.
- No opponent modeling. You're reading the board and asking "who has guns pointed at me right now."
- No machine learning. Scoring weights are hand-tuned constants. Self-play refines them later (Stage 12).
- No new data structures. Board context is a flat struct. Move classification uses existing attack tables.

**Build order (each step independently testable):**
1. Board scanner. Run before search, print output, verify by hand on test positions.
2. Cheap filter at opponent nodes. Classify moves as relevant/background. Verify classification.
3. Hybrid scoring on relevant moves. Replace "pick objectively strongest" with "pick highest hybrid score." Compare vs. pure BRS in test positions.
4. Progressive narrowing. Measure node count reduction.
5. Delta refresh. Compare against full re-read to verify accuracy.
6. Self-play comparison: hybrid BRS vs. plain BRS from Stage 7.

If the hybrid scoring makes things worse at any step, roll back to plain BRS. Nothing is thrown away.

**Board context struct:**
```
struct BoardContext {
    weakest_player: Player,
    most_dangerous: [Player; 3],
    root_danger_level: f64,
    high_value_targets: Vec<(Square, Player)>,
    convergence: Option<(Player, Player, Player)>,
    per_opponent: [OpponentProfile; 3],
}

struct OpponentProfile {
    player: Player,
    aggression_toward_root: f64,
    own_vulnerability: f64,
    best_target: Player,
    can_afford_to_attack_root: bool,
    supporting_attack_on_root: bool,
}
```

**Hybrid reply scoring:**
```
score = (harm_to_root * likelihood) + (objective_strength * (1.0 - likelihood))

likelihood: 0.7 base if move targets root, +0.2 if root is their best target,
            +0.1 if supporting another attacker, -0.3 if too exposed to attack us.
            0.1-0.3 base if move doesn't target root.
```

**Progressive narrowing:**
```
depth 1-3:  top 8-10 candidates
depth 4-6:  top 5-6
depth 7+:   top 3
```

**Board context also informs our own move ordering:** if context says Green is converging on our king with Blue's support, our defensive moves get searched first at MAX nodes -> tighter alpha-beta bounds -> more pruning.

**Why this works at depth 6-12:** This search runs 6-12 plies to answer "which of my moves survive tactical scrutiny?" At that depth the cheap filter runs at maybe a few thousand opponent nodes, the hybrid scoring runs ~1000 total evaluations, progressive narrowing keeps deep nodes tight, delta refresh happens 3-6 times total. The surviving moves get handed to MCTS for strategic evaluation.

**Huginn gates (this stage):**
- Board context gate: full BoardContext output
- Board context delta gate: what changed, what was updated, full re-read fallback triggered?
- Cheap filter gate: how many moves passed vs. background, which classified as interacting
- Reply scoring gate: every scored move -- opponent, objective strength, harm to root, likelihood, final score
- Progressive narrowing gate: depth, max allowed, how many considered vs. truncated

**Acceptance criteria:**
- Reply selection picks contextually realistic opponent moves
- Cheap filter correctly classifies interacting/non-interacting moves
- Progressive narrowing reduces node count at depth 6+
- Board context delta refresh matches full re-read
- Pre-search board read completes in < 1ms
- Hybrid BRS does not regress vs. plain BRS (measured by self-play or test position accuracy)

---

### STAGE 9: Transposition Table & Move Ordering

**Tier:** 3 — Strengthen Search
**Dependencies:** Stage 7 ([[stage_07_plain_brs]]) (uses BRS search loop; compatible with Stage 8 ([[stage_08_brs_hybrid]]) hybrid)

**The problem:** BRS search revisits the same positions through different move orders. Without a cache, you search them again from scratch. Without intelligent move ordering, alpha-beta prunes poorly. Both problems have well-known solutions.

**What you're building:**

1. **Transposition table.** Zobrist-keyed hash table. Each entry: key (upper 32 bits), best move, score, depth, flags (exact/lower/upper bound, age). 256 MB default. Depth-preferred replacement.

2. **Move ordering pipeline.** Priority: TT move -> winning captures (MVV-LVA) -> killer moves (2 per ply) -> counter-moves -> quiet moves by history heuristic -> losing captures (SEE < 0).

3. **Static Exchange Evaluation (SEE).** Predict whether a capture trade wins, loses, or is neutral. For 4PC: multiple opponents can recapture, making most captures more dangerous.

4. **History heuristic.** `history[player][piece_type][to_square]` counters. Incremented on beta cutoff by quiet moves, decremented on fail-low.

**Build order:**
1. TT data structure + probe/store
2. TT integration into BRS search (cutoffs, best move storage)
3. MVV-LVA capture ordering
4. Killer move tracking
5. History heuristic
6. SEE
7. Counter-move heuristic
8. Full move ordering pipeline

**What you DON'T need:**
- Multi-bucket TT schemes. One entry per index is fine to start.
- Sophisticated replacement policies. Depth-preferred with age is sufficient.

**TT entry:**
```rust
struct TTEntry {
    key: u32,        // upper 32 bits of Zobrist
    best_move: u16,  // compressed: from_square (8 bits) + to_square (8 bits)
    score: i16,      // centipawns
    depth: u8,
    flags: u8,       // EXACT, LOWER_BOUND, UPPER_BOUND, age
}
```

**Move compression for TT:** The canonical Move is a u32 (31 bits). The TT stores a compressed u16 containing only from_square and to_square. On TT hit, reconstruct the full Move by searching the current move list for the matching from/to pair. This is unambiguous in legal move lists (no two legal moves share both from and to squares, except promotions -- disambiguate by trying each). The tradeoff: 10 bytes per TT entry instead of 12, at the cost of a move list scan on TT hit.

**Huginn gates (this stage):**
- TT lookup gate: hash, hit/miss, stored depth/score/type/move on hit
- TT store gate: what's stored, replaced entry, replacement reason
- Move ordering gate: sequence produced with ordering scores
- Killer/SEE gate: killer moves per ply, SEE values per capture, capture reordering

**Acceptance criteria:**
- TT cutoffs reduce node count by >50% at depth 6
- Move ordering reduces node count significantly vs. random order
- No correctness regression (perft still passes)

---

### STAGE 10: MCTS Strategic Search

**Tier:** 3 — Strengthen Search
**Dependencies:** Stage 6 ([[stage_06_bootstrap_eval]])

**The problem:** BRS finds tactical moves but can't see long-term strategy. MCTS explores broadly using statistics -- it explores positions that BRS can't reach, evaluates multi-player dynamics through simulation, and handles uncertainty in opponent behavior. It needs to work with 4-player value vectors (each player has their own score).

**What you're building:**

1. **MCTS tree.** Nodes with visit count, 4-player value sum, children, prior probabilities.

2. **UCB1/PUCT selection.** At each node, pick the child that maximizes the current player's UCB1 score. PUCT when neural priors are available (uniform priors for now).

3. **4-player backpropagation (MaxN).** Each simulation produces a 4-element value vector. Backpropagate all 4 values up the tree. Each player maximizes their own component.

4. **Progressive widening.** `max_children = floor(W * N^B)`. Critical because 4 players x ~30 moves = too many children to expand naively.

5. **Leaf evaluation.** Uses the `Evaluator` trait from Stage 6. Evaluate from each player's perspective via `eval_4vec`, normalize to [0,1] with sigmoid.

6. **Simulation loop.** Select -> expand -> evaluate -> backpropagate. Budget by count or time. Return most-visited root child.

7. **MCTS implementation of `Searcher`.** `MctsSearcher` implements the `Searcher` trait defined in Stage 7.

**Build order:**
1. MCTS node struct
2. Selection (UCB1)
3. Expansion + leaf evaluation
4. Backpropagation (4-player MaxN)
5. Progressive widening
6. Simulation budget control
7. PV extraction (most-visited path)
8. Temperature for self-play exploration
9. Implement `MctsSearcher` against `Searcher` trait

**What you DON'T need:**
- Neural network priors (Stage 16). Use uniform priors.
- Root parallelism or virtual loss. Single-threaded is fine.
- Integration with BRS (Stage 11). MCTS works standalone first.

**MCTS node:**
```rust
struct MctsNode {
    move_to_here: Option<Move>,
    player_to_move: Player,
    visit_count: u32,
    value_sum: [f64; 4],
    prior: f32,
    children: Vec<MctsNode>,
    is_expanded: bool,
    is_terminal: bool,
}
```

**Huginn gates (this stage):**
- Simulation gate: selection path, leaf evaluation (4-vec), visit count before/after
- Selection gate (Verbose+): children UCB1 scores, child selected, player perspective
- Expansion gate: position, move, prior assigned, progressive widening check
- Root summary gate: full visit distribution, selected move, temperature

**Acceptance criteria:**
- MCTS finds reasonable moves in simple positions
- Visit counts concentrate on good moves over time
- 4-player value backpropagation is correct
- Progressive widening limits tree breadth
- 1000+ simulations in reasonable time
- `MctsSearcher` implements `Searcher` trait correctly

---

### STAGE 11: Hybrid Integration (BRS -> MCTS)

**Tier:** 3 — Strengthen Search
**Dependencies:** Stage 8 ([[stage_08_brs_hybrid]]), Stage 9 ([[stage_09_tt_ordering]]), Stage 10 ([[stage_10_mcts]])

**The problem:** BRS and MCTS exist independently. You need a controller that runs BRS first to filter moves tactically, then feeds the survivors to MCTS for strategic evaluation. The time budget must be split between the two phases.

**What you're building:**

1. **Search controller.** Orchestrates both phases: generate legal moves -> BRS tactical filter -> surviving moves -> MCTS strategic search -> best move. Composes two `Searcher` implementations through the trait.

2. **Surviving move threshold.** Any move within TACTICAL_MARGIN (default 150cp) of the best BRS score survives. Always keep at least 2 moves so MCTS has a choice.

3. **Adaptive time allocation.** Tactical positions (many captures/checks): 30% BRS, 70% MCTS. Quiet positions: 10% BRS, 90% MCTS. Also adapts on BRS result spread -- if all moves within 50cp, BRS can't distinguish, give more to MCTS.

4. **Unified info output.** `phase brs` during BRS, `phase mcts` during MCTS. Both emit depth, score, nodes, PV.

5. **Edge cases.** One legal move = instant return. Zero surviving moves = return best BRS move. One survivor = skip MCTS.

**Build order:**
1. Search controller skeleton (BRS -> threshold -> MCTS)
2. Surviving move threshold logic
3. Time allocation (fixed split first)
4. Adaptive time allocation (tactical vs. quiet detection)
5. Unified info output
6. Edge case handling

**What you DON'T need:**
- NNUE (Stage 16). Both phases use bootstrap eval via `Evaluator` trait.
- Pondering (Stage 13). Just allocate the given time budget.

**Controller flow:**
```
fn search(position, time_budget) -> Move:
    all_moves = generate_legal_moves(position)
    if len == 0: no move. if len == 1: return it.

    brs_results = brs_filter(position, all_moves, time_budget * 0.15)
    surviving = filter_survivors(brs_results, TACTICAL_MARGIN)
    emit_info("phase brs", brs_results)

    if surviving.len() == 1: return it.

    best = mcts_search(position, surviving, remaining_time)
    emit_info("phase mcts", mcts_results)
    return best
```

**Huginn gates (this stage):**
- Phase transition gate: surviving moves with BRS scores, time spent/remaining, threshold, eliminated count
- Surviving move comparison gate: BRS ranking vs. MCTS ranking (disagreements are informative)
- Time allocation gate: tactical/quiet detection, planned split, actual time consumed
- Search controller gate: full lifecycle from `go` to `bestmove`

**Acceptance criteria:**
- Hybrid finds better moves than BRS alone or MCTS alone (measured by test positions or self-play)
- BRS phase correctly filters losing moves
- MCTS phase respects the surviving move set
- Time allocation adapts to position type
- Engine never crashes or returns illegal moves under time pressure

---

### STAGE 12: Self-Play & Regression Testing

**Tier:** 4 — Measurement
**Dependencies:** Stage 11 ([[stage_11_hybrid_integration]])

**The problem:** You need infrastructure to measure whether changes improve or hurt the engine. Without this, you're guessing. Moving this to right after search integration means every subsequent stage (time management, NNUE, variants) gets measured from the start.

**What you're building:**

1. **Match manager.** Play N games between two engine versions, rotating colors. Compute ELO difference.

2. **SPRT.** Sequential Probability Ratio Test to decide if a change is a statistically significant improvement (H0: elo <= 0, H1: elo >= 5, alpha/beta = 0.05). Stop early when LLR exceeds bounds.

3. **Regression test suite.** Positions with known best moves or evaluation ranges. Tactical puzzles, defensive positions, endgame conversions.

4. **Data logging.** Store all self-play games for future NNUE training data (Stage 15).

**Build order:**
1. Match manager (play games, record results)
2. ELO calculation
3. SPRT implementation
4. Regression test position suite
5. Automated pipeline (run after each significant change)
6. Data logging for NNUE retraining

**What you DON'T need:**
- Opening books or endgame tablebases. The engine plays from the start.
- A rating list. Just relative comparison between versions.

**Huginn gates (this stage):**
- Regression detection gate: expected best move/eval, actual result, pass/fail
- Self-play anomaly gate: flags obviously bad moves (hung queen, walked into mate) with full Huginn trace for autopsy

**Acceptance criteria:**
- Match manager runs 1000+ games stably
- ELO calculations consistent with statistical theory
- SPRT correctly identifies improvements and rejections
- Regression tests catch known failure modes

---

### STAGE 13: Time Management

**Tier:** 4 — Measurement
**Dependencies:** Stage 11 ([[stage_11_hybrid_integration]]), Stage 12 ([[stage_12_self_play]])

**The problem:** The engine has a clock but no strategy for spending it. You need to allocate time per move based on position complexity, game phase, and remaining clock. You also need to tune search parameters using the self-play framework from Stage 12.

**What you're building:**

1. **Time allocation.** `base = remaining / expected_moves + increment`. Adjustments: tactical positions get more time, quiet positions less, critical scores (near elimination) get double, forced moves are instant. Safety: never use >25% of remaining, minimum 100ms.

2. **Search parameter tuning.** Use self-play (Stage 12) to tune: TACTICAL_MARGIN, BRS_TIME_FRACTION, MCTS_EXPLORATION_C, progressive widening exponent, null move reduction, LMR base.

3. **Pondering (optional).** Think on opponent's time.

**Build order:**
1. Basic time allocation formula
2. Position complexity detection (tactical vs. quiet)
3. Adaptive adjustments
4. Safety checks (never flag)
5. Parameter tuning via self-play
6. Pondering (if time permits)

**What you DON'T need:**
- Complex learning-based time management. The heuristic formula works.

**Huginn gates (this stage):**
- Time budget gate: remaining time, increment, complexity, computed budget, BRS/MCTS split
- Time overrun gate: budget exceeded, what was happening, graceful abort?
- Panic time gate: trigger condition, adjusted behavior

**Acceptance criteria:**
- Engine manages time correctly across full games (doesn't flag)
- Time adapts to position complexity
- Tuned parameters improve win rate vs. defaults

---

### STAGE 14: NNUE Feature Design & Architecture

**Tier:** 5 — Learn
**Dependencies:** Stage 6 ([[stage_06_bootstrap_eval]]) (Evaluator trait)

**The problem:** The bootstrap eval is crude -- it can't learn subtle piece interactions, board geometry, or 4-player dynamics. You need a neural network (NNUE) that evaluates positions fast enough for search (~1us incremental). This stage designs and implements the architecture and inference code. Training happens in Stage 15.

**What you're building:**

1. **Feature set (HalfKP-4).** Per perspective: `(piece_square, piece_type, relative_owner)`. Phase 1: 160 squares x 7 types x 4 relative owners = 4,480 features per perspective (~30 active per position, very sparse). Phase 2: add king bucketing (20 buckets x 4,480 = 89,600 features).

2. **Feature transformer.** Sparse input -> 256-neuron dense accumulator. One per perspective (4 total). SCReLU activation.

3. **Dual-head network.** All 4 accumulators concatenated (1024) -> shared hidden (32) -> BRS scalar head (1 centipawn value) + MCTS value head (4-player values, softmax). Optional policy head for MCTS priors.

4. **Incremental accumulator updates.** On non-king moves: add/subtract at most 3 feature columns. On king moves: full recompute. Same principle as Stockfish NNUE. Reads the "what changed" information from Stage 2's make/unmake (which piece moved, what was captured, was it a king move) to determine which features to add/remove.

5. **Quantized inference.** Feature transformer: int16 (scale 127). Hidden layers: int8 (scale 64). Accumulator: int16. Output: int32 scaled to centipawns.

6. **Weight file format (.onnue).** Header (magic, version, architecture hash), transformer weights, hidden layer weights, output heads, checksum.

7. **NNUE implementation of `Evaluator`.** `NnueEvaluator` implements the `Evaluator` trait from Stage 6.

**Build order:**
1. Feature set definition + feature index computation
2. Feature transformer (full computation from scratch)
3. Incremental accumulator update
4. Hidden layers + dual-head output
5. Quantized inference
6. .onnue file format load/save
7. Integration with make/unmake (accumulator push/pop)
8. Implement `NnueEvaluator` against `Evaluator` trait

**What you DON'T need:**
- Training code (Stage 15). This is architecture and inference only.
- Trained weights. Use random weights to verify the pipeline works.
- SIMD optimization (Stage 19). Scalar code first.

**Huginn gates (this stage):**
- Accumulator update gate: features added/removed, perspective, incremental vs. full recompute
- Forward pass gate: accumulator values, per-layer output, final scalar + vector outputs
- Quantization gate (Verbose+): float vs. quantized values at layer boundaries
- Weight load gate: architecture hash, feature set ID, parameter count, checksum

**Acceptance criteria:**
- Feature transformer produces correct output for known inputs
- Incremental updates match full recomputation
- Dual-head output produces valid ranges
- Quantized inference matches float within acceptable tolerance
- Inference speed: < 5us per incremental eval
- `NnueEvaluator` implements `Evaluator` trait correctly

---

### STAGE 15: NNUE Training Pipeline

**Tier:** 5 — Learn
**Dependencies:** Stage 14 ([[stage_14_nnue_design]]), Stage 12 ([[stage_12_self_play]]) (self-play for data generation)

**The problem:** You have the NNUE architecture but no trained weights. You need infrastructure to generate training data from self-play, train the network in PyTorch, and export weights to .onnue format.

**What you're building:**

1. **Training data format.** Each sample: position (FEN4 or binary), BRS eval target (centipawns from depth 8+), MCTS value target (4-vec), policy target (visit distribution), game result (4-vec: 1.0 win, 0.0 loss), metadata.

2. **Data generator.** Uses the self-play framework from Stage 12 to generate games. Extract positions at random intervals (every 4-8 plies). Record search outputs and game results. Stage 12's match manager handles the game infrastructure -- this stage adds the data extraction pipeline on top.

3. **PyTorch training script.** OdinNNUE model matching the Stage 14 architecture. Loss: `lambda_brs * MSE(brs) + lambda_mcts * CrossEntropy(mcts) + lambda_result * MSE(result)`.

4. **Weight export.** Convert PyTorch weights to .onnue binary format.

5. **Training schedule.** Gen-0: 100K self-play games with bootstrap eval -> train. Gen-1: 100K games with gen-0 NNUE -> retrain on combined data. Iterate.

**Build order:**
1. Training data format definition + binary serialization
2. Data generator (extract positions from self-play games via Stage 12)
3. PyTorch model + training loop
4. Validation + loss tracking
5. Weight export to .onnue
6. Gen-0 training run

**What you DON'T need:**
- Distributed training. Single-GPU PyTorch is fine.
- Fancy data augmentation. The 4-player symmetry provides some naturally.

**Huginn gates (this stage):**
- Data generation gate: position hash, BRS target, MCTS target, game result (catches malformed data)
- Training sample validation gate: flags positions where BRS and MCTS disagree significantly

**Acceptance criteria:**
- Training script runs, loss decreases
- Trained model loads via .onnue format
- Model evaluations correlate with game outcomes
- Data pipeline produces valid samples

---

### STAGE 16: NNUE Integration

**Tier:** 5 — Learn
**Dependencies:** Stage 15 ([[stage_15_nnue_training]]), Stage 12 ([[stage_12_self_play]])

**The problem:** You have trained weights and inference code. Now replace the bootstrap eval throughout the engine. This is the most dangerous swap -- the new eval might expose bugs in accumulator management or produce worse play than the handcrafted eval.

**Critical: This stage requires a mandatory before/after audit.** Record all metrics before the swap (search depth, NPS, test position scores, self-play win rate) and after. The `Evaluator` trait makes the swap clean -- change which implementation is behind the trait -- but verify everything.

**What you're building:**

1. **BRS integration.** Replace `BootstrapEvaluator` with `NnueEvaluator` behind the `Evaluator` trait at leaf nodes. Accumulator maintained incrementally through make/unmake.

2. **MCTS integration.** Replace bootstrap eval with NNUE MCTS head for leaf evaluation. Replace uniform priors with policy head output.

3. **Accumulator lifecycle management.** Full compute at root. Push/pop through BRS tree. Save/restore for MCTS simulations.

4. **Fallback.** Option to revert to `BootstrapEvaluator` when NNUE file is absent.

5. **A/B comparison.** Self-play tournament (Stage 12): NNUE engine vs. bootstrap engine.

**Build order:**
1. Accumulator lifecycle in BRS (push on make, pop on unmake)
2. BRS leaf eval swap (switch `Evaluator` implementation)
3. Accumulator lifecycle in MCTS (save at root, restore after simulation)
4. MCTS leaf eval swap
5. MCTS policy head integration
6. Fallback mechanism
7. A/B self-play comparison using Stage 12

**What you DON'T need:**
- Retraining. Use the weights from Stage 15. If they're bad, go back and retrain.
- Further NNUE architecture changes. The architecture is frozen from Stage 14.

**Performance targets:**
- NNUE incremental eval: < 2us (target 1us)
- NNUE full eval: < 50us
- BRS with NNUE: > 500K nps
- MCTS with NNUE: > 5K simulations/sec

**Huginn gates (this stage):**
- Eval swap gate: NNUE vs. bootstrap used, accumulator state. During transition, logs BOTH evaluations side-by-side for the same position.
- Accumulator lifecycle gate: tracks through full search lifecycle (catches state leaking between branches)
- NNUE-vs-bootstrap comparison gate (temporary): flags positions with >200cp disagreement

**Acceptance criteria:**
- NNUE engine beats bootstrap engine in self-play (>55% win rate)
- No correctness regression (perft passes, same legal moves)
- Incremental updates match full computation
- Performance targets met
- Fallback works when NNUE file is absent

---

### STAGE 17: Game Mode Variant Tuning

**Tier:** 6 — Polish
**Dependencies:** Stage 11 ([[stage_11_hybrid_integration]]) (hybrid search working), Stage 3 ([[stage_03_gamestate]]) (rules already implemented)

**The problem:** DKW, FFA scoring, Terrain, and Chess960 are implemented in the rules engine (Stage 3) but the search and eval don't account for them. The engine needs to play intelligently in each mode. Changes must be isolated behind the `GameMode` enum, not scattered through search code.

**What you're building:**

1. **DKW in search.** BRS accounts for random DKW king moves. MCTS treats them as chance nodes (average over random moves). Eval considers DKW kings as obstacles.

2. **FFA scoring in eval.** Points weight, leader penalty, elimination bonus, stalemate awareness (20 points for self-stalemate if losing).

3. **Terrain in search.** Eval assesses how terrain changes board geometry (fortresses, blocked diagonals, outposts). MCTS may need more simulations due to added complexity.

4. **Chess960 generator.** Generate valid random starting position (bishops on opposite colors, king between rooks), rotate for all 4 players. Chess960 castling conventions.

5. **Mode-specific tuning.** Different eval weights or search parameters per mode if needed. Self-play (Stage 12) validates changes.

**Build order:**
1. Chess960 position generator + validation
2. DKW chance node handling in MCTS
3. DKW awareness in BRS
4. FFA scoring integration in eval
5. Terrain-aware evaluation
6. Mode-specific parameter tuning via self-play

**What you DON'T need:**
- New game modes beyond what's specified. No teams mode evaluation tuning (that can come later).
- Mode-specific NNUE retraining (can do in a future training cycle).

**Huginn gates (this stage):**
- DKW random move gate: position, move, interference with active players
- Terrain conversion gate: pieces converted, positions, movegen diff before/after
- Chess960 setup gate: arrangement, bishop colors, king-between-rooks validation
- Scoring anomaly gate: flags score changes not matching the point table

**Acceptance criteria:**
- DKW games play to completion correctly
- FFA scores accurate per chess.com rules
- Terrain pieces persist and block movement
- Chess960 positions valid
- Engine plays reasonably in all modes

---

### STAGE 18: Full UI

**Tier:** 6 — Polish
**Dependencies:** Stage 5 ([[stage_05_basic_ui]]), Stage 11 ([[stage_11_hybrid_integration]])

**The problem:** The basic UI shell from Stage 5 works but lacks play modes, self-play observation, and comprehensive debug tooling. You need a full interface for development and testing.

**What you're building:**

1. **Play modes.** Human vs. engine, human vs. human (hot-seat), engine vs. engine. All 4 slots configurable: Human / Engine / Off.

2. **Visual enhancements.** Move arrows, last move highlight, check highlight, terrain piece styling.

3. **Comprehensive debug panel.** Best move + eval, per-player scores/values, depth/nodes/NPS, BRS surviving moves list, MCTS visit distribution bar chart, phase indicator, timeout reasons, Huginn trace viewer (expandable tree).

4. **Self-play dashboard.** Start/stop, speed control, game count, aggregate stats (win rates, avg game length, avg time/move), live board view.

5. **Game controls.** New game, undo, redo, set position (FEN4 input), mode selection (FFA, teams, terrain, 960).

**Build order:**
1. Player configuration (Human/Engine/Off per slot)
2. Play mode logic (who moves when, engine auto-play)
3. Visual enhancements
4. Debug panel with all specified fields
5. Huginn trace viewer
6. Self-play dashboard
7. Extended game controls

**What you DON'T need:**
- Game logic. The UI still owns ZERO game logic. Everything goes through the engine.
- Online play. This is local only.

**Huginn gates (this stage):**
None new in the engine. But this stage completes the Huginn display surface: expandable trace tree, simulation histogram, BRS/MCTS phase timeline, eval flow visualizer. This is where all observation points from every previous stage become visible.

**Acceptance criteria:**
- Can play a full game against engine
- Can watch engine vs. engine games
- Debug console shows all specified info
- Self-play runs 100+ games without crashes
- Terrain mode pieces display distinctly
- No game logic in UI code

---

### STAGE 19: Optimization & Hardening

**Tier:** 6 — Polish
**Dependencies:** Stage 16 ([[stage_16_nnue_integration]]) (NNUE integrated), Stage 12 ([[stage_12_self_play]]) (regression testing)

**The problem:** The engine works. Now make it fast, robust, and bullet-proof. Profile first, optimize second. The regression test suite (Stage 12) validates every optimization doesn't break correctness.

**What you're building:**

1. **SIMD for NNUE.** AVX2/SSE4 on x86, NEON on ARM. Process 16 int16 values per instruction for accumulator updates, 32 int8 for hidden layers. Use `std::arch` with scalar fallback.

2. **Memory optimization.** Pre-allocated move lists (no per-node allocation), MCTS arena allocator, stack-allocated move buffers in BRS.

3. **Profile-guided optimization.** Profile hot paths (NNUE eval, movegen, hash lookup), optimize the top bottlenecks.

4. **Bitboard retrofit (if profiling warrants).** Add bitboard operations behind the attack query API (Stage 2's abstraction boundary) without touching any code above Stage 2.

5. **Stress testing.** 10,000 self-play games across all modes. Fuzz test with random positions. Boundary cases (max pieces, all terrain, triple check).

6. **Error handling hardening.** No panics in the engine binary. Graceful degradation everywhere.

**Build order:**
1. Profile the engine, identify top bottlenecks
2. SIMD for NNUE inference
3. Memory optimization for search
4. Bitboard retrofit (only if profiling shows board scanning as bottleneck)
5. Stress testing
6. Edge case audit
7. Error handling pass

**What you DON'T need:**
- GPU inference. NNUE is designed for CPU.
- Multithreaded search (can be a future project).

**Acceptance criteria:**
- No crashes in 10,000 self-play games
- No panics from fuzz testing
- NNUE eval < 1us (incremental) with SIMD
- BRS > 1M nps with NNUE
- MCTS > 10K simulations/sec with NNUE

---

## 4.1 MAINTENANCE INVARIANTS

These rules apply to every stage after the feature is introduced. They are not stage-specific -- they are permanent.

| Invariant | Introduced | What It Means |
|-----------|-----------|---------------|
| **Prior-stage tests never deleted** | Stage 0 | Tests from earlier stages are never removed or modified to accommodate new code. |
| **Huginn compiles to nothing when off** | Stage 0 | `cargo build` (without `--features huginn`) produces a binary with zero Huginn symbols. |
| **Board representation tests pass** | Stage 1 | Board representation tests pass, FEN4 round-trips work. |
| **Perft values are forever** | Stage 2 | Once perft values are established, they never change. Any stage that causes perft to fail has a bug. |
| **Zobrist round-trip** | Stage 2 | `make -> unmake` restores the exact Zobrist hash. Always. (Zobrist keys are defined in Stage 1, but the invariant is enforceable from Stage 2 when make/unmake logic exists.) |
| **Attack query API is the board boundary** | Stage 2 | Nothing above Stage 2 reads `board.squares[]` directly. All board queries go through the attack query API. |
| **Game playouts complete without crashes** | Stage 3 | Random game playouts (1000+) complete without crashes, all game modes terminate correctly. |
| **Protocol round-trip works** | Stage 4 | Send position + go, get legal bestmove back. |
| **UI owns zero game logic** | Stage 5 | The UI never validates moves, computes legal moves, detects check, or evaluates positions. |
| **Evaluator trait is the eval boundary** | Stage 6 | All search code calls through the `Evaluator` trait. Never calls a specific implementation directly. |
| **Eval produces sane values** | Stage 6 | Materially different positions get different scores. |
| **Searcher trait is the search boundary** | Stage 7 | The hybrid controller composes through the `Searcher` trait. Never calls BRS or MCTS internals directly. |
| **Engine finds forced mates** | Stage 7 | Engine finds mate-in-1 in all test positions, doesn't hang pieces within search depth. |
| **TT produces no correctness regressions** | Stage 9 | Adding TT must not change the set of legal moves or corrupt search results. |

---

## 5. AUDIT PROTOCOL

Every stage undergoes two audits:

**PRE-AUDIT (before work begins):**
- Review audit/downstream logs from previous stages
- Verify build compiles and tests pass
- Check for flagged issues affecting this stage
- Document findings in `audit_log_stage_XX.md`

**POST-AUDIT (after work completes):**
- Verify all deliverables met
- Run all tests (existing + new)
- Check for: uniformity, bloat, efficiency, dead code, broken code, temporary code, future conflicts, search/eval issues, unaccounted concerns
- Document findings in `audit_log_stage_XX.md`
- Document downstream notes in `downstream_log_stage_XX.md`

**Comprehensive audit checklist:** See `AGENT_CONDUCT.md` ([[AGENT_CONDUCT]]) Section 2 (26 categories).
**Audit log format:** See `audit_log_stage_00.md` ([[audit_log_stage_00]]) template.
**Downstream log format:** See `downstream_log_stage_00.md` ([[downstream_log_stage_00]]) template.
**Audit procedures:** See `AGENT_CONDUCT.md` ([[AGENT_CONDUCT]]) Section 6.

---

## 6. NAMING CONVENTIONS

| Entity | Convention | Example |
|--------|-----------|---------|
| Project | Title Case | Project Odin |
| Engine | PascalCase | Odin |
| Telemetry | "Huginn" | Huginn trace |
| Rust modules | snake_case | `move_gen`, `board_repr` |
| Rust types | PascalCase | `GameState`, `MctsNode` |
| Rust functions | snake_case | `generate_legal_moves` |
| Rust constants | SCREAMING_SNAKE | `MAX_DEPTH`, `TACTICAL_MARGIN` |
| UI components | PascalCase | `BoardDisplay`, `DebugConsole` |
| Protocol commands | lowercase | `bestmove`, `isready` |

---

## 7. GLOSSARY

| Term | Definition |
|------|-----------|
| **BRS** | Best Reply Search. Collapses opponent moves into a single best reply per ply. Odin uses BRS's tree structure for alpha-beta compatibility. |
| **BRS/Paranoid Hybrid** | Odin's tactical search. BRS tree structure with reply selection driven by a board read that blends "objectively strongest" with "most harmful to me" weighted by likelihood given the board's incentive structure. |
| **Paranoid** | Search assuming all opponents minimize root player's score. In FFA this is realistic -- opponents target the leader. Odin uses Paranoid's realism as one signal in hybrid scoring. |
| **MCTS** | Monte Carlo Tree Search. Tree search using random sampling and statistics. |
| **NNUE** | Efficiently Updatable Neural Network. Fast neural evaluation with incremental updates. |
| **PUCT** | Predictor + Upper Confidence bounds for Trees. MCTS selection with neural priors. |
| **UCB1** | Upper Confidence Bound 1. Bandit algorithm for MCTS selection. |
| **Huginn** | Odin's telemetry/tracer. Passive, read-only, external observer. Compile-gated: absent from release builds. |
| **Perft** | Performance test. Counting leaf nodes at given depth to verify move generation. |
| **Zobrist** | Position hash using XOR of random values per piece-square combination. |
| **DKW** | Dead King Walking. Eliminated king continues making random moves. |
| **Terrain** | Odin-specific: eliminated pieces become permanent immovable obstacles. |
| **FFA** | Free-For-All. Every player for themselves. |
| **MaxN** | Multiplayer search where each player maximizes their own score. |
| **MVV-LVA** | Most Valuable Victim - Least Valuable Attacker. Capture ordering heuristic. |
| **SEE** | Static Exchange Evaluation. Predicting outcome of a capture sequence. |
| **SPRT** | Sequential Probability Ratio Test. Statistical method for testing if a change improves the engine. |
| **SCReLU** | Squared Clipped ReLU. Activation: clamp(x, 0, 1)^2. |
| **Odin Protocol** | Custom UCI-like protocol for 4-player chess engine communication. |

---

## APPENDIX A: Stage Dependency Map

```
TIER 1 — FOUNDATION
Stage 0  --- (none) ---------> Skeleton + Huginn Core
Stage 1  --- 0 ---------------> Board Representation
Stage 2  --- 1 ---------------> Move Generation + Attack Query API
Stage 3  --- 2 ---------------> Game State & Rules
Stage 4  --- 3 ---------------> Odin Protocol
Stage 5  --- 4 ---------------> Basic UI Shell

TIER 2 — SIMPLE SEARCH
Stage 6  --- 3 ---------------> Bootstrap Eval + Evaluator Trait
Stage 7  --- 6 ---------------> Plain BRS + Searcher Trait

TIER 3 — STRENGTHEN SEARCH
Stage 8  --- 7 ---------------> BRS/Paranoid Hybrid Layer
Stage 9  --- 7 ---------------> TT & Move Ordering
Stage 10 --- 6 ---------------> MCTS Strategic Search
Stage 11 --- 8 + 9 + 10 ------> Hybrid Integration

TIER 4 — MEASUREMENT
Stage 12 --- 11 --------------> Self-Play & Regression Testing
Stage 13 --- 11 + 12 ---------> Time Management

TIER 5 — LEARN
Stage 14 --- 6 ---------------> NNUE Feature Design & Architecture
Stage 15 --- 14 + 12 ---------> NNUE Training Pipeline
Stage 16 --- 15 + 12 ---------> NNUE Integration

TIER 6 — POLISH
Stage 17 --- 11 + 3 ----------> Game Mode Variant Tuning
Stage 18 --- 5 + 11 ----------> Full UI
Stage 19 --- 16 + 12 ---------> Optimization & Hardening
```

**Parallel work opportunities:**
- Stages 5 and 6 can run in parallel (UI doesn't need eval, eval doesn't need UI)
- Stages 8, 9, and 10 can all run in parallel (8 depends on 7, 9 depends on 7, 10 depends on 6 -- no cross-dependencies among 8, 9, and 10)
- Stage 14 can start in parallel with Stages 10-11 (NNUE design depends only on the Evaluator trait from Stage 6)
- Stages 17 and 18 can run in parallel (variant tuning doesn't need full UI, full UI doesn't need variants)

## APPENDIX B: Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| 160-square board makes NNUE too large | Medium | High | Start with simplified feature set (no king-relative), upgrade if needed |
| Hybrid reply heuristics need extensive tuning | High | Medium | Self-play framework (Stage 12) provides feedback early. Huginn's reply scoring gate shows exactly which replies were chosen and why. |
| MCTS too slow for real-time play | Medium | High | Progressive widening, strong NNUE priors reduce needed simulations |
| Terrain mode creates degenerate endgames | Low | Low | Terrain is opt-in; engine can learn to play around it |
| DKW random moves create noise in search | Medium | Low | Model as chance nodes in MCTS; in BRS, ignore (they're random) |
| Zobrist collisions with 4 players + large board | Low | High | Use u128 if collisions detected |
| NNUE training data insufficient | Medium | High | Start with bootstrap games via self-play framework (Stage 12), iterate; quality > quantity |
| UI/engine desync | Low | Medium | Odin Protocol enforces strict request-response; UI never assumes |

---

*End of Masterplan v3.0*
