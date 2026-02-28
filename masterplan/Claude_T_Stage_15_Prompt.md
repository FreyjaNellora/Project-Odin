# Stage 15 Prompt — NNUE Training Pipeline

You are implementing Stage 15 of Project Odin, a four-player chess engine (14x14 board, R/B/Y/G).

**Read these files before writing any code:**
1. `masterplan/STATUS.md` — project state (Stage 14 complete, 519 tests, v1.14)
2. `masterplan/HANDOFF.md` — last session summary
3. `masterplan/AGENT_CONDUCT.md` Section 1.1 — stage entry protocol
4. `masterplan/DECISIONS.md` — ADR-003 (Dual-Head NNUE), ADR-004 (HalfKP-4)
5. `masterplan/MASTERPLAN.md` — Stage 15 section
6. `masterplan/downstream_log_stage_14.md` — NNUE API contracts, constants, warnings

---

## What You're Building

The complete training pipeline: data generation from self-play → PyTorch training → .onnue weight export. By the end, a `weights_gen0.onnue` file exists that the Rust engine can load.

This stage is primarily **Python + Node.js**, not Rust. The only Rust changes are a datagen subcommand for the engine.

---

## Architecture Recap (Stage 14 — What Training Must Produce Weights For)

```
Input: 4,480 sparse features per perspective (HalfKP-4)
  |
Feature Transformer: 4,480 -> 256 (SCReLU, int16 quantized) x4 perspectives
  |
Concatenate: 4 x 256 = 1024
  |
Hidden Layer: 1024 -> 32 (ClippedReLU, int8 weights)
  |
Dual Output Heads:
  - BRS Head: 32 -> 1 (centipawn scalar)
  - MCTS Head: 32 -> 4 (per-player sigmoid values)
```

**Key constants from Stage 14:**
| Constant | Value |
|----------|-------|
| FEATURES_PER_PERSPECTIVE | 4,480 |
| FT_OUT | 256 |
| HIDDEN_SIZE | 32 |
| QA (SCReLU clamp) | 255 |
| OUTPUT_SCALE | 400 |
| SIGMOID_K | 4000.0 |

**Weight dimensions (.onnue layout):**
| Layer | Shape | Type |
|-------|-------|------|
| FT weights | [4][4480][256] | int16 |
| FT biases | [4][256] | int16 |
| Hidden weights | [1024][32] | int8 |
| Hidden biases | [32] | int32 |
| BRS weights | [32] | int8 |
| BRS bias | [1] | int32 |
| MCTS weights | [32][4] | int8 |
| MCTS biases | [4] | int32 |

---

## Pipeline Overview

```
[1] Self-Play Data Generation (Node.js + Engine)
    match.mjs runs 1000+ games at depth 6-8
    Per-move: position_moves, score_cp, v1-v4, depth, game result
         |
         v
[2] Position Extraction (Rust datagen subcommand)
    Replays each game from move list
    Extracts per-position: active features (4 perspectives), BRS target, MCTS target
    Outputs binary training samples (.bin)
         |
         v
[3] PyTorch Training (Python)
    Reads .bin samples, trains OdinNNUE model
    Multi-task loss: BRS MSE + MCTS cross-entropy + game result
    Outputs PyTorch checkpoint
         |
         v
[4] Weight Export (Python)
    Quantizes float weights to int16/int8/int32
    Writes .onnue binary format (magic, header, weights, CRC32)
    Outputs weights_gen0.onnue
```

---

## Step 1: Data Generation — Enhance match.mjs

**Modify `observer/match.mjs`** to add a `--datagen` mode optimized for training data:

```javascript
// New config fields:
{
  "mode": "datagen",           // triggers datagen mode
  "games": 1000,
  "depth": 6,                  // depth 6 for speed (depth 8 for quality)
  "sample_interval": 4,        // sample every Nth ply (randomized +-1)
  "output_format": "jsonl",    // one JSON object per line per position
  "output_file": "training_data_gen0.jsonl"
}
```

**Per-sampled-position output (JSONL):**
```json
{
  "position_moves": "e2e4 j13j11 ...",
  "ply": 24,
  "side_to_move": "Yellow",
  "score_cp": 4312,
  "v1": 0.73, "v2": 0.71, "v3": 0.75, "v4": 0.72,
  "depth": 6,
  "game_id": 42,
  "game_result": [0.0, 1.0, 0.0, 0.0]
}
```

