---
type: session
date: 2026-02-20
stage: 5
tags:
  - stage/05
  - tier/foundation
---

# Session: 2026-02-20 -- Stage 05 Basic UI Shell

## Goal

Implement Stage 5: Basic UI Shell. Build a Tauri v2 desktop application with SVG board rendering, engine subprocess communication, click-to-move, debug console, and game controls.

## What Happened

1. Followed Stage Entry Protocol: read STATUS, HANDOFF, DECISIONS, stage spec, all upstream audit and downstream logs (stages 0-4).
2. Created git tags: `stage-04-complete` / `v1.4`.
3. Filled pre-audit section of `audit_log_stage_05.md` with upstream findings and risk analysis.
4. Asked user about desktop framework choice. Decision: Tauri v2 with Rust backend spawning engine subprocess.
5. Created comprehensive implementation plan (5 build steps matching spec).
6. Implemented all 5 build steps:
   - **Step 1:** Tauri v2 scaffolding (src-tauri/), SVG board renderer (BoardDisplay/BoardSquare/PieceIcon), board types and constants
   - **Step 2:** Engine subprocess management (engine.rs), IPC commands, protocol parser, useEngine hook
   - **Step 3:** Click-to-move flow (useGameState hook), display-side board updates
   - **Step 4:** Debug console with color-coded log and parsed info summary
   - **Step 5:** Game controls, status bar, layout wiring
7. Fixed build issues: workspace conflict (excluded src-tauri from root workspace), missing icon.ico (generated minimal ICO), lib crate name mismatch.
8. Verified: Rust backend compiles, TypeScript compiles, all 229 engine tests pass.
9. Wrote 45 Vitest unit tests (29 board-constants + 16 protocol-parser). All pass.
10. Created 7 git commits following the commit plan.
11. Filled post-audit, downstream log, vault notes.

## Components Touched

- [[Component-BasicUI]] (new) — entire UI shell
- [[Component-Protocol]] — no changes, but heavily referenced for parser implementation
- Root `Cargo.toml` — added `exclude = ["odin-ui/src-tauri"]`

## Discoveries

1. **Tauri v2 init requires interactive terminal.** Had to create all scaffolding files manually since `tauri init` failed in non-TTY environment.
2. **Tauri build requires icon.ico on Windows** regardless of tauri.conf.json icon list. Generated minimal 1x1 ICO with Python.
3. **Workspace conflict:** src-tauri's Cargo.toml was auto-detected as workspace member. Required explicit `exclude` in root workspace config.
4. **King positions differ between Red/Green and Blue/Yellow.** Red and Green use `R N B Q K B N R` (King at index 4), Blue and Yellow use `R N B K Q B N R` (King at index 3). Important for test accuracy.

## Issues Created/Resolved

- Created: [[Issue-DKW-Invisible-Moves-UI]] — DKW king instant moves not visible in UI rendering cache. Accepted limitation for Stage 5.
- Existing issues unchanged: [[Issue-Perft-Values-Unverified]], [[Issue-DKW-Halfmove-Clock]].
