# Audit Log — Stage 03: GameState

## Pre-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Build State
- Compiles: Yes (`cargo build` in 0.07s, `cargo build --features huginn` in 1.88s)
- Tests pass: Yes (125 total: 87 unit + 2 stage-00 + 18 stage-01 + 18 stage-02 — all pass)
- Previous downstream flags reviewed: Yes — Stage 0, Stage 1, and Stage 2 downstream logs reviewed

### Findings

**From [[downstream_log_stage_02]]:**
1. Attack query API is the board boundary (ADR-001). Use `is_square_attacked_by`, `attackers_of`, `is_in_check` for all attack/check queries. Never read `board.squares[]` directly.
2. En passant stores full target square (`Option<Square>`), not file. Cleared at start of every `make_move`.
3. Board derives Clone (added Stage 2).
4. `generate_legal(board: &mut Board) -> Vec<Move>` — requires `&mut` for make/unmake. Stage 3 check detection will use `is_in_check(player, &board)`.
5. `make_move(board, mv) -> MoveUndo` and `unmake_move(board, mv, undo)` are the mutation API. Side-to-move is advanced by make_move.
6. **Perft values not independently verified** (WARNING from [[audit_log_stage_02]]). Not blocking for Stage 3 — rules engine is built on top of movegen, not replacing it.
7. **Huginn gates not wired** (NOTE from [[Issue-Huginn-Gates-Unwired]]). Stage 3 has 7 Huginn gates specified. Will defer wiring per established pattern.

**From [[downstream_log_stage_01]]:**
1. Board is `[Option<Piece>; 196]`, index = `rank * 14 + file`. Check `is_valid_square(sq)` before accessing.
2. Piece lists per player via `board.piece_list(player)`. King squares via `board.king_square(player)`.
3. Zobrist hash incrementally maintained by all Board mutation methods.
4. FEN4 format custom — no external standard.

**From [[downstream_log_stage_00]]:**
1. `huginn_observe!` macro available. Arguments must be pure.
2. No global buffer instance yet.

**From [[MOC-Active-Issues]]:**
- WARNING: [[Issue-Perft-Values-Unverified]] — not blocking Stage 3.
- NOTE: [[Issue-Huginn-Gates-Unwired]] — accumulates more gates this stage, still deferred.

### Risks for This Stage

1. **Check detection timing (Appendix C, 4PC_RULES_REFERENCE):** Checkmate is confirmed only at the affected player's turn, not when the check is delivered. Intervening players may rescue the king. Getting this wrong would cause false checkmate detection.
2. **DKW timing:** Dead kings make random moves instantly between turns, not as a full turn. Implementing this as a turn would break the game flow.
3. **Terrain piece semantics:** Terrain pieces block movement, cannot be captured, produce no check. Must not interfere with existing attack/movegen logic — terrain pieces are "walls."
4. **Promoted queen dual value (Appendix C):** Worth 1 point on capture in FFA scoring, but evaluates at 900cp in search. Stage 3 scoring must use capture points, not eval values.
5. **Stalemate awards 20 points in FFA.** Not zero, not a draw. This is a common 4PC gotcha.
6. **GameState must be cheaply cloneable (design note).** Use fixed-size arrays where possible. `position_history: Vec<u64>` will grow — consider bounded storage or accept the clone cost.
7. **Turn skipping for eliminated players.** Must not infinite-loop if all players are eliminated.
8. **Three-way check from multiple opponents.** Check detection must test all 3 opponents, not just one.
9. **Game-over conditions are multi-faceted:** Three eliminated, claim win (21+ point lead with 2 remaining), autoclaim, draw conditions. Each needs correct implementation.


---

## Post-Audit
**Date:** 2026-02-20
**Auditor:** Claude Opus 4.6

### Deliverables Check

