---
type: moc
tags:
  - type/moc
last_updated: 2026-02-20
---

# Active Issues

Open problems, workarounds, and tech debt. Updated as issues are created and resolved.

## Blocking

_None._

## Warning

- [[Issue-Perft-Values-Unverified]] -- Stage 2 perft values self-consistent but no external reference exists to cross-check

## Notes

- [[Issue-Huginn-Gates-Unwired]] -- Stages 1-4 Huginn observation gates not wired; deferred until buffer plumbing exists
- [[Issue-DKW-Halfmove-Clock]] -- DKW instant moves increment halfmove_clock via make_move; may cause premature 50-move rule triggers in DKW games

## Recently Resolved

- [[Issue-EP-Representation-4PC]] -- En passant stored file index, insufficient for 4PC. Fixed Stage 2: now stores full square index. (2026-02-20)
