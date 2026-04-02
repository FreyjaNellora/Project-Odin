# Project Odin -- Agent Orientation (Mythos)

A four-player chess engine: NNUE + BRS/Paranoid Hybrid + MCTS.

> **This file is for Mythos/frontier-class models.** Standard models: use `CLAUDE.md`.
>
> Governance rules live in `masterplan/AGENT_CONDUCT_MYTHOS.md`.

---

## Orientation

Read `masterplan/STATUS.md` and `masterplan/HANDOFF.md`. Explore `masterplan/AGENT_CONDUCT_MYTHOS.md`, `masterplan/DECISIONS.md`, and `masterplan/MASTERPLAN.md` as needed.

---

## Quick Understanding (Obsidian Vault)

| You want to know... | Read |
|---|---|
| Big picture navigation | `masterplan/_index/MOC-Project-Odin.md` |
| Tier 1 stages, logs, invariants | `masterplan/_index/MOC-Tier-1-Foundation.md` |
| Known issues | `masterplan/_index/MOC-Active-Issues.md` |
| Session history | `masterplan/_index/MOC-Sessions.md` |
| All wikilink targets | `masterplan/_index/Wikilink-Registry.md` |

Full vault instructions: `masterplan/CLAUDE.md`

---

## What Goes Where -- The Hard Line

| Content | Where | Rule |
|---|---|---|
| Stage specs, acceptance criteria | `masterplan/` | Authoritative. Never duplicate elsewhere. |
| ADRs, audit logs, downstream logs | `masterplan/` | Formal records. |
| Project state, session handoff | `masterplan/STATUS.md` + `HANDOFF.md` | Update at session end. |
| Implementation knowledge, component docs | `masterplan/components/` | How things actually work at code level. |
| Component relationships | `masterplan/connections/` | How things connect to each other. |
| Session history | `masterplan/sessions/` | Preserved history (HANDOFF gets overwritten). |
| Bugs, workarounds | `masterplan/issues/` | Runtime problems and resolutions. |
| Implementation patterns | `masterplan/patterns/` | Reusable approaches. |
| Platform/web design | `PLATFORM_DESIGN.md` (project root) | Separate from engine masterplan. For later. |

---

## At Session End

Update `masterplan/HANDOFF.md` and `masterplan/STATUS.md`. Create vault notes (issues, components, connections, patterns) per AGENT_CONDUCT_MYTHOS.md §1.13. Create a session note in `masterplan/sessions/`.
