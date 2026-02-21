---
type: connection
tags:
  - stage/06
  - stage/07
  - area/eval
  - area/search
last_updated: 2026-02-21
---

# Connection: Eval to Search

## What Connects

- [[Component-Eval]] (provider)
- [[stage_07_plain_brs]] / future Searcher (consumer)

## How They Communicate

Search owns a `Box<dyn Evaluator>` and calls it at leaf nodes:

```rust
let score = evaluator.eval_scalar(&position, player);
```

For MCTS (Stage 10):
```rust
let values = evaluator.eval_4vec(&position);
```

## Contract

1. **Search calls through the Evaluator trait.** Never calls specific implementations directly.
2. **eval_scalar is perspective-relative.** Search must pass the correct player for the perspective it wants.
3. **eval_4vec returns independent sigmoids.** Each value in [0,1], indexed by Player::index(). Not softmax.
4. **Eval is thread-safe for the bootstrap.** BootstrapEvaluator is stateless (zero-size, `&self` only). NNUE will need special handling for accumulator state.
5. **PIECE_EVAL_VALUES available for move ordering.** Search can use these for MVV-LVA without depending on eval internals.

## Evolution

| Stage | Change |
|---|---|
| 6 (current) | Trait defined, no search consumer yet |
| 7 | BRS search calls eval_scalar at leaf nodes |
| 8 | BRS hybrid may use eval for board context scoring |
| 9 | Move ordering uses PIECE_EVAL_VALUES for MVV-LVA |
| 10 | MCTS calls eval_4vec for value estimates |
| 16 | NnueEvaluator replaces bootstrap; search code unchanged |
