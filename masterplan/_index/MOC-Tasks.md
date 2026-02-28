---
type: moc
tags:
  - type/moc
last_updated: 2026-02-27
---

# Task Tracker

Active and completed task logs. Each task file records an agent's understanding, investigation, plan, and execution for a unit of work. Files are named with `_in_progress` or `_complete` suffix to show status at a glance.

## Naming Convention

- **In progress:** `Task-Short-Name_in_progress.md`
- **Complete:** `Task-Short-Name_complete.md` (renamed from `_in_progress` when done)

When a task completes, rename the file (change suffix) and update this MOC.

## In Progress

<!-- Tasks currently being worked on -->
_None._

## Completed

<!-- Completed tasks — newest at top. Add reference note: "See [[Session-X]] and [[Issue-Y]] for context." -->

## Queued

<!-- Tasks identified but not yet started -->
- Fix lead_penalty hardcoded weight (W1 from audit) — see [[Issue-Bootstrap-Eval-Lead-Penalty-Tactical-Mismatch]]
- Consolidate prev_player() DRY violation (W2 from audit)
- Investigate + fix pawn-push preference / king walk — see [[Issue-Pawn-Push-Preference-King-Walk]]
- Vec clone cost retrofit — see [[Issue-Vec-Clone-Cost-Pre-MCTS]]
- Protocol parser edge case tests (W3 from audit)