Key rules:
- `game_result` is filled AFTER the game ends: winner gets 1.0, losers get 0.0. Draws: each surviving player gets 1.0/N.
- `score_cp` comes from the `info ... score cp X` line (BRS eval).
- `v1-v4` come from the `info ... v1 X v2 Y v3 Z v4 W` line (MCTS per-player).
- `position_moves` is the move list to reach this position from startpos.
- Sample positions at random intervals (every 4-8 plies) to decorrelate.
- Skip positions where the side_to_move is eliminated.
- Skip the first 4 plies (opening is too uniform).

**Game result mapping:**
- Winner (by elimination of all opponents): `[1, 0, 0, 0]` (Red wins example)
- Draw at ply cap: surviving players share equally, e.g., 2 alive → `[0.5, 0, 0.5, 0]`
- Eliminated player: always 0.0 in that slot

---

## Step 2: Feature Extraction — Rust Datagen Subcommand

**Create `odin-engine/src/datagen.rs`** — A CLI mode that:
1. Reads a JSONL file of training positions
2. For each position, replays the move list to reconstruct the board
3. Computes the 4 active feature vectors (one per perspective)
4. Writes binary training samples

**Binary sample format (.bin):**
```
Per sample (fixed size):
  [4 feature vectors]
    For each of 4 perspectives:
      count: u8 (number of active features, max 64)
      indices: [u16; 64] (padded with 0, only first `count` are valid)
    Total: 4 * (1 + 64*2) = 516 bytes

  [Targets]
    brs_target: i16 (centipawns from search)
    mcts_targets: [f32; 4] (v1-v4 from search, or game_result blend)
    game_result: [f32; 4] (final outcome)
    Total: 2 + 16 + 16 = 34 bytes

  [Metadata]
    ply: u16
    game_id: u32
    Total: 6 bytes

  Sample total: 556 bytes
```

**Add to `odin-engine/src/main.rs`:**
```rust
// If argv contains "--datagen", run datagen mode instead of protocol loop
if args.contains("--datagen") {
    datagen::run(args)?;
    return Ok(());
}
```

**datagen::run() logic:**
1. Parse CLI args: `--input training_data_gen0.jsonl --output training_data_gen0.bin`
2. For each JSONL line:
   - Parse `position_moves` → replay from startpos to reconstruct Board
   - For each of 4 perspectives: call `active_features(board, perspective)`
   - Parse `score_cp` → i16 BRS target
   - Parse `v1-v4` → [f32; 4] MCTS targets
   - Parse `game_result` → [f32; 4]
   - Write binary sample
3. Print summary: total samples, invalid/skipped, output file size

**Why Rust for feature extraction?** The board representation, move replay, and feature indexing already exist in Rust. Reimplementing in Python would be error-prone and slow.

---

## Step 3: PyTorch Training — `odin-nnue/`

**Create directory `odin-nnue/` at project root** with these files:

### `odin-nnue/model.py` — Network Architecture

```python
import torch
import torch.nn as nn

class SCReLU(nn.Module):
    """Squared Clipped ReLU: clamp(x, 0, QA)^2"""
    def __init__(self, qa=255.0):
        super().__init__()
        self.qa = qa

    def forward(self, x):
        return torch.clamp(x, 0.0, self.qa) ** 2

class OdinNNUE(nn.Module):
    def __init__(self, num_features=4480, ft_out=256, hidden=32):
        super().__init__()
        self.num_features = num_features
        self.ft_out = ft_out
        self.qa = 255.0

        # 4 separate feature transformers (one per perspective)
        self.ft = nn.ModuleList([
            nn.Linear(num_features, ft_out) for _ in range(4)
        ])
        self.screlu = SCReLU(self.qa)

        # Hidden layer: 4*256 = 1024 -> 32
        self.hidden = nn.Linear(4 * ft_out, hidden)

        # Dual output heads
        self.brs_head = nn.Linear(hidden, 1)       # BRS scalar
        self.mcts_head = nn.Linear(hidden, 4)      # MCTS 4-player

    def forward(self, features):
        """
        features: list of 4 sparse tensors or dense [batch, 4, 4480]
        Returns: (brs_out, mcts_out)
          brs_out: [batch, 1] raw centipawn prediction
          mcts_out: [batch, 4] raw logits (sigmoid applied in loss)
        """
        # Feature transformer per perspective
        ft_outs = []
        for p in range(4):
            ft_outs.append(self.screlu(self.ft[p](features[:, p, :])))
        # Concatenate: [batch, 1024]
        concat = torch.cat(ft_outs, dim=1)
        # Normalize after SCReLU squaring (divide by QA to keep values reasonable)
        concat = concat / self.qa

        # Hidden layer + ReLU
        h = torch.relu(self.hidden(concat))

        # Output heads
        brs_out = self.brs_head(h)
        mcts_out = self.mcts_head(h)

        return brs_out, mcts_out
```

