# Project Odin -- Agent Orientation

A four-player chess engine: NNUE + BRS/Paranoid Hybrid + MCTS.

## Model Tier Router

**Mythos/frontier-class models** (Claude Opus 4.x, Claude Sonnet 4.x, or equivalent):
→ Use `CLAUDE_MYTHOS.md` and `masterplan/AGENT_CONDUCT_MYTHOS.md` instead of this file and `AGENT_CONDUCT.md`.
→ Those files contain the same constraints and domain knowledge with procedural scaffolding removed.

**All other models:**
→ Continue reading this file.

---

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

## Critical Rules (Never Forget)

1. **PLAN FIRST + APPROVAL CHAIN.** Always start in plan mode. Write the plan to `.claude/dispatch_comms.jsonl` (type: "plan", tier: 1). Dispatch reviews first, then user approves — do NOT execute until both have signed off. Plans are living documents; adapt mid-execution if needed, following the same chain. (AGENT_CONDUCT 1.0, 1.0a, 1.20)
2. **Depth-4 rule — CORRECTNESS REQUIREMENT.** Only depths divisible by 4 are valid (4, 8, 12...). Depth 3 and any non-multiple-of-4 is PROHIBITED. Training data at non-multiple-of-4 depths must be discarded. (AGENT_CONDUCT 1.2)
3. **Game behavior changes require approval.** Any change to move generation, eval, search, rules, scoring, or board representation: analyze → explain → write Tier 2 to `.claude/dispatch_comms.jsonl` → WAIT for user approval. Never silent. (AGENT_CONDUCT 1.19)
4. **Write to `.claude/dispatch_comms.jsonl`.** Write plans (type: "plan", tier: 1) before execution — include what's changing, why, files affected, risks, and verification method. Log periodic work progress, stuck reports, and Tier 2 requests. Dispatch reviews all plans. Stuck reports are informational — keep working. (AGENT_CONDUCT 1.0, 1.20)
5. **Save point before each stage.** Commit + tag before starting any new stage. (AGENT_CONDUCT 1.21)
6. **Spot-check outputs.** Don't trust "N records generated" — read actual data. Don't trust "X tests passed" — verify coverage. (AGENT_CONDUCT 1.22)
7. **Stages aren't done until the user says so** from testing in the UI.
8. **Pre-closeout re-read.** Before ending any session: re-read AGENT_CONDUCT.md + CLAUDE.md, self-audit the comms log, run `git status`, verify no untracked work, write a closeout comms entry. Non-negotiable. (AGENT_CONDUCT 1.23)
9. **Cleanup agent follows every session.** Dispatch spins a fresh agent to audit the outgoing agent's work — finds uncommitted files, log gaps, untracked artifacts. It reports to Dispatch; it does not fix. (AGENT_CONDUCT 1.24)

## At Session End

1. Update `masterplan/HANDOFF.md` and `masterplan/STATUS.md` (per AGENT_CONDUCT.md 1.14).
2. Create vault notes per AGENT_CONDUCT.md 1.13 (issues, components, connections, patterns).
3. Create a session note in `masterplan/sessions/`.
