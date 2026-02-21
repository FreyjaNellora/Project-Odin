---
type: issue
severity: WARNING
status: open
stage_found: 5
tags:
  - stage/05
  - dkw
last_updated: 2026-02-20
---

# Issue: DKW Invisible Moves in UI

## Problem

When a player enters Dead King Walking (DKW) state, their king makes instant random moves inside `apply_move()` (see [[Pattern-DKW-Instant-Moves]]). These moves are not reported through the Odin Protocol as separate events. The UI's rendering cache has no way to know about them.

## Impact

After a DKW event occurs during play, the UI will show the DKW king in its pre-DKW position while the engine's internal state has the king on a different square. This creates a visual desync.

## Mitigation

- **Current:** Accepted limitation. DKW only occurs after resignation/checkmate, which is rare during manual play in Stage 5.
- **Future (Stage 18):** Could add a protocol extension to report DKW king positions, or a `display` command that returns current board state for UI synchronization.

## Related

- [[Component-BasicUI]] — affected component
- [[Pattern-DKW-Instant-Moves]] — explains DKW mechanics
- [[downstream_log_stage_03]] — documents DKW behavior
- [[audit_log_stage_05]] — risk #2 in pre-audit
