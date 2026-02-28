# Stage 14 Prompt — NNUE Feature Design & Architecture

You are implementing Stage 14 of Project Odin, a four-player chess engine (14x14 board, R/B/Y/G).

**Read these files before writing any code:**
1. `masterplan/STATUS.md` — project state (Stage 13 complete, 490 tests, v1.13)
2. `masterplan/HANDOFF.md` — last session summary
3. `masterplan/AGENT_CONDUCT.md` Section 1.1 — stage entry protocol
4. `masterplan/DECISIONS.md` — ADR-003 (Dual-Head NNUE), ADR-004 (HalfKP-4), ADR-012 (Turn Order)
5. `masterplan/MASTERPLAN.md` — Stage 14 section (search for "Stage 14")
6. `masterplan/downstream_log_stage_13.md` — warnings W14-W16 you must respect
7. `masterplan/audit_log_stage_13.md` — current test/build baselines

---

## What You're Building

The NNUE inference pipeline — feature encoding, accumulator, forward pass, weight loading. **No training code** (Stage 15). **No search integration** (Stage 16). Use random weights to verify correctness.

---

## Architecture Overview

### 1. Feature Set: HalfKP-4

Per-perspective encoding: `(piece_square, piece_type, relative_owner)`

- **160** valid board squares (14x14 minus 36 corners)
- **7** piece types: Pawn(0), Knight(1), Bishop(2), Rook(3), Queen(4), King(5), PromotedQueen(6)
- **4** relative owners: Own(0), CW-Opponent(1), Across-Opponent(2), CCW-Opponent(3)
  - Relative to the perspective player, NOT absolute color
- **Total features per perspective: 160 x 7 x 4 = 4,480**
- **Active per position: ~30** (very sparse)

Feature index formula:
```
feature_index(square, piece_type, relative_owner) = square_index * 28 + piece_type * 4 + relative_owner
```
where `square_index` is the dense index (0-159) of the square on the valid board.

**You must build a square-to-dense-index mapping.** The 14x14 board has 36 invalid corner squares. Map the 160 valid squares to dense indices 0-159.

### 2. Network Topology

```
Input: 4,480 sparse features (per perspective)
  |
  v
Feature Transformer: 4,480 -> 256 (SCReLU activation, int16 quantized)
  |                                    x4 perspectives
  v
Concatenate: 4 x 256 = 1024
  |
  v
Hidden Layer: 1024 -> 32 (ClippedReLU)
  |
  v
Dual Output Heads:
  - BRS Head: 32 -> 1 (centipawn scalar, i16)
  - MCTS Head: 32 -> 4 (per-player values, sigmoid -> [0,1])
```

**SCReLU** (Squared Clipped ReLU): `screlu(x) = clamp(x, 0, QA)^2` where QA = 255 (quantization scale). The squaring improves gradient flow for training and is cheap at inference.

### 3. Quantization Scheme

| Layer | Weight type | Accumulator type | Scale |
|-------|-------------|------------------|-------|
| Feature Transformer | int16 | int16 | QA = 255 |
| Hidden Layer | int8 | int32 | QB = 64 |
| Output Heads | int8 | int32 | — |

After SCReLU squaring in the transformer, values are in range [0, QA^2]. The hidden layer rescales.

### 4. Accumulator & Incremental Updates

```rust
pub struct Accumulator {
    /// Per-perspective accumulators. Index by Player::index().
    values: [[i16; 256]; 4],
    /// Whether each perspective needs full recompute (king moved).
    needs_refresh: [bool; 4],
}

pub struct AccumulatorStack {
    stack: Vec<Accumulator>,  // pre-allocated to MAX_DEPTH
    current: usize,
}
```

**Push (on make_move):**
1. Copy `stack[current]` to `stack[current + 1]`
2. For each perspective (4 players):
   - If the moving piece is that perspective's king → mark `needs_refresh[perspective] = true`
   - Else: compute feature deltas (removed features, added features), update accumulator incrementally
3. Increment `current`

**Pop (on unmake_move):**
1. Decrement `current` — previous accumulator is untouched (copy-on-push)

**Full refresh** (when `needs_refresh` is true):
- Scan the board's piece list, compute all active features, recompute the 256-neuron accumulator from scratch
- Clear `needs_refresh` flag

