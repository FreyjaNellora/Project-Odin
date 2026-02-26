# Stage 10: MCTS Strategic Search (Gumbel MCTS) — Detailed Notes

*This file will be populated with implementation notes, decisions, and findings as Stage 10 is developed.*

## Pre-Implementation Notes

**Gumbel MCTS replaces UCB1 at root (ADR-016).** The MASTERPLAN spec has been updated to use Gumbel-Top-k + Sequential Halving at root nodes instead of standard UCB1. Key reason: works with as few as 2 simulations, which is critical for Phase 2 residual budget scenarios. See [[ADR-016]] for full rationale.

**Progressive History provides MCTS warm-start (ADR-017).** Non-root nodes incorporate BRS history heuristic scores via `PH(a) = H(a) / (N(a) + 1)`. The history table is extracted from BRS after Phase 1 and injected into MctsSearcher. See [[ADR-017]] for full rationale.

**Prior policy pre-NNUE:** `pi(a) = softmax(ordering_score(a) / T)` using BRS's move ordering scores. This is a weak prior — Gumbel noise provides exploration and Sequential Halving corrects via Q-values.

---

## Related

- Audit log: [[audit_log_stage_10]]
- Downstream log: [[downstream_log_stage_10]]
- Full spec: [[MASTERPLAN]] Section 4, Stage 10
- ADRs: [[ADR-016]] (Gumbel MCTS), [[ADR-017]] (Progressive History)