| Deliverable | Status | Notes |
|---|---|---|
| GameState struct with all fields | Done | Board, player_status, scores, current_player, elimination_order, position_history, game_mode, terrain_mode, game_over, winner, rng_seed |
| Turn rotation with elimination skip | Done | `next_active_player()` walks forward skipping non-Active, max 4 iterations |
| Check detection using attack lookups | Done | `kings_checked_by_move()` checks all active opponent kings |
| Checkmate/stalemate determination | Done | `determine_status_at_turn()` generates legal moves; 0+check=mate, 0+no-check=stalemate |
| Chain elimination loop | Done | `check_elimination_chain()` re-checks next player after each elimination |
| FFA scoring system | Done | All capture values match spec. Check bonuses (double=+1, triple=+5) |
| Elimination pipeline | Done | `eliminate_player()` handles checkmate, stalemate, resign, timeout, DKW |
| DKW random king moves | Done | `generate_dkw_move()` with LCG, king-only legal moves, processed after each active move |
| Terrain conversion | Done | `convert_to_terrain()` sets pieces to Terrain, removes king |
| Game-over detection | Done | Last standing, claim win (21+ lead), draw (repetition/50-move) |
| Position repetition tracking | Done | Zobrist pushed after each active move, 3-fold check |
| 50-move rule | Done | halfmove_clock >= 200 (4 players × 50 rounds) |
| Board::set_piece_status | Done | In-place status change, hash-neutral |
| Terrain awareness in movegen | Done | attacks.rs: terrain doesn't attack/give check. generate.rs: terrain blocks, uncapturable |
| Perft values unchanged | Done | 20, 395, 7800, 152050 verified |
| Integration tests | Done | 18 tests in stage_03_gamestate.rs |
| 1000+ random game playouts (normal) | Done | PERMANENT INVARIANT |
| 1000+ random game playouts (terrain) | Done | PERMANENT INVARIANT |
| Huginn gates | Deferred | Per established pattern — unwired until telemetry needed |

### Code Quality
#### Uniformity
Good. GameState module follows the same patterns as board and movegen: public API in mod.rs, implementation details in sub-modules (scoring.rs, rules.rs). Attack/check queries go through the established API per ADR-001.

#### Bloat
Minimal. Three new files total ~600 lines of implementation code. No unnecessary abstractions. MoveResult carries only essential data. GameState has only the fields needed.

#### Efficiency
Adequate for Stage 3 scope. `determine_status_at_turn()` generates full legal move list to detect checkmate/stalemate — acceptable cost since it's called at most once per player per move. DKW move generation also generates full legal moves for king filtering — could be optimized later if needed.

`position_history: Vec<u64>` grows unbounded. For Stage 3 this is fine. Future stages may want to cap or use a hash table.

#### Dead Code
`game_mode` field is stored but not read (allowed with `#[allow(dead_code)]` — needed for Teams mode in Stage 17). `GameOverReason` enum defined but only used for documentation — could be removed, but provides clear API surface.

#### Broken Code
None found. All 164 tests pass. Random playouts (2000+ games across normal and terrain modes) complete without crashes.

#### Temporary Code
None.

### Search/Eval Integrity
Stage 3 is pure rules — no eval or search code. GameState::board_mut() provides &mut Board for legal move generation, which will be used by search in Stage 7+. The Clone derive ensures MCTS (Stage 10) can cheaply fork game states.

Scoring constants are in `scoring.rs` and are purely rule-based FFA points. Eval values (Stage 6+) are separate and will use the Evaluator trait.

### Future Conflict Analysis
1. **Stage 4 (Odin Protocol):** Protocol needs to create GameState and call apply_move. Public API is ready.
2. **Stage 5 (UI Shell):** UI will need scores(), current_player(), is_game_over(), board() — all exposed.
3. **Stage 7 (Searcher):** BRS will use GameState::clone() + apply_move. Clone is derived.
4. **Stage 10 (MCTS):** Will clone GameState for tree expansion. Vec<u64> position_history may need optimization.
5. **Stage 17 (Game Mode Variants):** game_mode field is ready but unused. Teams scoring and rules will require extending scoring.rs and rules.rs.

### Unaccounted Concerns
1. **WARNING:** DKW kings can be captured, awarding 0 points but eliminating the DKW player. If a DKW king is the only piece blocking a checkmate line, capturing it could change the game. This is correct per rules but creates complex interactions.
2. **WARNING:** The 50-move rule uses halfmove_clock from the Board, which counts all moves (not just active player moves). With 4 players, 50 full rounds = 200 half-moves. The Board increments halfmove_clock by 1 per make_move, so this is correct — but DKW instant moves also increment the clock. This is a grey area in the rules. Current implementation is reasonable.
3. **NOTE:** Chain elimination only walks forward — if Red checkmates Blue, and Blue's elimination causes Yellow to be checkmated, we check Yellow then Green. But we don't re-check Red (who just moved). This is correct because Red just proved they had a move.

### Reasoning & Methods
- Read all upstream audit and downstream logs before implementation
- Modified movegen first (terrain awareness) and verified perft values unchanged before proceeding
- Built GameState incrementally: types → scoring → rules → mod.rs orchestration
- Unit tests in each module file, integration tests in stage_03_gamestate.rs
- Random playouts use LCG PRNG with fixed seeds for reproducibility
- All tests run on every build: `cargo test` (164 total)


---

## Related

- Stage spec: [[stage_03_gamestate]]
- Downstream log: [[downstream_log_stage_03]]
