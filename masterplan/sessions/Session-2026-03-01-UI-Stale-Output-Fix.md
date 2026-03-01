# Session: UI Stale Engine Output Fix

**Date:** 2026-03-01
**Type:** Non-stage bugfix
**Issues resolved:** [[Issue-UI-Stale-Engine-Output]], [[Issue-UI-EP-Display-Lateral]]

## Summary

Fixed three UI bugs: stale engine output corrupting game state on restart, stale bestmove on New Game, and en passant display for 4-player lateral captures. Also carried forward MCTS override fixes from a prior session.

## Changes

### Rust: Engine Generation Tagging (`engine.rs`, `lib.rs`)

- Added `generation: u64` to `EngineManager`, bumped on every `spawn()`
- Created `EngineOutputPayload { line: String, gen: u64 }` struct
- Reader thread captures generation at spawn time, tags every event
- `spawn()` returns `Result<u64, String>` so frontend knows current gen

### TypeScript: Event Filtering (`useEngine.ts`)

- Listens for `EngineOutputPayload` instead of plain string
- Stores `engineGenRef` from `invoke('spawn_engine')` return value
- Discards all events where `gen !== engineGenRef.current`
- Simplified engine-exit handler (compare gen directly)

### TypeScript: New Game Stale Bestmove (`useGameState.ts`)

- Added `ignoreNextBestmoveRef` flag
- `newGame()` sends `stop` + sets flag when search is in flight
- `info`, `nextturn`, `bestmove` handlers check flag before processing
- Flag consumed when stale bestmove arrives; new search works normally

### TypeScript: En Passant Display (`useGameState.ts`)

- `applyMoveToBoard` now checks both candidate capture squares:
  - `(toFile, fromRank)` — for vertical-moving captured pawns
  - `(fromFile, toRank)` — for lateral-moving captured pawns (Green/Blue)
- Validates candidate has opponent pawn before removing

### Rust: MCTS Override Fixes (`hybrid.rs`, carried from prior session)

- `BRS_CONFIDENCE_MARGIN` lowered to 25cp
- `MCTS_OVERRIDE_TOLERANCE` of 30cp added
- Mate detection at score >= 9000

## Root Cause Analysis

The stale output bug had two manifestation paths:

1. **Engine restart path:** `spawn()` kills the old process via `child.kill()` + `child.wait()`, but the reader thread (spawned with `thread::spawn`) is NOT joined. It continues draining buffered stdout and emitting events with the same event name. Fixed at Rust level with generation tagging.

2. **New Game path:** `newGame()` reuses the same engine process. If a `go` command was in flight, the engine's bestmove arrives after state is reset and gets processed as the new game's first move. Fixed at JS level with `ignoreNextBestmoveRef` + `stop`.

A JS-only generation counter approach was attempted first but failed: the stale bestmove arrived after `sendGoFromRef` set `awaitingGenRef = gameGenRef`, so the guard passed.

## Kaggle Training Pipeline Setup

Also in this session, set up infrastructure for GPU-accelerated NNUE training on Kaggle:

- Fixed `observer/datagen_config.json` engine path
- Created `odin-nnue/kaggle_train.ipynb` — self-contained notebook bundling model, dataset, training, and .onnue export. GPU-enabled with CUDA device detection.
- Created `masterplan/patterns/Pattern-Kaggle-Training-Pipeline.md` — full step-by-step docs for the local→Kaggle→local workflow.

The pipeline: local datagen (match.mjs) → local feature extraction (odin-engine --datagen) → upload .bin to Kaggle → GPU training → download .pt + .onnue.