### `odin-nnue/dataset.py` — Binary Data Loader

```python
import struct
import torch
from torch.utils.data import Dataset

SAMPLE_SIZE = 556  # bytes per sample

class OdinDataset(Dataset):
    def __init__(self, bin_path, num_features=4480):
        self.data = open(bin_path, 'rb').read()
        self.num_samples = len(self.data) // SAMPLE_SIZE
        self.num_features = num_features

    def __len__(self):
        return self.num_samples

    def __getitem__(self, idx):
        offset = idx * SAMPLE_SIZE

        # Parse 4 feature vectors
        features = torch.zeros(4, self.num_features)
        for p in range(4):
            base = offset + p * 129  # 1 + 64*2 = 129 bytes per perspective
            count = self.data[base]
            for i in range(count):
                feat_idx = struct.unpack_from('<H', self.data, base + 1 + i * 2)[0]
                features[p, feat_idx] = 1.0

        target_offset = offset + 516  # after 4 feature vectors

        # BRS target (i16)
        brs_target = struct.unpack_from('<h', self.data, target_offset)[0]

        # MCTS targets (4 x f32)
        mcts_targets = struct.unpack_from('<4f', self.data, target_offset + 2)

        # Game result (4 x f32)
        game_result = struct.unpack_from('<4f', self.data, target_offset + 18)

        return (
            features,
            torch.tensor(brs_target, dtype=torch.float32),
            torch.tensor(mcts_targets, dtype=torch.float32),
            torch.tensor(game_result, dtype=torch.float32),
        )
```

### `odin-nnue/train.py` — Training Loop

```python
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, random_split
from model import OdinNNUE
from dataset import OdinDataset

# Hyperparameters
BATCH_SIZE = 4096
LR = 0.01
EPOCHS = 20
LAMBDA_BRS = 1.0
LAMBDA_MCTS = 0.5
LAMBDA_RESULT = 0.25
SIGMOID_K = 4000.0

def train():
    dataset = OdinDataset('training_data_gen0.bin')
    train_size = int(0.9 * len(dataset))
    val_size = len(dataset) - train_size
    train_set, val_set = random_split(dataset, [train_size, val_size])

    train_loader = DataLoader(train_set, batch_size=BATCH_SIZE, shuffle=True, num_workers=4)
    val_loader = DataLoader(val_set, batch_size=BATCH_SIZE, shuffle=False, num_workers=4)

    model = OdinNNUE()
    optimizer = torch.optim.Adam(model.parameters(), lr=LR)
    scheduler = torch.optim.lr_scheduler.StepLR(optimizer, step_size=5, gamma=0.5)

    best_val_loss = float('inf')
    patience = 5
    patience_counter = 0

    for epoch in range(EPOCHS):
        model.train()
        total_loss = 0
        for features, brs_target, mcts_target, game_result in train_loader:
            brs_pred, mcts_pred = model(features)

            # BRS loss: MSE in centipawn scale (normalize by OUTPUT_SCALE)
            brs_loss = nn.functional.mse_loss(
                brs_pred.squeeze() / 400.0,
                brs_target / 400.0
            )

            # MCTS loss: MSE between sigmoid predictions and blended target
            # Blend: 70% search value + 30% game result
            mcts_blended = 0.7 * mcts_target + 0.3 * game_result
            mcts_pred_sigmoid = torch.sigmoid(mcts_pred / SIGMOID_K)
            mcts_loss = nn.functional.mse_loss(mcts_pred_sigmoid, mcts_blended)

            # Game result loss: how well does the model predict game outcomes?
            result_pred = torch.sigmoid(mcts_pred / SIGMOID_K)
            result_loss = nn.functional.mse_loss(result_pred, game_result)

            loss = LAMBDA_BRS * brs_loss + LAMBDA_MCTS * mcts_loss + LAMBDA_RESULT * result_loss

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()

            total_loss += loss.item()

        # Validation
        model.eval()
        val_loss = 0
        with torch.no_grad():
            for features, brs_target, mcts_target, game_result in val_loader:
                brs_pred, mcts_pred = model(features)
                brs_loss = nn.functional.mse_loss(brs_pred.squeeze() / 400, brs_target / 400)
                mcts_blended = 0.7 * mcts_target + 0.3 * game_result
                mcts_pred_sigmoid = torch.sigmoid(mcts_pred / SIGMOID_K)
                mcts_loss = nn.functional.mse_loss(mcts_pred_sigmoid, mcts_blended)
                result_pred = torch.sigmoid(mcts_pred / SIGMOID_K)
                result_loss = nn.functional.mse_loss(result_pred, game_result)
                val_loss += (LAMBDA_BRS * brs_loss + LAMBDA_MCTS * mcts_loss + LAMBDA_RESULT * result_loss).item()

        avg_train = total_loss / len(train_loader)
        avg_val = val_loss / len(val_loader)
        print(f"Epoch {epoch+1}/{EPOCHS}  train_loss={avg_train:.6f}  val_loss={avg_val:.6f}  lr={scheduler.get_last_lr()[0]:.6f}")

        # Early stopping
        if avg_val < best_val_loss:
            best_val_loss = avg_val
            patience_counter = 0
            torch.save(model.state_dict(), 'best_model.pt')
        else:
            patience_counter += 1
            if patience_counter >= patience:
                print(f"Early stopping at epoch {epoch+1}")
                break

        scheduler.step()

    print(f"Best validation loss: {best_val_loss:.6f}")
    print("Model saved to best_model.pt")

if __name__ == '__main__':
    train()
```

