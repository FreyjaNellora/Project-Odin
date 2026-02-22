---
type: session
date: 2026-02-21
stage: 7 (post-completion bugfix, second pass)
tags: [stage/07, area/protocol, area/ui]
---

# Session: 2026-02-21 — Stage 7 Bugfix Pass 2

## Goal

Resolve "Red checkmated but engine stops instead of advancing to Blue" — still broken
after session 1 fixes. Three bugs were identified by the user; verify which were already
done and fix the remainder.

## What Happened

**Orientation:** Read STATUS.md, HANDOFF.md. Previous session claimed both bugs fixed
and 199/199 tests passing. User reported the issue was still present at runtime.

**Diagnosis (Section 1.15):** Direct code read of `protocol-parser.ts` found Bug C
immediately — the `eliminated` parser extracted the full remainder as the color string,
including the reason word. No spiraling was needed; the bug was confirmed in one pass.

**Verification of Bugs A + B:** `cargo test` revealed 3 Stage 7 integration tests
failing. These tests assumed only search info lines are emitted but the Bug B fix added
`info string nextturn Blue` to the normal path too. Tests were not updated in session 1.
Fixed the 3 tests to filter `info string` lines from search-info-line counts/checks.

**Bug C fix:** One-line change in `protocol-parser.ts` — `.split(/\s+/)[0]` to extract
first token only. Added 9 regression tests covering `eliminated` (with/without reason),
`nextturn`, and `gameover` parsing.

**Final state:** 199 lib + 305 integration engine tests passing; 54 Vitest tests passing.

## Components Touched

- [[Component-Protocol-Parser]] (UI) — parser fix
- [[stage_07_plain_brs]] — integration tests updated

## Discoveries

The stage 7 integration tests were written before the `nextturn` emission was added to
the normal `handle_go` path (Bug B fix). Test assertions that counted ALL non-bestmove
lines as "search info lines" became wrong as soon as protocol string lines were added.
**Pattern learned:** Integration tests that count info lines should filter by
`info depth` / `info score` format, not by line count or `starts_with("info ")`.

## Issues Created/Resolved

- [[Issue-Checkmate-Detection-DKW-Ordering]] — addendum added for Bug C; fully resolved
