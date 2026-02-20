# Vault Instructions for AI Agents

This folder (`masterplan/`) is both the authoritative project documentation AND an Obsidian vault. The existing files (MASTERPLAN.md, stage specs, audit logs, downstream logs, DECISIONS.md, etc.) are indexed by Obsidian with [[wikilinks]] connecting them.

## Two-Phase Workflow

### Phase 1: Before Starting Work — Full Read

Do a proper read of the project. Follow the protocol in [[AGENT_CONDUCT]] Section 1.1:

1. [[STATUS]] -- Where is the project?
2. [[HANDOFF]] -- What was the last session doing?
3. [[AGENT_CONDUCT]] Section 1.1 -- Stage entry protocol
4. [[DECISIONS]] -- Architectural decisions (if new to project or starting a new stage)
5. [[MASTERPLAN]] -- Full spec for the stage you're working on (Section 4)
6. Upstream audit and downstream logs per the dependency chain

### Phase 2: While Working — Quick Wiki Lookup

Once you're mid-task and need to look something up fast, use the wiki:

| You need to find... | Start here |
|---|---|
| Which tier/stage am I in? | [[MOC-Project-Odin]] |
| Stage 0-5 specs, logs, invariants | [[MOC-Tier-1-Foundation]] |
| Stage 6-7 specs, logs, traits | [[MOC-Tier-2-Simple-Search]] |
| Stage 8-11 specs, logs, hybrid/TT/MCTS | [[MOC-Tier-3-Strengthen-Search]] |
| Stage 12-13 specs, logs, measurement | [[MOC-Tier-4-Measurement]] |
| Stage 14-16 specs, logs, NNUE | [[MOC-Tier-5-Learn]] |
| Stage 17-19 specs, logs, polish | [[MOC-Tier-6-Polish]] |
| Open bugs and workarounds | [[MOC-Active-Issues]] |
| What happened in past sessions | [[MOC-Sessions]] |
| All wikilink targets in the vault | [[Wikilink-Registry]] |

From any MOC, follow [[wikilinks]] to jump directly to the stage spec, its audit log, or its downstream log. Each of those files links back to the others.

## What Agents Add to the Vault

The existing files are the source of truth. Agents add NEW notes to capture knowledge that doesn't belong in the existing files:

| Folder | What goes here | When to create |
|---|---|---|
| `sessions/` | Build session journals | Every session end (preserves what [[HANDOFF]] overwrites) |
| `issues/` | Bugs, workarounds, edge cases | When you encounter a problem |
| `components/` | How a module actually works at impl level | First time implementing a component |
| `connections/` | How two components interact | When you discover cross-component behavior |
| `patterns/` | Reusable implementation approaches | When a pattern emerges |

Templates for each type are in `_templates/`.

### Rules for New Notes

1. **Never duplicate existing masterplan content.** Link to it with [[wikilinks]].
2. **Check [[Wikilink-Registry]] before creating any link.** Reuse existing targets. Only create new ones when no existing target covers the concept. See [[AGENT_CONDUCT]] Section 1.12 for the full wikilink discipline rules.
3. **If you create a new wikilink target, add it to [[Wikilink-Registry]] immediately.**
4. **Use the templates.** They ensure consistent structure.
5. **Update the relevant MOC.** If you create an issue, add it to [[MOC-Active-Issues]]. If you create a session note, add it to [[MOC-Sessions]].

## File Naming

- Session notes: `Session-YYYY-MM-DD-Brief-Title.md`
- Issue notes: `Issue-Brief-Description.md`
- Component notes: `Component-Name.md`
- Connection notes: `Connection-A-to-B.md`
- Pattern notes: `Pattern-Name.md`

## At Session End

1. Update [[HANDOFF]] and [[STATUS]] (per [[AGENT_CONDUCT]] 1.13)
2. Create a session note in `sessions/`