**Feature delta computation for a non-king move:**
- Remove: `(from_sq, moving_piece_type, relative_owner)` — piece left this square
- Add: `(to_sq, moving_piece_type, relative_owner)` — piece arrived here
- If capture: Remove `(to_sq, captured_piece_type, captured_relative_owner)` — captured piece gone
- If promotion: Remove uses moving piece type, Add uses promoted piece type
- If castling: also move the rook (additional remove + add pair)
- If en passant: captured pawn is on a different square than `to_sq`

**What you can read from Move + MoveUndo:**
```rust
// Move (u32 bitfield):
move.from_sq()          // u8
move.to_sq()            // u8
move.piece_type()       // PieceType (the moving piece)
move.captured_piece()   // PieceType (0 if no capture) — WARNING: this is just the type
move.promotion()        // PieceType (0 if no promotion)
move.flags()            // Castle, EnPassant, DoublePush, Promotion flags

// MoveUndo (returned by make_move):
undo.captured_piece     // Option<Piece> — full Piece struct (type + owner)
```

### 5. Forward Pass (Inference)

```
fn forward(accumulators: &[[i16; 256]; 4], perspective_order: [Player; 4]) -> (i16, [f64; 4]) {
    // 1. Apply SCReLU to each perspective's accumulator
    //    screlu_out[i] = clamp(acc[i], 0, QA)^2   (produces i32 temporarily)

    // 2. Concatenate 4 perspectives in perspective_order → 1024 values
    //    perspective_order[0] = root player (the one BRS is evaluating for)
    //    perspective_order[1..3] = opponents in turn order

    // 3. Hidden layer: 1024 -> 32 (int8 weights, int32 accumulation, ClippedReLU)

    // 4a. BRS head: 32 -> 1 scalar (rescale to centipawns)
    // 4b. MCTS head: 32 -> 4 values (sigmoid each to [0,1])

    // Return (brs_cp, mcts_values)
}
```

### 6. .onnue Weight File Format

Binary format:
```
[Header — 48 bytes]
  Magic: "ONUE" (4 bytes)
  Version: u32 = 1
  Architecture hash: [u8; 32] — SHA256 of architecture descriptor string
  Feature count: u32 = 4480
  Hidden size: u32 = 256

[Feature Transformer — 4 perspectives]
  Weights: int16[4][4480][256]  (4 perspectives x 4480 features x 256 neurons)
  Biases:  int16[4][256]

[Hidden Layer]
  Weights: int8[1024][32]
  Biases:  int32[32]

[BRS Output Head]
  Weights: int8[32]
  Bias:    int32

[MCTS Output Head]
  Weights: int8[32][4]
  Biases:  int32[4]

[Footer]
  Checksum: u32 (CRC32 of all preceding bytes)
```

Implement `NnueWeights::load(path) -> Result<Self>` and `NnueWeights::save(path) -> Result<()>`.

For testing, implement `NnueWeights::random(seed: u64) -> Self` that fills weights with deterministic pseudo-random values.

### 7. NnueEvaluator Struct

```rust
pub struct NnueEvaluator {
    weights: NnueWeights,
    accumulators: AccumulatorStack,
}

impl Evaluator for NnueEvaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16 {
        // 1. Ensure accumulator is current (refresh if needed)
        // 2. Forward pass with player as perspective_order[0]
        // 3. Return BRS head output
    }

    fn eval_4vec(&self, position: &GameState) -> [f64; 4] {
        // 1. Ensure accumulator is current
        // 2. Forward pass
        // 3. Return MCTS head outputs (4 sigmoid values)
    }
}
```

**Important:** The Evaluator trait is FROZEN. Signature is:
```rust
pub trait Evaluator {
    fn eval_scalar(&self, position: &GameState, player: Player) -> i16;
    fn eval_4vec(&self, position: &GameState) -> [f64; 4];
}
```

Note `&self` not `&mut self`. The accumulator stack needs interior mutability (`RefCell` or `UnsafeCell`) since eval is called from immutable search contexts. Use `RefCell<AccumulatorStack>` for safety — the performance cost is negligible vs. the forward pass.

