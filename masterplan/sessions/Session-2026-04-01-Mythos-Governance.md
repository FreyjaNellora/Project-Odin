---
type: session
date: 2026-04-01
stage: Post-Stage-20 (governance infrastructure)
tags:
  - type/session
  - area/governance
---

# Session: Mythos Governance Adaptation

**Date:** 2026-04-01
**Stage:** Post-Stage-20 (no engine code changed)
**Focus:** Adapting agent governance files for Mythos/frontier-class models

---

## What Was Done

Created a dual-file governance system so frontier-class models receive goal-oriented instructions while all domain knowledge, hard constraints, and pitfall tables are preserved.

### Files Created

- **`CLAUDE_MYTHOS.md`** (project root) — condensed orientation for Mythos models. Orientation section reduced to one directive ("Read STATUS.md + HANDOFF.md, explore as needed"). What-Goes-Where table kept verbatim. Session-end condensed to one sentence.

- **`masterplan/AGENT_CONDUCT_MYTHOS.md`** — condensed conduct rules. ~500 lines vs ~1,160 in the original. Strategy:
  - **Kept verbatim:** Search depth policy (§1.2), First Law invariants table (§1.3), autonomy boundary tables (§1.4), naming convention table (§1.5), named constants table (§1.5), decision principles (§1.8), blocking issue resolution table (§1.10), all of Section 2 (26-category audit checklist), all of Section 4 (what tracing cannot catch).
  - **Condensed to intent:** Stage entry protocol (7 steps → one paragraph), debugging discipline (anti-spiral flowchart + table → 5 core rules), session-end protocol (5 steps → one sentence), task tracking lifecycle (8 steps → key rules + when-to-use table), diagnostic observer workflow (10 steps → constraint statement + table).
  - **Removed:** Compensating guardrails for failure modes frontier models don't have (spiral detection flowchart, step-counting ceremony, repeated "why" explanations for obvious constraints).

### Files Modified

- **`CLAUDE.md`** — model-tier router block added at the top, above "Before You Start". Directs Mythos/frontier-class models to the `_MYTHOS` files.

- **`masterplan/DECISIONS.md`** — ADR-018 added: "Dual Governance Files -- Standard vs. Mythos". Documents the two-file strategy, what's kept vs. removed in Mythos variants, and the maintenance rule for keeping them in sync.

---

## What Was NOT Done

- No engine code was changed.
- No NNUE weights changed.
- Wikilink-Registry and MOC files updated as part of this session per vault protocol.
- HANDOFF.md and STATUS.md not updated (this session did not advance any stage or change project state that would affect the next engineering session).

---

## Vault Protocol Notes

- New wikilink targets added: `[[CLAUDE_MYTHOS]]`, `[[AGENT_CONDUCT_MYTHOS]]`, `[[Session-2026-04-01-Mythos-Governance]]`.
- ADR-018 added to DECISIONS.md; wikilink target `[[DECISIONS]]` already exists and covers it.

---

## Maintenance Rule (from ADR-018)

When `AGENT_CONDUCT.md` is updated:
- New domain knowledge (pitfall table, named constant, hard constraint) → add to `AGENT_CONDUCT_MYTHOS.md` too.
- New procedural scaffolding → evaluate: new knowledge (add) or compensating procedure (omit).