### `odin-nnue/export.py` — PyTorch → .onnue Export

```python
import struct
import numpy as np
import torch
from model import OdinNNUE

# Must match Rust constants exactly
ONNUE_MAGIC = b'ONUE'
ONNUE_VERSION = 1
QA = 255
FEATURES = 4480
FT_OUT = 256
HIDDEN = 32

def architecture_hash():
    """Must produce the same 32-byte hash as the Rust implementation."""
    descriptor = f"HalfKP4-{FEATURES}-{FT_OUT}-{HIDDEN}-1-4"
    desc_bytes = descriptor.encode('ascii')
    result = bytearray(32)
    for chunk_idx in range(4):
        h = (0xcbf29ce484222325 + chunk_idx) & 0xFFFFFFFFFFFFFFFF
        for b in desc_bytes:
            h ^= b
            h = (h * 0x00000100000001b3) & 0xFFFFFFFFFFFFFFFF
        result[chunk_idx*8:chunk_idx*8+8] = h.to_bytes(8, 'little')
    return bytes(result)

def crc32_ieee(data):
    """CRC32 matching the Rust implementation."""
    import binascii
    return binascii.crc32(data) & 0xFFFFFFFF

def quantize_ft(weight, bias, qa=QA):
    """Quantize feature transformer: float -> int16."""
    # Scale to use int16 range effectively
    w_max = max(weight.abs().max().item(), 1e-6)
    scale = min(qa / w_max, 32767.0 / w_max)

    w_q = torch.clamp(torch.round(weight * scale), -32768, 32767).to(torch.int16)
    b_q = torch.clamp(torch.round(bias * scale), -32768, 32767).to(torch.int16)
    return w_q, b_q

def quantize_hidden(weight, bias):
    """Quantize hidden layer: float -> int8 weights, int32 biases."""
    w_max = max(weight.abs().max().item(), 1e-6)
    scale = min(127.0 / w_max, 127.0)

    w_q = torch.clamp(torch.round(weight * scale), -128, 127).to(torch.int8)
    b_q = torch.round(bias * scale * 64).to(torch.int32)  # scale by QB=64
    return w_q, b_q

def export(model_path, output_path):
    model = OdinNNUE()
    model.load_state_dict(torch.load(model_path, map_location='cpu', weights_only=True))
    model.eval()

    buf = bytearray()

    # Header (48 bytes)
    buf += ONNUE_MAGIC
    buf += struct.pack('<I', ONNUE_VERSION)
    buf += architecture_hash()
    buf += struct.pack('<I', FEATURES)
    buf += struct.pack('<I', FT_OUT)

    # Feature transformer (4 perspectives)
    for p in range(4):
        w = model.ft[p].weight.detach()  # [256, 4480]
        b = model.ft[p].bias.detach()    # [256]
        w_q, b_q = quantize_ft(w, b)
        # Write as [4480][256] (transpose from PyTorch's [out, in])
        for feat in range(FEATURES):
            for neuron in range(FT_OUT):
                buf += struct.pack('<h', w_q[neuron, feat].item())
        for neuron in range(FT_OUT):
            buf += struct.pack('<h', b_q[neuron].item())

    # Hidden layer
    w = model.hidden.weight.detach()   # [32, 1024]
    b = model.hidden.bias.detach()     # [32]
    w_q, b_q = quantize_hidden(w, b)
    # Write as [1024][32]
    for inp in range(4 * FT_OUT):
        for neuron in range(HIDDEN):
            buf += struct.pack('<b', w_q[neuron, inp].item())
    for neuron in range(HIDDEN):
        buf += struct.pack('<i', b_q[neuron].item())

    # BRS head
    w = model.brs_head.weight.detach()  # [1, 32]
    b = model.brs_head.bias.detach()    # [1]
    w_q, b_q = quantize_hidden(w, b)
    for h in range(HIDDEN):
        buf += struct.pack('<b', w_q[0, h].item())
    buf += struct.pack('<i', b_q[0].item())

    # MCTS head
    w = model.mcts_head.weight.detach()  # [4, 32]
    b = model.mcts_head.bias.detach()    # [4]
    w_q, b_q = quantize_hidden(w, b)
    # Write as [32][4]
    for h in range(HIDDEN):
        for v in range(4):
            buf += struct.pack('<b', w_q[v, h].item())
    for v in range(4):
        buf += struct.pack('<i', b_q[v].item())

    # CRC32 footer
    checksum = crc32_ieee(bytes(buf))
    buf += struct.pack('<I', checksum)

    with open(output_path, 'wb') as f:
        f.write(buf)

    print(f"Exported {output_path} ({len(buf)} bytes)")
    print(f"CRC32: {checksum:#010x}")

if __name__ == '__main__':
    import sys
    model_path = sys.argv[1] if len(sys.argv) > 1 else 'best_model.pt'
    output_path = sys.argv[2] if len(sys.argv) > 2 else 'weights_gen0.onnue'
    export(model_path, output_path)
```

