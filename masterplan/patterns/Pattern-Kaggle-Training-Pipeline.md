---
type: pattern
stage_introduced: 15
tags:
  - stage/15
  - area/nnue
  - area/training
status: active
last_updated: 2026-03-01
---

# Pattern: Kaggle GPU Training Pipeline

## Context

NNUE training requires GPU acceleration. The user's local machine can handle self-play data generation but not efficient PyTorch training. Kaggle provides free GPU (T4/P100) with 30h/week quota.

## Pipeline Overview

The training pipeline is split across local and Kaggle:

```
LOCAL                          KAGGLE
─────                          ──────
1. Self-play datagen           3. GPU training
   (match.mjs)                    (kaggle_train.ipynb)
        ↓                              ↓
2. Feature extraction          4. Download outputs
   (odin-engine --datagen)        (.pt + .onnue)
        ↓                              ↓
   Upload .bin ──────────→     Upload as Dataset
                                        ↓
                               5. Place .onnue in
                                  odin-engine/weights/
```

## Step-by-Step

### Step 1: Self-Play Data Generation (Local)

```bash
cd observer
node match.mjs datagen_config.json
```

Config: `observer/datagen_config.json`
- `mode`: "datagen"
- `engine_a`: path to release engine binary
- `games`: number of self-play games (1000 for Gen-0)
- `depth`: search depth per move (8 recommended)
- `sample_interval`: sample every N plies (4 recommended)
- `output_file`: JSONL output path

Produces: `training_data_gen0.jsonl` (one JSON per sampled position)

**Time estimate**: ~2-5 min/game at depth 8 on mid-range PC. 1000 games = ~30-80 hours.

### Step 2: Feature Extraction (Local)

```bash
target/release/odin-engine --datagen --input observer/training_data_gen0.jsonl --output odin-nnue/training_data_gen0.bin
```

Converts JSONL positions to binary HalfKP-4 features. Each sample = 556 bytes.

**Time estimate**: Fast (~seconds for 25K samples).

### Step 3: Upload & Train on Kaggle

1. Go to kaggle.com > Datasets > New Dataset
2. Upload `training_data_gen0.bin` as dataset "odin-training-data"
3. Go to kaggle.com > Notebooks > New Notebook
4. Upload `odin-nnue/kaggle_train.ipynb` (or paste cells)
5. Add the uploaded dataset
6. Enable GPU accelerator (Settings > Accelerator > GPU)
7. Update `BIN_PATH` in cell 1 to match dataset path
8. Run All

The notebook handles: model creation, training with early stopping, export to `.onnue`.

**Time estimate**: ~5-15 minutes for 25K samples on T4 GPU.

### Step 4: Download Outputs

From Kaggle `/kaggle/working/`:
- `best_model.pt` — PyTorch state dict (for future fine-tuning)
- `weights_gen0.onnue` — quantized binary (for Rust engine)

### Step 5: Deploy Weights

Place `.onnue` file where the engine can find it:
```
odin-engine/weights/weights_gen0.onnue
```

The engine loads via `EngineOptions::nnue_file` or the default weight path.

## Files

| File | Location | Purpose |
|------|----------|---------|
| `datagen_config.json` | `observer/` | Self-play datagen config |
| `match.mjs` | `observer/` | Match runner with datagen mode |
| `datagen.rs` | `odin-engine/src/` | JSONL → binary feature extraction |
| `kaggle_train.ipynb` | `odin-nnue/` | Self-contained Kaggle training notebook |
| `model.py` | `odin-nnue/` | OdinNNUE architecture (local reference) |
| `dataset.py` | `odin-nnue/` | Binary dataset loader (local reference) |
| `train.py` | `odin-nnue/` | Training loop (local reference) |
| `export.py` | `odin-nnue/` | .onnue export (local reference) |

## Key Constants (Must Match Across Python/Rust)

- Features: 4,480 (HalfKP-4: 160 squares x 7 pieces x 4 relative owners)
- FT output: 256 per perspective
- Hidden: 32 neurons
- Architecture hash: FNV-1a of "HalfKP4-4480-256-32-1-4"
- Sample size: 556 bytes
- SIGMOID_K: 4000.0
- OUTPUT_SCALE: 400.0

## Generational Training

After Gen-0:
1. Run self-play with Gen-0 weights loaded (`--nnue weights_gen0.onnue`)
2. Generate Gen-1 JSONL and .bin
3. Train on Kaggle with combined Gen-0 + Gen-1 data (or just Gen-1)
4. Export `weights_gen1.onnue`
5. Repeat

Each generation should produce stronger play as the NNUE learns from better self-play data.

## Related

- [[Component-Search]] — BRS/MCTS use NNUE eval
- [[Component-Eval]] — Bootstrap eval is the fallback
- [[MASTERPLAN]] Stages 14-16 — NNUE design, training, integration
