---
type: session
tags:
  - type/session
  - stage/9
  - topic/search
  - topic/eval
  - topic/debugging
date: 2026-02-27
---

# Session 2026-02-27 — Engine Game Analysis Fixes

## Context

User ran a self-play game (11 moves per player) and identified 3 bugs in engine play. Previous session ([[Session-2026-02-26-BRS-Architecture-Investigation]]) had diagnosed root causes and created fix instructions. This session implemented all 4 priority fixes from the analysis.

## What Was Done

### 1. Game Analysis & Diagnosis

Full root cause analysis of 3 bugs + 4 additional issues from `engine_game_analysis_prompt.md`:

| Bug | Symptom | Root Cause | Severity |
|---|---|---|---|
| Bug 1 | Green exposes back-rank rook | Depth asymmetry + paranoid modeling + no undefended-piece signal | Major |
| Bug 2 | Blue pushes 7 undefended pawns | PST pawn gradient + paranoid modeling discourages piece development | Major |
| Bug 3 | Red knight undevelops Ni3→j1 | Phantom threats from 70% paranoid likelihood | Major |
| Issue 4 | All players too passive | Same paranoid modeling root cause | Major |
| Issue 5 | PV instability at depth 7 | Noisy evaluation landscape, TT staleness | Minor |
| Issue 6 | Stale board context during search | `scan_board()` runs once pre-search; context outdated at deep nodes | Minor (structural) |
| Issue 7 | Arbitrary fallback in select_hybrid_reply | `moves.first()` when no relevant moves exist | Minor |

### 2. Likelihood Tuning (Priority 1)

**File:** `board_scanner.rs`

Tuned hybrid likelihood constants to reduce paranoid opponent modeling:
- `LIKELIHOOD_BASE_TARGETS_ROOT`: 0.7 → 0.5
- `LIKELIHOOD_EXPOSED_PENALTY`: 0.3 → 0.5
- `LIKELIHOOD_BASE_NON_ROOT`: 0.2 → 0.3

Effect: shifts paranoid/realistic blend from ~80/20 to ~50/50. Opponents modeled as attacking root only when board context justifies it.

### 3. Hybrid Reply Fallback (Priority 2)

**File:** `board_scanner.rs`

Added `pick_objectively_strongest()` function: when no opponent moves are classified as relevant, eval-based selection picks the move that minimizes root's static eval. Replaces arbitrary `moves.first()`.

### 4. TT Player-Awareness (Priority 3)

**Files:** `zobrist.rs`, `brs.rs`

- Added `root_player: [u64; 4]` to `ZobristKeys` struct with accessor `root_player_key()`
- In `alphabeta()`: `tt_hash = hash ^ root_player_key(self.root_player.index())`
- TT probe/store uses `tt_hash`; repetition detection uses raw `hash`
- Prevents TT entry contamination across different root-player searches

### 5. TT Persistence (Priority 4)

**Files:** `brs.rs`, `protocol/mod.rs`

- `BrsSearcher` stored as `Option<BrsSearcher>` in `OdinEngine`, lazily created on first `go`
- Added `set_info_callback()` method to replace output buffer per search call
- TT survives across `go` commands; `search()` already increments generation

### 6. Root TT Probe Safety (Bonus Fix)

**File:** `brs.rs`

Discovered during testing: likelihood tuning changed search tree enough to trigger a latent bug in aspiration windows. When the initial narrow-window search stored a TT_LOWER entry and the re-search probed the same position, the TT probe tightened alpha to a value no move could beat, leaving PV empty at depths 5-6.

Fix: at ply 0, TT probe only returns move hint (for ordering), never adjusts alpha/beta or returns cutoff scores. Root is always fully searched.

## Issues Resolved

- [[Issue-BRS-Paranoid-Opponent-Modeling]] — **RESOLVED** (likelihood tuning)
- [[Issue-TT-Not-Player-Aware]] — **RESOLVED** (root_player Zobrist key)
- [[Issue-TT-Fresh-Per-Search]] — **RESOLVED** (persistent BrsSearcher)

## Test Results

389 engine tests pass (246 unit + 143 integration, 3 ignored). 0 warnings.

## Key Insight

The root TT probe bug was **latent** — it existed before this session but never triggered because the old likelihood constants (0.7 paranoid) produced search trees where aspiration windows rarely failed. The new constants (0.5) changed the evaluation landscape enough that depth-5 scores differed significantly from depth-4, causing aspiration failures that exposed the TT-alpha tightening issue. This is a good example of why tuning search parameters can surface bugs in seemingly unrelated code.