### `odin-nnue/requirements.txt`
```
torch>=2.0
numpy
```

---

## Step 4: Gen-0 Training Run

### Data Generation (run on your machine)

```bash
# 1. Generate 1000 games with bootstrap eval at depth 6
cd observer
node match.mjs datagen_config.json

# 2. Extract features from JSONL positions
cd ../odin-engine
cargo run --release -- --datagen --input ../observer/training_data_gen0.jsonl --output ../odin-nnue/training_data_gen0.bin

# 3. Train
cd ../odin-nnue
pip install -r requirements.txt
python train.py

# 4. Export to .onnue
python export.py best_model.pt weights_gen0.onnue

# 5. Verify the weights load in Rust
cd ../odin-engine
cargo test -- test_load_gen0_weights
```

### `observer/datagen_config.json`
```json
{
  "mode": "datagen",
  "engine_a": "../odin-engine/target/release/odin-engine",
  "engine_b": "../odin-engine/target/release/odin-engine",
  "games": 1000,
  "depth": 6,
  "sample_interval": 4,
  "output_file": "training_data_gen0.jsonl",
  "game_mode": "ffa",
  "eval_profile": "aggressive",
  "stop_at": {
    "max_ply": 200,
    "on_gameover": true
  }
}
```

---

## Build Order

**Step 1: datagen_config.json + match.mjs datagen mode**
- Add datagen mode to match.mjs (sample positions, write JSONL, backfill game_result)
- Test: run 5 games, verify JSONL output is well-formed

**Step 2: Rust datagen subcommand**
- Create `odin-engine/src/datagen.rs`
- Wire into `main.rs` (--datagen flag)
- Replay positions from JSONL, extract features, write .bin
- Test: process the 5-game JSONL, verify .bin is correct size

**Step 3: PyTorch model + dataset**
- Create `odin-nnue/` directory with model.py, dataset.py
- Test: load a .bin file, verify tensor shapes, run a forward pass

**Step 4: Training loop**
- Create train.py with multi-task loss, validation, early stopping
- Test: train for 2 epochs on the small dataset, verify loss decreases

**Step 5: Export script**
- Create export.py
- Test: export trained model → .onnue, load in Rust, verify no errors

**Step 6: Integration test**
- Load exported .onnue weights into NnueEvaluator in Rust
- Run eval_scalar and eval_4vec on starting position
- Verify outputs are in valid ranges and differ from random weights

