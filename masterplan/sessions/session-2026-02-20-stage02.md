# Session: Stage 02 Implementation

**Date:** 2026-02-20
**Agent:** Claude Opus 4.6
**Stage:** 2 — Move Generation + Attack Query API

---

## Summary

Completed Stage 2 in full: pre-computed attack tables, attack query API, pseudo-legal + legal move generation, make/unmake, perft validation. Fixed two critical 4PC-specific bugs found during testing.

## Key Decisions

1. **Pawn attack reverse lookup:** Use `(player + 2) % 4` to find the opposite-facing player's capture table for reverse pawn attack detection.
2. **En passant captured square:** Use `prev_player(capturing_player)` to determine pushing player's direction, since EP only lasts one turn in 4PC.
3. **Perft values established as permanent invariants:** depth 1=20, 2=395, 3=7800, 4=152050.

## Bugs Found & Fixed

1. **Pawn attack reverse lookup (attacks.rs):** `is_square_attacked_by` was using the attacker's own pawn table to find reverse attacks. In 4PC, pawn captures are directional — the reverse of player P's captures equals the opposite-facing player `(P + 2) % 4`'s captures.

2. **En passant captured square (moves.rs):** The function computed the captured pawn's position using the capturing player's backward direction. In 4PC, the capturing player may face a completely different direction than the pushing player. Fix: use `prev_player(capturing_player)` since EP lasts exactly one turn.

3. **En passant representation (from Stage 1):** `en_passant: Option<u8>` stored a file index. Insufficient for 4PC where Blue/Green pawns move along files, not ranks. Changed to `Option<Square>`. Zobrist EP keys expanded from 14 to 196.

## Commits

1. `26b6365` — `[Stage 02] Fix en passant representation: file -> square for 4PC`
2. `197f546` — `[Stage 02] Move generation, attack query API, make/unmake, perft`
3. `847af5e` — `[Stage 02] Format and clippy fixes`
4. (session-end meta commit)

## Test Count

125 total (87 unit + 2 stage-00 + 18 stage-01 + 18 stage-02)

## Related

- [[stage_02_movegen]]
- [[audit_log_stage_02]]
- [[downstream_log_stage_02]]
