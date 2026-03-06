---
status: open
severity: note
stage: 19
area: ui
tags:
  - stage/19
  - area/ui
  - severity/note
last_updated: 2026-03-06
---

# Issue: React Hooks Queue Error on UI Load

## Description

UI occasionally shows "Should have a queue. You are likely calling Hooks conditionally" error on load. Stack trace points to `useState` in `App.tsx:22` (`useSelfPlay` call) through Vite-bundled chunks.

## Investigation

All 4 hook files examined -- no conditional hooks found:
- `useEngine.ts` -- 4 useState, 1 useEffect, 3 useCallback, 1 useRef (all top-level)
- `useGameState.ts` -- 20+ useState, many useCallback/useRef (all top-level)
- `useSelfPlay.ts` -- 4 useState, 5 useRef, 4 useCallback, 1 useEffect, 3 useMemo (all top-level)
- `App.tsx` -- 3 custom hooks + 1 useState + 1 useEffect (all top-level)

## Likely Cause

Vite HMR (Hot Module Replacement) artifact. When dev server hot-reloads modules, React's fiber tree can get out of sync with the new module's hook call order. The Vite chunk hash (`PYPE4BMF`) in the stack trace supports this.

## Workaround

Refresh the page or restart the Tauri dev server. Does not affect production builds.

## Impact

None -- UI-only, does not affect engine, datagen, or training pipeline. Only observed in dev mode.

## Resolution

(Pending -- monitor if it reproduces on fresh page loads without HMR)