**Step 7: Full Gen-0 run (optional — may need human to run)**
- Generate 1000 games (this takes time — ~hours at depth 6)
- Extract features
- Train
- Export

---

## Acceptance Criteria (Tests Required)

### Rust Tests (in `odin-engine/tests/stage_15_datagen.rs`)

| ID | Test | What it verifies |
|----|------|-----------------|
| T1 | `test_datagen_replay_startpos` | Replaying empty move list gives starting position |
| T2 | `test_datagen_replay_moves` | Replaying known move sequence gives correct board |
| T3 | `test_datagen_feature_extraction` | Extracted features match active_features() for replayed position |
| T4 | `test_datagen_binary_roundtrip` | Write sample → read back → all fields match |
| T5 | `test_datagen_skips_eliminated` | Positions where side_to_move is eliminated are skipped |

### Python Tests (in `odin-nnue/test_pipeline.py`)

| ID | Test | What it verifies |
|----|------|-----------------|
| T6 | `test_model_forward_shape` | OdinNNUE produces correct output shapes |
| T7 | `test_dataset_loading` | OdinDataset reads .bin correctly, returns valid tensors |
| T8 | `test_loss_computation` | Multi-task loss computes without NaN/Inf |
| T9 | `test_export_magic` | Exported .onnue starts with "ONUE" magic |
| T10 | `test_export_architecture_hash` | Hash matches Rust implementation |
| T11 | `test_export_roundtrip` | Export → Rust load succeeds (integration test) |
| T12 | `test_training_loss_decreases` | 5 epochs on small data: final loss < initial loss |

### Integration Test (Rust, after export)

| ID | Test |
|----|------|
| T13 | `test_load_exported_weights` | Load .onnue from export.py, eval_scalar returns valid range |

---

## Critical Invariants — DO NOT VIOLATE

1. **Weight layout must match Stage 14 exactly.** The .onnue file format is defined — do not change it.
2. **Architecture hash must match.** Python `architecture_hash()` must produce identical 32 bytes as Rust's.
3. **CRC32 must match.** Python CRC32 must be IEEE 802.3, same polynomial as Rust.
4. **Feature index formula is fixed:** `dense_sq * 28 + piece_type * 4 + relative_owner`. Don't change.
5. **Quantization order:** FT weights stored as `[perspective][feature][neuron]`, hidden as `[input][neuron]`.
6. **Turn order R→B→Y→G** — relative owner rotation must match Rust.
7. **SIGMOID_K = 4000.0** — must be consistent between training and inference.
8. **OUTPUT_SCALE = 400** — must be consistent.
9. **Do NOT modify any Rust eval/search code** except adding the datagen subcommand.
10. **perft invariants are permanent.** If they change, you broke something.

---

## Scope Boundaries — What NOT To Build

- Search integration (Stage 16)
- Distributed training (single machine is fine)
- Online learning during play (batch training only)
- SIMD optimization (Stage 19)
- King bucketing Phase 2 (deferred)
- Policy head (deferred)
- Gen-1+ training (Gen-0 only for this stage)

---

## Notes on Data Quality

- **Depth 6 is minimum for useful BRS targets.** Depth 8 is better but 4x slower. Start with depth 6, upgrade later.
- **The v1-v4 values from bootstrap eval will be compressed** (~0.70-0.78 range due to SIGMOID_K=4000 and bootstrap eval range). This is expected — the NNUE will learn its own scale.
- **Game results are the strongest signal.** The BRS/MCTS targets from bootstrap eval are noisy but correlated. The game_result is ground truth. The 70/30 blend in the loss function weights search targets more than outcome because we want the NNUE to mimic the search, not just predict winners.
- **1000 games x ~50 positions/game = ~50K training samples** for Gen-0. This is small but sufficient for a first-generation net. Gen-1+ can scale up.

---

## Pre-Audit Checklist

1. `cargo build && cargo test` — verify 519 tests pass
2. `cargo clippy` — verify 0 warnings
3. Create `masterplan/audit_log_stage_15.md` with pre-audit section

## Post-Audit Checklist

1. `cargo test` — all existing 519 + new Rust tests pass
2. `cargo clippy` — 0 warnings
3. `python -m pytest odin-nnue/test_pipeline.py` — all Python tests pass
4. Fill post-audit section of `audit_log_stage_15.md`
5. Create `masterplan/downstream_log_stage_15.md`
6. Update `masterplan/STATUS.md` and `masterplan/HANDOFF.md`
7. Create session note in `masterplan/sessions/`
