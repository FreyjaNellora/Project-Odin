# Downstream Log — Stage 17: Game Mode Variant Tuning

## Notes for Future Stages

### Must-Know

- **W26 (new):** DKW chance nodes in MCTS skipped. Random DKW king moves have negligible strategic impact (one king move among 2-5 options on a mostly-empty zone). Expectimax approach would cost 3-5x per DKW simulation for minimal gain. Documented, no code changes.
- **W27 (new):** FFA self-stalemate detection skipped. Too complex for marginal gain. Future improvement if needed.
- **W28 (new):** Chess960 FEN notation not addressed. `position startpos` is the primary entry point. FEN castling notation changes (file letters instead of KQkq) deferred. FEN4-loaded boards always use standard castling_starts.
- **W29 (new):** castling make/unmake uses atomic remove-both-then-place pattern for Chess960 compatibility. If adding new castling variants, follow the same pattern (never `move_piece` for castling).
- **W30 (new):** `Board::empty()` initializes `castling_starts` with standard values (not zeros). Any code creating boards via `Board::empty()` gets standard castling starts by default.
- **W18 (carried):** King moves still mark `needs_refresh` even without king bucketing. Profile in Stage 19.
- **W19 (carried):** EP/castling fall back to full refresh. Profile in Stage 19.
- **W20 (carried):** `serde` + `serde_json` only in datagen CLI path. Do NOT import serde in eval/search hot path.

### API Contracts

- **`Board::chess960_position(seed: u64) -> Board`** — generates Chess960 starting position from deterministic seed. All 4 players get same logical arrangement (Red/Green same orientation, Blue/Yellow reversed). Sets `castling_starts` based on actual piece positions.
- **`Board::castling_starts() -> &[(Square, Square, Square); 4]`** — returns (king_start, ks_rook, qs_rook) per player. Always initialized (standard values for non-Chess960 boards).
- **`Board::set_castling_starts(player, king, ks_rook, qs_rook)`** — set castling start squares for a player.
- **`chess960::generate_back_rank(seed: u64) -> [PieceType; 8]`** — pure function, deterministic.
- **`chess960::is_valid_chess960(rank: &[PieceType; 8]) -> bool`** — validation function.
- **`compute_priors(moves, temperature, board)` in mcts.rs** — now requires `board: &Board` parameter for dead piece status check.
- **`castling_config(player, board)` in moves.rs** — now requires `&Board` parameter to read `castling_starts`.
- **`EngineOptions::chess960: bool`** — enables Chess960 mode. Set via `setoption name Chess960 value true`.
- **`EvalWeights` expanded with 5 fields:** `terrain_fortress_bonus`, `terrain_king_wall_bonus`, `terrain_king_trap_penalty`, `dkw_proximity_penalty`, `claim_win_urgency_bonus`.

### Key Constants

| Constant | Value | Location |
|----------|-------|----------|
| `dkw_proximity_penalty` | 20cp | EvalWeights |
| `terrain_fortress_bonus` | 15cp | EvalWeights |
| `terrain_king_wall_bonus` | 20cp per piece | EvalWeights |
| `terrain_king_trap_penalty` | 30cp | EvalWeights (3+ adjacent) |
| `claim_win_urgency_bonus` | 50cp (Std) / 100cp (Agg) | EvalWeights |
| Terrain outpost bonus (knight/bishop) | 10cp | terrain.rs (fixed) |
| Dead capture victim_val | 1 (BRS) / 1.0 (MCTS) | brs.rs, mcts.rs |
| DKW proximity threshold | Manhattan <= 3 | dkw.rs |
| FFA claim-win threshold | 21 points | ffa_strategy.rs |

### Known Limitations

- **W13 (carried):** MCTS score 9999 (max) in some positions — unchanged.
- **Pondering not implemented:** Deferred from Stage 13.
- **No SIMD:** Stage 19 target.
- **Chess960 FEN:** Not supported (W28). Only `position startpos` with Chess960 enabled.
- **Self-play validation not done:** Manual tuning validation via observer (human-driven, post-implementation).

### Performance Baselines

| Metric | Value | Notes |
|--------|-------|-------|
| Chess960 position generation | <1us | Pure deterministic computation |
| DKW eval penalty | <1us | 3 king distance checks |
| FFA strategy eval | <1us | Score comparisons |
| Terrain eval | <5us | O(piece_count × 8) board lookups |
| Test count | 557 | 308 unit + 249 integration (6 ignored) |

### Open Questions

- **Terrain weight tuning:** Default values (15/20/30cp) are educated guesses. Need self-play validation to confirm or adjust.
- **FFA claim-win threshold:** Hardcoded 21 points. Should this be configurable?
- **DKW proximity penalty scaling:** Flat 20cp regardless of distance (within threshold). Could be inversely proportional to distance for more precision.

### Reasoning

- **Atomic castling make/unmake:** In standard chess, king and rook never share destination squares during castling. In Chess960, they can overlap (e.g., rook on g1, king castles to g1 while rook goes to f1). The remove-both-then-place pattern prevents "square already occupied" panics.
- **Dead piece value of 1 (not 0):** Using 0 would make dead captures sort equivalently to non-captures. Using 1 gives them minimal positive value so they're attempted but after all alive captures.
- **Board::empty() with standard castling_starts:** Prevents FEN4-loaded boards (which start from empty) from having invalid (0,0,0) castling starts that cause panics when walking paths between invalid squares.
- **DKW proximity gated on PieceStatus::Dead:** The GameState tracks DeadKingWalking status, but we check the board-level piece status directly since we're in the eval context (no GameState access for player status in all paths).

---

## Related

- Stage spec: [[stage_17_variants]]
- Audit log: [[audit_log_stage_17]]
