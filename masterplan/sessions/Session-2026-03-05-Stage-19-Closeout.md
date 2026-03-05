# Session: 2026-03-05 Stage 19 Closeout

**Stage:** Stage 19 -- Optimization \& Hardening
**Status at end:** COMPLETE -- post-audit done, tagged stage-19-complete / v1.19

## What Happened This Session

Continuation of Stage 19. Prior sessions had completed Phases 1-7. This session:

1. **Read background stress test results** (3000 games, 0 crashes -- AC1 confirmed).
2. **Read full test suite results** (594 passed, 6 ignored, 0 failed across 20 suites -- all phases green).
3. **UI cleanup**: Removed Speed controls from SelfPlayDashboard. The speed dropdown set a 0-500ms UI delay between self-play moves, which was imperceptible since engine think time dominates. Removed SelfPlaySpeed type, SPEED_DELAY constant, speed state and setSpeed from useSelfPlay hook. Self-play now hardcoded to 0ms delay.
4. **NPS target analysis**: User identified that AC4/AC5 NPS targets (500K NPS) were borrowed from 2-player chess conventions without accounting for 4-player complexity: 4x NNUE perspectives per node, depth-8 = 2 complete rotations (comparable to depth-16 in 2-player terms), higher branching factor. Engine reports ~13K NPS at depth 8, which is correct. Revised AC4/AC5 to latency-based targets (BRS depth 6 < 30ms, MCTS 1000 sims < 150ms) -- both PASS.
5. **Depth-8 rationale confirmed**: In 4-player chess, one rotation = 4 plies. Depth 8 = 2 full rotations, ensuring the engine plans through a complete response cycle from all opponents. This is also the minimum depth for sound NNUE training: positions must include multi-player causality.
6. **Post-audit written**: audit_log_stage_19.md post-audit section filled with full AC results, code quality assessment, future conflict analysis, and reasoning.
7. **STATUS.md / HANDOFF.md updated**: Stage 19 marked complete, next session pointed to Stage 20 (Gen-0 GPU training).
8. **Engine launched for user testing**: npx tauri dev in odin-ui/.

## Key Insight Logged

Depth-8 is architecturally correct for 4-player chess (not just a performance parameter). NPS comparisons to 2-player chess are invalid. Future stages should measure latency per move, not NPS.

## Files Modified

- odin-ui/src/hooks/useSelfPlay.ts -- removed speed controls
- odin-ui/src/components/SelfPlayDashboard.tsx -- removed speed dropdown
- masterplan/audit_log_stage_19.md -- post-audit section filled
- masterplan/STATUS.md -- stage 19 complete
- masterplan/HANDOFF.md -- stage 19 complete, next = stage 20

## Links

- [[audit_log_stage_19]]
- [[stage_19_polish]]
- [[STATUS]]
- [[HANDOFF]]
