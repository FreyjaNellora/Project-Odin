---
type: issue
date_opened: 2026-02-20
last_updated: 2026-02-20
date_resolved: 2026-02-20
stage: 5
severity: warning
status: resolved
tags:
  - stage/05
  - severity/warning
  - area/ui
---

# Issue: Castling Display Broken for Blue/Green

## Description

In `applyMoveToBoard` (useGameState.ts), castling detection checked `Math.abs(fileOf(to) - fileOf(from)) >= 2`. This works for Red and Yellow (who castle by moving the king horizontally, changing file). But Blue and Green kings castle by moving vertically (changing rank, not file), so the detection never triggered for these players.

Symptom: When Blue or Green castled, the king moved but the rook stayed in its original position on the display.

## Affected Components

- [[Component-BasicUI]] — useGameState.ts `applyMoveToBoard` function
- Display-side rendering cache only. Engine state was never affected.

## Workaround

None needed after fix. Before fix, Blue/Green castling showed only the king move.

## Resolution

Added orientation-aware detection:

```typescript
const isVertical = piece.owner === 'Red' || piece.owner === 'Yellow';
const moveDist = isVertical
  ? fileOf(to) - fileOf(from)
  : rankOf(to) - rankOf(from);
if (Math.abs(moveDist) >= 2) {
  // Red/Yellow: rook placement by file offset
  // Blue/Green: rook placement by rank offset
}
```

## Related

- [[audit_log_stage_05]] — Post-Audit Addendum
- [[Session-2026-02-20-Stage05-Bugfix]]
- [[Component-BasicUI]]