**Stage 14 scope:** NnueEvaluator exists and passes all tests with random weights. It does NOT replace BootstrapEvaluator yet — that's Stage 16.

---

## File Structure

Create these files:
- `odin-engine/src/eval/nnue/mod.rs` — NnueEvaluator, forward pass
- `odin-engine/src/eval/nnue/features.rs` — HalfKP-4 feature indexing, square mapping
- `odin-engine/src/eval/nnue/accumulator.rs` — Accumulator, AccumulatorStack, incremental updates
- `odin-engine/src/eval/nnue/weights.rs` — NnueWeights, .onnue format, load/save/random
- `odin-engine/tests/stage_14_nnue.rs` — all acceptance tests

Modify:
- `odin-engine/src/eval/mod.rs` — add `pub mod nnue;` and re-export NnueEvaluator

---

## Build Order

**Step 1: Feature Indexing**
- Build the 160-entry square-to-dense-index lookup table
- Implement `feature_index(square_dense: u8, piece_type: PieceType, relative_owner: u8) -> u16`
- Implement `relative_owner(piece_owner: Player, perspective: Player) -> u8`
- Test: index range [0, 4480), no collisions, round-trip consistency

**Step 2: Weights**
- Define `NnueWeights` struct with all weight/bias arrays
- Implement `NnueWeights::random(seed)` with SplitMix64 (same PRNG used in MCTS — see `search/mcts.rs`)
- Implement `.onnue` binary load/save with magic, version, checksum
- Test: save then load round-trip produces identical weights

**Step 3: Accumulator**
- Define `Accumulator` and `AccumulatorStack`
- Implement full-refresh from a Board's piece list
- Test: full refresh on starting position produces 4 valid accumulators (one per perspective)

**Step 4: Incremental Updates**
- Implement push (copy + delta update) and pop (decrement)
- Handle: quiet moves, captures, promotions, castling, en passant, king moves (mark refresh)
- Test: incremental update matches full refresh after 1, 5, 10, 20 random moves
- Test: push N + pop N restores original accumulator bit-for-bit

**Step 5: Forward Pass**
- Implement SCReLU, hidden layer matmul, ClippedReLU, dual output heads
- All integer arithmetic (no floats until final sigmoid)
- Test: known-input forward pass produces expected output (compute reference by hand or with float version)

**Step 6: NnueEvaluator**
- Wire accumulator + forward pass behind `Evaluator` trait
- `RefCell<AccumulatorStack>` for interior mutability
- Test: `eval_scalar` returns value in [-30000, 30000]
- Test: `eval_4vec` returns 4 values in [0.0, 1.0]
- Test: same position evaluated twice gives identical result

**Step 7: Benchmarks**
- Measure full eval time (target: <50us)
- Measure incremental update time (target: <5us, goal: 1us)
- Print results as `info string` in tests

---

## Acceptance Criteria (Tests Required)

Write all tests in `odin-engine/tests/stage_14_nnue.rs`:

| ID | Test | What it verifies |
|----|------|-----------------|
| T1 | `test_feature_index_range` | All indices in [0, 4480), no duplicates |
| T2 | `test_feature_index_symmetry` | Same piece on same square from different perspectives gives different indices |
| T3 | `test_dense_square_mapping` | 160 valid squares map to dense 0-159, invalid squares rejected |
| T4 | `test_weights_random_deterministic` | Same seed → same weights |
| T5 | `test_weights_save_load_roundtrip` | Save to temp file, load back, all values identical |
| T6 | `test_accumulator_full_refresh` | Starting position produces valid (non-zero) accumulators |
| T7 | `test_incremental_matches_full` | After N random moves, incremental acc == full refresh acc |
| T8 | `test_push_pop_roundtrip` | Push N, pop N → accumulator matches original bit-for-bit (N=1,5,10,20) |
| T9 | `test_forward_pass_deterministic` | Same input → same output, twice |
| T10 | `test_eval_scalar_range` | Output in [-30000, 30000] for various positions |
| T11 | `test_eval_4vec_range` | All 4 values in [0.0, 1.0] |
| T12 | `test_eval_consistency` | eval_scalar(pos, Red) and eval_4vec(pos)[0] are monotonically related |
| T13 | `test_incremental_captures` | Accumulator correct after a capture move |
| T14 | `test_incremental_castling` | Accumulator correct after castling (king + rook both move) |
| T15 | `test_incremental_promotion` | Accumulator correct after pawn promotion |
| T16 | `test_onnue_magic_validation` | Loading a file with wrong magic bytes fails gracefully |
| T17 | `test_benchmark_incremental` | Incremental eval < 5us (prints timing) |
| T18 | `test_benchmark_full` | Full eval < 50us (prints timing) |

