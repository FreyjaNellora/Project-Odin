---
type: issue
date_opened: 2026-02-20
last_updated: 2026-02-20
date_resolved:
stage: 2
severity: warning
status: open
tags:
  - stage/02
  - area/movegen
  - severity/warning
---

# Perft Values Unverified Against External Reference

## Description

Stage 2 established perft values as permanent invariants:

| Depth | Nodes |
|---|---|
| 1 | 20 |
| 2 | 395 |
| 3 | 7,800 |
| 4 | 152,050 |

These values are **self-consistent** -- Zobrist make/unmake round-trips verify, piece lists stay synchronized, 1000 random game playouts complete without crashes. However, **no independent 4PC FFA chess engine exists** to cross-check these numbers.

A systematic error (e.g., wrong promotion rank for one player, incorrect castling path) could produce consistent but **wrong** perft counts.

## Affected Components

- [[stage_02_movegen]] -- perft implementation and values
- [[audit_log_stage_02]] -- documented as unaccounted concern #1
- [[downstream_log_stage_02]] -- listed as known limitation #6

## Workaround

Values are treated as permanent invariants from this implementation. If an external reference becomes available, compare immediately. Most likely verification sources:

1. Another 4PC engine (none known to exist publicly)
2. Manual move counting at depth 1-2 (20 and 395 can be hand-verified)
3. chess.com API if they expose legal move counts

## Resolution

<!-- When verified or proven wrong, document here. -->

## Related

- [[stage_02_movegen]]
- [[audit_log_stage_02]]
- [[downstream_log_stage_02]]
