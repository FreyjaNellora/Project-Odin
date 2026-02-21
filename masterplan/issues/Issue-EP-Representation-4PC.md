---
type: issue
date_opened: 2026-02-20
last_updated: 2026-02-20
date_resolved: 2026-02-20
stage: 2
severity: warning
status: resolved
tags:
  - stage/01
  - stage/02
  - area/board
  - severity/warning
---

# En Passant Representation Insufficient for 4PC

## Description

Stage 1's Board stored en passant as `Option<u8>` representing a **file index** (0-13). This is the standard chess approach where the file alone identifies the EP target because pawns only move along ranks.

In 4PC, Blue and Green pawns move along **files** (horizontally), not ranks. Their en passant target squares exist on specific **ranks**, not identifiable by file alone. Storing just a file index makes it impossible to reconstruct Blue/Green EP targets.

## Affected Components

- [[stage_01_board]] -- original design
- [[stage_02_movegen]] -- discovered during pre-audit
- [[audit_log_stage_02]] -- documented as pre-audit finding #5
- [[downstream_log_stage_01]] -- original API contract listed `en_passant() -> Option<u8>` as file

## Workaround

None needed -- fixed before any downstream consumer relied on the file-only representation.

## Resolution

Changed `en_passant: Option<u8>` from file index to full target **square index** (`Option<Square>`). The type is still `u8` but the semantics changed from "which file" to "which square."

Changes made:
1. `board_struct.rs` -- field semantics changed
2. `zobrist.rs` -- EP keys expanded from 14 (one per file) to 196 (one per square index)
3. `fen4.rs` -- EP parsing/serialization now uses full square notation (e.g., "e3") instead of file letter
4. All Stage 1 EP tests updated

Committed as: `[Stage 02] Fix en passant representation: file -> square for 4PC`

Per [[AGENT_CONDUCT]] Section 1.2, this change was documented in [[audit_log_stage_02]] and [[downstream_log_stage_02]].

## Related

- [[audit_log_stage_01]] -- original design
- [[audit_log_stage_02]] -- pre-audit finding #5
- [[downstream_log_stage_02]] -- updated API contract
- [[DECISIONS]] -- no ADR needed (clear correctness fix, not a design trade-off)
