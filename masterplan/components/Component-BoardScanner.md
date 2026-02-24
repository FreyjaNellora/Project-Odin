---
type: component
stage_introduced: 8
tags:
  - stage/08
  - area/search
status: active
last_updated: 2026-02-23
---

# Component: Board Scanner (Hybrid Opponent Modeling)

Pre-search analysis engine that scans the board for attack patterns, king exposure, score standings, and high-value targets. Produces a `BoardContext` struct that informs hybrid opponent reply scoring and progressive narrowing at MIN nodes.

## Purpose

Without the board scanner, BRS treats all opponents identically — each picks the move that minimizes root player's eval. With it, opponent moves are scored by a hybrid formula that considers both how harmful a move is to root AND how likely the opponent is to play it. This produces more realistic opponent modeling in 4-player chess.

## Files

- `search/board_scanner.rs` — all scanner, classifier, and hybrid scoring logic

## Key Types

- **BoardContext** — Pre-search analysis output. Contains game mode, root player, weakest player, most dangerous opponents (sorted), root danger level, high-value targets, convergence detection, and per-opponent profiles.
- **OpponentProfile** — Per-opponent analysis: aggression toward root, own vulnerability, best target, can afford to attack, supporting attack flag.
- **MoveClass** — `Relevant` (targets root) or `Background` (doesn't interact with root).
- **ScoredReply** — Hybrid-scored move: `hybrid_score`, `objective_strength`, `harm_to_root`, `likelihood`.

## Public API

| Item | Signature | Notes |
|---|---|---|
| `scan_board` | `(&GameState, Player) -> BoardContext` | Runs once pre-search, < 1ms |
| `classify_move` | `(Move, &Board, Player) -> MoveClass` | Pure table lookup, no eval |
| `classify_moves` | `(&[Move], &Board, Player) -> (Vec<Move>, Option<Move>)` | Splits into relevant + best background |
| `narrowing_limit` | `(depth: u8) -> usize` | Depth schedule: 10/6/3 |
| `select_hybrid_reply` | `(&mut GameState, &dyn Evaluator, Player, Player, &[Move], &BoardContext, u8) -> Option<Move>` | Full hybrid: classify → narrow → score → select |
| `score_reply` | `(Move, &Board, Player, Player, &BoardContext, i16, i16) -> ScoredReply` | Single move hybrid scoring |

## Internal Design

### Board Scanner Flow
1. Compute material totals per player
2. Find weakest player (lowest material among active)
3. Compute root king danger (attacked adjacent squares / total, check penalty, shield bonus)
4. Profile each opponent: aggression, vulnerability, best target, can afford attack
5. Detect supporting attacks (A targets root + B has aggression > 0.15)
6. Sort opponents by aggression → most_dangerous ordering
7. Find high-value targets (opponent pieces ≥ 300cp attacked by root)
8. Detect convergence (2+ opponents targeting root with aggression > 0.2)

### Move Classifier
A move is **Relevant** if it:
- Captures one of root's pieces, OR
- Lands adjacent to root's king (Chebyshev distance ≤ 1), OR
- Is a knight landing within 2 squares of root's king

Everything else is **Background**. Best background move tracked by capture value.

### Hybrid Reply Scoring Formula
```
score = harm_to_root * likelihood + objective_strength * (1 - likelihood)
```
- `harm_to_root` (0-1): capture value toward root + king proximity
- `likelihood` (0.1-1.0): base 0.7 for relevant moves, bonuses for best_target/supporting, penalty for high vulnerability
- `objective_strength` (0-1): normalized eval delta (how much this move improves opponent's position)

### Progressive Narrowing
Depth-based candidate limits applied before hybrid scoring:
- Depth 1-3: top 10 candidates
- Depth 4-6: top 6
- Depth 7+: top 3

`cheap_presort` sorts by capture value (MVV) before truncation.

## Connections

- Depends on: [[Component-Board]] (piece_list, piece_at, king_square, is_valid_square), [[Component-MoveGen]] (is_square_attacked_by, make_move, unmake_move), [[Component-Eval]] (eval_scalar for hybrid scoring), [[Component-GameState]] (game_mode, player_status, scores)
- Depended on by: [[Component-Search]] (BrsContext stores BoardContext, min_node calls select_hybrid_reply)
- Communicates via: [[Connection-Eval-to-Search]]

## Tracing Points

Potential `tracing` spans/events:

| Span/Event | Location | Level |
|---|---|---|
| `board_context` | After scan_board() returns | DEBUG |
| `cheap_filter` | After classify_moves in select_hybrid_reply | TRACE |
| `progressive_narrowing` | After truncation in select_hybrid_reply | TRACE |
| `reply_scoring` | After hybrid scoring loop | TRACE |

## Gotchas

1. **Scanner data is frozen for entire search.** `BoardContext` is computed once pre-search. Deep in the tree, actual board state may differ significantly. Delta updater deferred to v2.
2. **Classify_move checks pre-move board, not post-move.** The move's destination is checked against root's pieces and king on the current board (before the move is applied). This is correct — we're classifying the move before applying it.
3. **Quiescence MIN nodes skip hybrid scoring.** They use plain `select_best_opponent_reply`. This is by design — quiescence is captures-only with small candidate sets.
4. **`INVALID_SQUARE = 255` used as sentinel.** Unused high-value target slots contain this value. Never dereference without checking `high_value_target_count`.

## Performance Notes

- Board scanner: < 1ms per call in release build, < 10ms in debug.
- Progressive narrowing: ~49% node reduction at depth 6, ~46% at depth 8 vs Stage 7 baseline.
- Hybrid scoring overhead: one `eval_scalar` call per relevant candidate per MIN node. Bounded by narrowing limits.

## Known Issues

- Scanner data staleness during search — acceptable for v1, delta updater deferred.
- W5 (stale GameState fields) affects scanner indirectly — scanner reads accurate pre-search data, but eval during search reads stale scores/status.

## Build History

- [[Session-2026-02-23-Stage08]] — initial implementation (Steps 1-7)
