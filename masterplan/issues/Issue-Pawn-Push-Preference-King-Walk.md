---
type: issue
tags:
  - type/issue
  - severity/warning
  - area/eval
  - area/search
status: open
date: 2026-02-27
---

# Issue: Pawn Push Preference + King Walk

**Severity:** WARNING
**Status:** Open
**Observed:** 2026-02-27

## Symptom

Red consistently plays 4-5 pawn pushes before developing any pieces, and walks its king into the open. Example game (Red's moves):

```
1. k2k4     (fine — center pawn)
2. f2f3     (bad — blocks knight development on e1)
3. i2i3     (passive — another pawn push)
4. Ki2      (terrible — king walks from h1 to i2!)
5. d2d3     (slow — 4th pawn push)
6. j2j4     (more pawns — 5th pawn push, zero pieces developed)
```

Blue and Green developed knights early. Red did not develop a single piece in 6 moves.

## Suspected Root Causes

### 1. BRS Single-Reply Bias Against Developed Pieces (HIGH)

BRS picks ONE reply per opponent at MIN nodes. If the opponent's "best reply" is to capture/threaten a developed knight, the entire line scores badly for root. Pawns are safe (not worth capturing), so pawn moves look safer to BRS. This is a fundamental BRS model limitation — it over-punishes exposed pieces because opponents get a "free shot."

**Evidence needed:** Trace BRS search tree for Ne1f3 vs f2f3. Does the MIN node opponent reply target the knight?

### 2. King Safety Insufficient King-Walk Penalty (MEDIUM)

The king PST was previously fixed to penalize rank 1 moves (-15cp), but the king walking to i2 suggests the penalty is too small or the pawn shield bonus is miscalibrated after f2f3 + i2i3 create a false "shield" around i2.

**Evidence needed:** Check king PST value at canonical i2 square. Check pawn shield detection — does it think f3/i3 pawns provide a shield for a king on i2?

### 3. Connected Pawn Bonus Reinforces Pawn Pushes (LOW)

The new +8cp connected pawn bonus rewards one-square pawn pushes (f2f3 gets +8cp because e2/g2 defend it). This is small compared to +25cp knight dev bonus, but it's pulling in the wrong direction.

**Possible mitigation:** Only award connected pawn bonus for pawns that have advanced at least 2 ranks, or reduce to +4cp.

## Investigation Plan

Files to examine:
- `odin-engine/src/eval/king_safety.rs` — king walk penalty, pawn shield detection
- `odin-engine/src/eval/pst.rs` — king PST values for walk squares
- `odin-engine/src/eval/multi_player.rs` — threat penalty on exposed pieces
- `odin-engine/src/search/board_scanner.rs` — BRS opponent reply selection
- `odin-engine/src/search/brs.rs` — MIN node logic

## Related Issues

- [[Issue-Hanging-Piece-Eval-Double-Count]] — previous attempt to penalize hanging pieces in eval was reverted (double-counted search threats)
- [[Issue-BRS-Paranoid-Opponent-Modeling]] — related BRS modeling limitation
