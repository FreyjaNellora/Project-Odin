---
type: issue
date_opened: 2026-02-20
last_updated: 2026-02-20
date_resolved: 2026-02-20
stage: 5
severity: blocking
status: resolved
tags:
  - stage/05
  - severity/blocking
  - area/ui
---

# Issue: En Passant False Positive for Blue/Green Pawns

## Description

In `applyMoveToBoard` (useGameState.ts), the en passant detection condition checked only `fileOf(from) !== fileOf(to)`. For Blue and Green pawns, whose forward direction changes the file (not rank), every forward step triggered en passant detection. The captured-square formula `squareFrom(fileOf(to), rankOf(from))` then equaled the destination square itself, removing the just-placed piece.

Symptom: Blue and Green pawns visually vanished when moved forward.

## Affected Components

- [[Component-BasicUI]] — useGameState.ts `applyMoveToBoard` function
- Display-side rendering cache only. Engine state was never affected.

## Workaround

None needed after fix. Before fix, only Red and Yellow pawns worked visually.

## Resolution

Changed en passant condition to require both file AND rank to change (a true diagonal):

```typescript
const isDiagonal = fileOf(from) !== fileOf(to) && rankOf(from) !== rankOf(to);
if (piece.pieceType === 'Pawn' && isDiagonal && prev[to] === null) {
```

This correctly identifies diagonal moves for all 4 player orientations. See also [[Pattern-EP-Captured-Square-4PC]].

## Related

- [[audit_log_stage_05]] — Post-Audit Addendum
- [[Session-2026-02-20-Stage05-Bugfix]]
- [[Component-BasicUI]]
