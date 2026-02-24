# Downstream Log — Stage 03: GameState

## Notes for Future Stages

### Must-Know

1. **GameState wraps Board + MoveGen.** All game lifecycle goes through GameState::apply_move(). Do NOT use make_move/unmake_move directly for game progression — only for search lookahead.
2. **Turn management.** GameState tracks current_player separately from board.side_to_move(). After apply_move, both are synchronized. Search should use board-level make/unmake, not GameState.
3. **Terrain mode is a GameState flag**, not a Board flag. Board doesn't know about terrain mode — it just stores piece status. The GameState decides whether to convert eliminated players' pieces to terrain vs DKW.
4. **DKW moves are instant.** They happen between active player turns via process_dkw_moves(). DKW moves modify the board permanently (no unmake). Board's side_to_move is temporarily changed and restored.
5. **Checkmate timing: confirmed at the affected player's turn.** Not when check is delivered. Chain elimination loop handles cascading checkmates.
6. **DKW moves run BEFORE elimination chain.** In `apply_move()`, `process_dkw_moves()` executes before `check_elimination_chain()`. This ordering is a permanent invariant — DKW pieces can move to positions that change whether a player has legal moves. If the order is reversed, `check_elimination_chain` may see legal moves (e.g., capturing a DKW piece) that disappear after DKW processing, leaving a player with zero moves but not eliminated. *Added post-Stage 7 bugfix.*
7. **`handle_no_legal_moves()` is the safety net for missed eliminations.** If `check_elimination_chain` in `apply_move()` fails to detect a checkmate (edge case), `handle_no_legal_moves()` catches it when the protocol calls `handle_go()` and finds zero legal moves. It calls `determine_status_at_turn()`, eliminates the player, advances the turn, and returns a `MoveResult`. *Added post-Stage 7 bugfix.*

### API Contracts

1. `GameState::new(board, game_mode, terrain_mode) -> Self` — primary constructor
2. `GameState::new_standard_ffa() -> Self` — standard starting position, no terrain
3. `GameState::new_standard_ffa_terrain() -> Self` — standard starting position, terrain mode
4. `GameState::apply_move(mv) -> MoveResult` — the central method. Panics if game is over.
5. `GameState::legal_moves() -> Vec<Move>` — generates legal moves for current_player. Calls board.set_side_to_move internally.
6. `GameState::resign_player(player) -> MoveResult` — triggers DKW or terrain conversion
7. `GameState::timeout_player(player) -> MoveResult` — same as resign
8. `GameState::board() -> &Board` — immutable board access
9. `GameState::board_mut() -> &mut Board` — mutable board access (needed for search make/unmake)
10. `GameState::clone()` — derives Clone for MCTS tree expansion
11. `MoveResult` struct: mv, points_scored, eliminations, dkw_moves, game_ended
12. `PlayerStatus`: Active, DeadKingWalking, Eliminated
13. `scoring::capture_points(PieceType, PieceStatus) -> i32` — Dead/Terrain = 0
14. `scoring::check_bonus_points(kings_checked: usize) -> i32` — 0/1: 0, 2: +1, 3: +5
15. `rules::determine_status_at_turn(board, player) -> TurnDetermination` — checkmate/stalemate/has-moves
16. `rules::is_draw_by_repetition(history, current_hash) -> bool` — 3-fold
17. `rules::is_draw_by_fifty_moves(halfmove_clock) -> bool` — >= 200
18. `GameState::handle_no_legal_moves() -> MoveResult` — Call when `legal_moves().is_empty()`. Determines checkmate vs stalemate via `determine_status_at_turn()`, eliminates the current player, advances to next alive player, returns MoveResult with eliminations. *Added post-Stage 7 bugfix.*

### Known Limitations

1. **position_history grows unbounded.** For long games or MCTS with many clones, the Vec<u64> could become large. Consider capping at ~1000 entries or using a hash map for repetition detection in future stages.
2. **DKW increments halfmove_clock.** DKW instant moves go through make_move which increments the clock. This may cause the 50-move rule to trigger earlier than expected. The rules are ambiguous on this.
3. **No auto-claim implementation.** The spec mentions "autoclaim triggers when eliminated 2nd-place leads 3rd-place by 21+ points." Currently only check_claim_win for active players is implemented.
4. **GameOverReason enum defined but not stored.** The GameState stores game_over bool and winner, but not WHY the game ended. Future stages may want this.
5. **No move history stored.** GameState only stores position hashes, not the moves themselves. For protocol/UI replay, move history would need to be added.
6. **~~Huginn gates not wired.~~** *(Historical — Huginn was retired in Stage 8 and replaced with the `tracing` crate; see ADR-015.)*

### Performance Baselines

| Metric | Value | Notes |
|---|---|---|
| Test count | 164 | 108 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 + 18 stage-03 |
| 1000 random games (normal mode, debug) | ~104s | Permanent invariant test |
| 1000 random games (terrain mode, debug) | ~104s | Permanent invariant test |
| Perft values | 20/395/7800/152050 | Unchanged from Stage 2 |

### Open Questions

1. Should DKW moves increment the halfmove_clock? Current implementation: yes (via make_move). Could be changed by resetting clock after DKW moves.
2. Should position_history be bounded? Current: unbounded Vec. MCTS clones will copy the full history.
3. Should the Game enum include Teams mode variants now, or defer to Stage 17?

### Reasoning

1. **Why separate scoring.rs and rules.rs?** Scoring is pure calculation (no board state). Rules needs board access for check detection, DKW moves, terrain conversion. Separating keeps scoring easily testable and reusable.
2. **Why `#[allow(dead_code)]` on GameState?** The game_mode field is needed for future Teams mode but not yet used. Better to have the field ready than to add it later.
3. **Why does apply_move use make_move directly (not through movegen)?** apply_move IS the game-level wrapper. It calls make_move for board mutation, then adds scoring, elimination, DKW, and game-over detection on top.



---

## Related

- Stage spec: [[stage_03_gamestate]]
- Audit log: [[audit_log_stage_03]]
