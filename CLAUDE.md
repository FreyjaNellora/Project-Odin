# Project Odin -- Agent Orientation

A four-player chess engine: NNUE + BRS/Paranoid Hybrid + MCTS.

## Before You Start

Read these files in order:

1. `masterplan/STATUS.md` -- Where is the project? What stage? What's blocked?
2. `masterplan/HANDOFF.md` -- What was the last session doing? What's next?
3. `masterplan/AGENT_CONDUCT.md` Section 1.1 -- Full stage entry protocol.

If you're new to the project or starting a new stage, also read:
4. `masterplan/DECISIONS.md` -- Why key architectural choices were made.
5. `masterplan/MASTERPLAN.md` -- Full spec (refer to specific sections as needed, don't read all 1,484 lines unless necessary).

## Quick Understanding (Obsidian Vault)

For **fast lookup** on how the engine works, use the knowledge vault at `masterplan/`:

| You want to know... | Read |
|---|---|
| Big picture navigation | `masterplan/_index/MOC-Project-Odin.md` |
| Tier 1 stages, logs, invariants | `masterplan/_index/MOC-Tier-1-Foundation.md` |
| Known issues | `masterplan/_index/MOC-Active-Issues.md` |
| Session history | `masterplan/_index/MOC-Sessions.md` |
| All wikilink targets | `masterplan/_index/Wikilink-Registry.md` |

Full vault instructions: `masterplan/CLAUDE.md`

## What Goes Where -- The Hard Line

| Content | Where | Rule |
|---|---|---|
| Stage specs, acceptance criteria | `masterplan/` | Authoritative. Never duplicate elsewhere. |
| ADRs, audit logs, downstream logs | `masterplan/` | Formal records. |
| Project state, session handoff | `masterplan/STATUS.md` + `HANDOFF.md` | Update per AGENT_CONDUCT.md 1.14. |
| Implementation knowledge, component docs | `masterplan/components/` | How things actually work at code level. |
| Component relationships | `masterplan/connections/` | How things connect to each other. |
| Session history | `masterplan/sessions/` | Preserved history (HANDOFF gets overwritten). |
| Bugs, workarounds | `masterplan/issues/` | Runtime problems and resolutions. |
| Implementation patterns | `masterplan/patterns/` | Reusable approaches. |
| Platform/web design | `PLATFORM_DESIGN.md` (project root) | Separate from engine masterplan. For later. |

## At Session End

1. Update `masterplan/HANDOFF.md` and `masterplan/STATUS.md` (per AGENT_CONDUCT.md 1.14).
2. Create vault notes per AGENT_CONDUCT.md 1.13 (issues, components, connections, patterns).
3. Create a session note in `masterplan/sessions/`.
