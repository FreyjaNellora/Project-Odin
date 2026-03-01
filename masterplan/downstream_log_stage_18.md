# Downstream Log — Stage 18: Full UI

## Notes for Future Stages

### Must-Know

- **W31:** `gameWinner: Player | null` in useGameState — null means both "no game over" and "draw". Disambiguated by `isGameOver` boolean. A discriminated union would be cleaner but isn't needed for current usage.
- **W32:** Undo past eliminations doesn't restore eliminated player state. Kings remain removed from display board, `eliminatedPlayersRef` isn't rolled back. Would need engine cooperation to fix properly. Acceptable for dev tool.
- **PlayMode removed.** The `PlayMode` type (`manual`/`semi-auto`/`full-auto`) and `humanPlayer` state are gone. Replaced by `SlotConfig = Record<Player, 'human' | 'engine'>`. Any code referencing `PlayMode` or `humanPlayer` needs updating to use `slotConfig`.
- **P2 features intentionally deferred.** Move arrows, check highlight, terrain square styling, and FEN4 parser are NOT implemented in the Tauri UI. This is a deliberate prioritization decision — the Tauri UI is a dev tool, and these visual features will be rebuilt for the web platform. Future agents should NOT treat these as bugs or regressions.

### API Contracts

- **New engine `info string` emissions (Stage 18):**
  - `info string in_check <color>` — emitted after position/move when next player is in check
  - `info string brs_moves <move:score> [...]` — emitted after BRS phase 1 with surviving moves and scores
  - `info string mcts_visits <move:visits> [...]` — emitted before bestmove with top-5 MCTS root children
  - `info string stop_reason <time|depth|nodes|forced|complete|time_pressure|brs_confidence>` — emitted before bestmove
- **Parser handles all new emissions.** `parseEngineMessage()` returns `{ type: 'in_check', player }` or `{ type: 'info', data: { brsMoves, mctsVisits, stopReason } }`.
- **`useGameState` exports:** `SlotConfig`, `SlotRole`, `DEFAULT_SLOT_CONFIG`, `GameMode`, `EvalProfileSetting`. No longer exports `PlayMode`.
- **`useSelfPlay` hook:** Takes `UseGameStateResult`, returns `UseSelfPlayResult` with start/stop/reset/stats.

### Known Limitations

- Self-play dashboard requires engine to be connected (Start button disabled otherwise).
- Self-play saves/restores slot config and engine delay, but not game mode, eval profile, terrain, or chess960 settings.
- Undo/redo are disabled during active search (`awaitingBestmoveRef`). User must wait for bestmove before undoing.
- `replayMoveOnBoard` duplicates board mutation logic from `applyMoveToBoard`. Changes to board display logic need to be applied in both places.
- Self-play `setTimeout(50ms)` before `newGame()` is a timing hack to let React flush slot config state.

### Performance Baselines

- Engine protocol: `info string` emissions add negligible overhead (~1 format! call per search).
- UI: 63 Vitest tests (was 56 at Stage 17 entry, +7 protocol parser tests).
- Self-play can run 10+ games sequentially without crashes (structural analysis; AC4 manual verification recommended).

### Open Questions

- Should self-play save/restore ALL game settings (mode, eval, terrain, chess960) or just slot config + delay? Current: just slot config + delay.
- Should undo attempt to reconstruct eliminated player state? Would require engine to emit elimination info on position commands.

### Reasoning

**Why defer P2 visual features?** The Tauri UI is a development tool for testing the engine during training. The real product will be a web platform with its own frontend. Investing in move arrows, check highlights, terrain styling, and FEN4 parsing in the Tauri UI has diminishing returns — all would need to be rebuilt. Engine-side protocol extensions (P0-A) are permanent investment that benefits any frontend.

**Why per-slot config instead of play modes?** Per-slot config (`Record<Player, 'human' | 'engine'>`) is strictly more expressive than the old 3-mode system. "Play as Red" = Red human + rest engine. "Watch" = all engine. "Hot Seat" = all human. Plus any custom combination. The old `PlayMode` + `humanPlayer` was two separate state variables encoding the same information less clearly.

---

## Carried Warnings

- **W26:** DKW chance nodes in MCTS skipped.
- **W27:** FFA self-stalemate detection skipped.
- **W28:** Chess960 FEN notation not addressed.
- **W29:** Castling make/unmake uses atomic remove-both-then-place for Chess960.
- **W30:** Board::empty() initializes castling_starts with standard values.
- **W18 (carried):** King moves mark `needs_refresh` even without king bucketing.
- **W19 (carried):** EP/castling fall back to full refresh.
- **W20 (carried):** `serde` + `serde_json` in engine (datagen CLI path only).
- **W13 (carried):** MCTS score 9999 (max) in some positions.
- **Pondering not implemented:** Deferred from Stage 13.

---

## Related

- Stage spec: [[stage_18_full_ui]]
- Audit log: [[audit_log_stage_18]]