---

## Critical Invariants — DO NOT VIOLATE

1. **Evaluator trait is FROZEN.** Do not modify the trait signature.
2. **Turn order R→B→Y→G** (ADR-012). Relative owner mapping must respect this.
3. **perft values are permanent invariants.** perft(1)=20, perft(2)=395, perft(3)=7800, perft(4)=152050. If any change, you broke something.
4. **`unmake_move` takes 3 args:** `unmake_move(board, mv, undo)`. Don't change this.
5. **Do NOT modify BootstrapEvaluator.** NnueEvaluator is a parallel implementation, not a replacement yet.
6. **Do NOT integrate into search.** Stage 16 does that. Stage 14 is inference-only.
7. **SIGMOID_K = 4000.0** for eval_4vec sigmoid normalization. Use this same constant.
8. **Eliminated players score -30,000.** NnueEvaluator should return ELIMINATED_SCORE for eliminated players.

---

## Existing Code You'll Need

**Board piece access:**
```rust
board.piece_at(sq)           // -> Option<Piece>
board.piece_lists()          // -> &[[(PieceType, Square); 20]; 4] (with piece_counts)
board.piece_counts()         // -> &[u8; 4]
board.king_squares()         // -> &[u8; 4]
board.side_to_move()         // -> Player
```

**Player enum:**
```rust
pub enum Player { Red = 0, Blue = 1, Yellow = 2, Green = 3 }
player.index() -> usize     // 0-3
Player::from_index(i)       // usize -> Player
player.next()               // next in turn order
```

**PieceType enum:**
```rust
pub enum PieceType { Pawn=0, Knight=1, Bishop=2, Rook=3, Queen=4, King=5, PromotedQueen=6 }
piece_type.index() -> usize  // 0-6
```

**Square validity:**
```rust
use crate::board::is_valid_square;  // checks the 36 invalid corners
```

**GameState:**
```rust
game_state.board()           // -> &Board
game_state.player_status(p)  // -> PlayerStatus (Active, Eliminated, Stalemated)
```

**SplitMix64 PRNG** (for random weights — same one used in MCTS):
```rust
// See odin-engine/src/search/mcts.rs for the implementation
// Copy the splitmix64 function, or extract it to a shared util
```

---

## Warnings from Downstream Logs

- **W14:** `TimeManager::allocate()` uses `score_cp < 2000` for near-elimination. NNUE eval should stay in the same centipawn scale (material baseline ~4300cp per player). If your output scaling differs, document it.
- **W13 (carried):** MCTS score can hit 9999 (max). Not your problem, but be aware.

---

## Scope Boundaries — What NOT To Build

- Training code / data generation (Stage 15)
- Search integration / replacing BootstrapEvaluator (Stage 16)
- SIMD vectorization (Stage 19)
- King bucketing / Phase 2 features (deferred)
- Policy head (deferred)

---

## Pre-Audit Checklist (Do This Before Coding)

Per AGENT_CONDUCT.md 1.1:
1. `cargo build && cargo test` — verify 490 tests pass
2. `cargo clippy` — verify 0 warnings
3. Create `masterplan/audit_log_stage_14.md` with pre-audit section
4. Record build state, test counts, any observations

## Post-Audit Checklist (After All Tests Pass)

1. `cargo test` — all existing 490 + new tests pass
2. `cargo clippy` — 0 warnings
3. Fill post-audit section of `audit_log_stage_14.md`
4. Create `masterplan/downstream_log_stage_14.md` with API contracts, warnings, baselines
5. Update `masterplan/STATUS.md` and `masterplan/HANDOFF.md`
6. Create session note in `masterplan/sessions/`
