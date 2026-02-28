# Engine Self-Play Game Analysis — Prompt for Claude

You are analyzing a game played by the Odin engine (v0.4.3-narrowing), a 4-player chess engine. The game was NOT played to completion. Your job is to diagnose the bugs the user identified, and look for any other issues.

## Board Layout & Coordinate System

14×14 board with 3×3 corner cutoffs. Files a-n (left→right, 0-13). Ranks 1-14 (bottom→top, 0-13 internally).

Players:
- **Red** (south, rank 1-2): pieces on ranks 1-2, faces north (+rank). King h1, Queen g1.
- **Blue** (west, files a-b): pieces on files a-b, faces east (+file). King a7, Queen a8.
- **Yellow** (north, ranks 13-14): pieces on ranks 13-14, faces south (-rank). King g14, Queen h14.
- **Green** (east, files m-n): pieces on files m-n, faces west (-file). King n8, Queen n7.

Turn order: R→B→Y→G→R→...

Starting material per player: 8P + 2N + 2B + 2R + Q + K = 4300cp (P=100, N=300, B=500, R=500, Q=900, K=0).

## Engine Architecture

**Search:** Best-Reply Search (BRS) with alpha-beta, depth 7. Iterative deepening. At MAX nodes (root player's turn), all legal moves are explored. At MIN nodes (each opponent's turn), a SINGLE reply is selected via hybrid scoring (harm_to_root × likelihood + objective_strength × (1-likelihood)).

**Key constants:**
- `LIKELIHOOD_BASE_TARGETS_ROOT = 0.7` — base probability that an opponent targets the root player (known to be too high / too paranoid)
- `LIKELIHOOD_EXPOSED_PENALTY = 0.3` — penalty to likelihood if opponent is exposed
- `LIKELIHOOD_BASE_NON_ROOT = 0.2` — base likelihood for moves NOT targeting root

This means opponents are modeled as ~80% paranoid (attacking root) and ~20% realistic. This is a **known issue** — the engine over-assumes opponents cooperate against it.

**Eval formula:** `material + positional(PST) + king_safety - threat_penalty + lead_penalty + ffa_points + relative_material_advantage`

**Piece values:** P=100, N=300, B=500, R=500, Q=900, K=0

**PST design (recently fixed for 4-player symmetry):**
- Knight: center 4×4 peak +12cp, back-rank penalty -8cp, first hop ≈+10cp
- Bishop: center peak +32cp, back-rank penalty -15cp, fianchetto rank +15cp
- Rook: royal aisles (files g,h + ranks 7,8) peak +18cp center. No home penalty.
- Queen: center peak +8cp, back-rank penalty -5cp. Modest — queen is so mobile.
- King: home bonus +30cp (castled corners), center penalty -50cp.
- Pawn: forward advancement up to +50cp at promotion rank.

**King safety:** Pawn shield +50cp per shield pawn (max 3=150cp). Open file penalty -40cp per open file near king. Attacker pressure -25cp base + -20cp per extra attack square.

**Progressive narrowing:** At depth 7+, opponent moves truncated to top 5. Root-capture protection: moves capturing root player's pieces are exempt from truncation.

**BRS depth 7 turn structure for Red searching:** Red(MAX, ply0) → Blue(MIN) → Yellow(MIN) → Green(MIN) → Red(MAX, ply4) → Blue(MIN) → Yellow(MIN) → Green(MIN). Red gets 2 MAX turns, 4 plies apart (~1.75 rounds look-ahead).

**TT is fresh per search** (not persisted between moves — known issue).

## Known Issues (before this game)

1. Hybrid scoring too paranoid (80/20 blend) — opponents modeled as coordinating against root
2. TT not player-aware (hash doesn't include root_player)
3. TT fresh per `go` command (not persisted)
4. Depth 7 asymmetry: root gets 2 MAX moves 4 plies apart — can't see 2-move tactical plans

---

## The Game Log

Format: `Move#.Player:move(eval, depth, nodes)`

```
1.Red:e2e4(4456cp, d7, 7,441 nodes)
1.Blue:b9d9(4402cp, d7, 11,625 nodes)
1.Yellow:d13d11(4264cp, d7, 2,077 nodes)
1.Green:m5k5(4506cp, d7, 8,276 nodes)
2.Red:Nj1i3(4483cp, d7, 13,241 nodes)
2.Blue:b6d6(4422cp, d7, 18,964 nodes)
2.Yellow:i13i11(4443cp, d7, 6,474 nodes)
2.Green:m10l10(4441cp, d7, 2,117 nodes)
3.Red:Bf1e2(4486cp, d7, 20,090 nodes)
3.Blue:Na5c6(4465cp, d7, 16,519 nodes)
3.Yellow:k13k12(4176cp, d7, 6,645 nodes)
3.Green:Bn6m5(3803cp, d7, 15,034 nodes)
4.Red:Be2n11(4523cp, d7, 30,292 nodes)
4.Blue:b5c5(4595cp, d7, 12,355 nodes)
4.Yellow:Nj14i12(4187cp, d7, 9,111 nodes)
4.Green:m6k6(3840cp, d7, 29,748 nodes)
5.Red:k2k4(5076cp, d7, 5,026 nodes)
5.Blue:Ba6c4(4612cp, d7, 16,950 nodes)
5.Yellow:k12k11(4153cp, d7, 26,788 nodes)
5.Green:l10k10(3950cp, d7, 36,575 nodes)
6.Red:Bn11h5(5104cp, d7, 18,004 nodes)
6.Blue:Na10c11(4470cp, d7, 53,083 nodes)
6.Yellow:Rd14d12(3811cp, d7, 28,044 nodes)
6.Green:Bm5j8(3945cp, d7, 21,462 nodes)
7.Red:Ne1f3(5110cp, d7, 19,707 nodes)
7.Blue:Bc4i10(4456cp, d7, 58,773 nodes)
7.Yellow:g13g12(3942cp, d7, 10,875 nodes)
7.Green:Nn10l9(4029cp, d7, 49,716 nodes)
8.Red:d2d3(5199cp, d7, 12,604 nodes)
8.Blue:d9e9(4293cp, d7, 34,817 nodes)
8.Yellow:Bf14h12(4093cp, d7, 61,310 nodes)
8.Green:m8k8(3823cp, d7, 83,435 nodes)
9.Red:j2j3(5212cp, d7, 57,421 nodes)
9.Blue:d6e6(4243cp, d7, 26,837 nodes)
9.Yellow:e13e11(3948cp, d7, 91,069 nodes)
9.Green:Bn9l7(3801cp, d7, 44,308 nodes)
10.Red:Rk1k2(5057cp, d7, 72,380 nodes)
10.Blue:b10d10(4286cp, d7, 58,094 nodes)
10.Yellow:e11d10(3969cp, d7, 110,146 nodes)
10.Green:Nn5l6(3824cp, d7, 22,270 nodes)
11.Red:Ni3j1(5142cp, d7, 42,967 nodes)
11.Blue:e9f9(4198cp, d7, 55,625 nodes)
```

Final per-player evals:
```
R: 5146  B: 4327  Y: 4631  G: 3897
```

---

## Full Search Info (per-depth PV traces)

<details>
<summary>Click to expand full search traces for every move</summary>

### Move 1 — Red: e2e4
```
depth 1: f2f4 (4432cp)
depth 2: f2f4 (4432cp)
depth 3: f2f4 (4432cp)
depth 4: f2f4 (4432cp)
depth 5: f2f4 (4442cp)
depth 6: f2f4 (4450cp)
depth 7: e2e4 (4456cp) ← switches at final depth
```

### Move 1 — Blue: b9d9
```
depth 5: a5c6 (4366cp)
depth 6: a10c9 (4366cp)
depth 7: b9d9 (4402cp) ← switches at final depth
```

### Move 1 — Yellow: d13d11
```
depth 2-5: e13e12 preferred
depth 6: d13d11 (4389cp) ← switches
depth 7: d13d11 (4264cp) ← eval drops but move holds
```

### Move 1 — Green: m5k5
```
depth 1-4: m6k6 preferred
depth 5-6: m10k10 preferred
depth 7: m5k5 (4506cp) ← switches at final depth
```

### Move 2 — Red: Nj1i3
```
depth 1-5: f1e2 preferred (Bf1e2)
depth 6: f2f4
depth 7: j1i3 (4483cp) ← switches to knight
```

### Move 2 — Blue: b6d6
```
depth 1-5: a5c6 preferred
depth 6: d9e9
depth 7: b6d6 (4422cp) ← pawn push to d6
```

### Move 3 — Green: Bn6m5
```
depth 1-5: m8k8 (rook pawn push)
depth 6: n6m5 (bishop develops, 3770cp)
depth 7: n6m5 (3803cp)
```
Note: Green's eval is already notably lower (3803 vs 4400+ for others).

### Move 4 — Red: Be2n11
```
depth 2-6: e2n11 (4514cp) — bishop snipe across board
depth 7: e2n11 (4523cp)
```
Red's bishop shoots to n11 (near Green's back rank area).

### Move 4 — Green: m6k6
```
depth 1-4: m8k8 or m5j8
depth 6: none? multiple changes
depth 7: m6k6 (3840cp) — another pawn push
```
Green keeps pushing pawns instead of developing.

### Move 5 — Red: k2k4
```
depth 4-7: k2k4 (5076cp)
```
Red's eval jumps from 4523 to 5076 (+553cp). Something happened — probably Red sees material gain in the tree.

### Move 6 — Red: Bn11h5
```
All depths: n11h5 (bishop retreats/repositions)
depth 7: 5104cp
```

### Move 8 — Green: m8k8
```
depth 1-5: m8k8 (pawn push)
depth 7: m8k8 (3823cp, 83,435 nodes — huge tree)
```
Green pushes ANOTHER pawn (3rd pawn push in a row after m5k5, m6k6, m10l10, l10k10, m8k8).

### Move 9 — Blue: d6e6
```
PV mostly showed e9f9
depth 7: bestmove d6e6 (4243cp) — but PV was "none"
```
Blue pushes d6→e6 pawn. This is UNDEFENDED pawn advance.

### Move 10 — Yellow: e11d10
```
depth 6: e11d10 (4239cp)
depth 7: e11d10 (3969cp) — eval drops at depth 7 but still chooses it
```
Yellow captures Blue's d10 pawn (e11xd10). 110,146 nodes — the biggest search in the game.

### Move 11 — Red: Ni3j1
```
depth 1-5: i1j2, h2h3, i1k3 considered
depth 7: i3j1 (5142cp) — KNIGHT UNDEVELOPS back to j1
```
Red moves Ni3 BACK to j1 — undevelopment!

### Move 11 — Blue: e9f9
```
depth 1-5: e9f9
depth 7: e9f9 (4198cp)
```
Blue pushes another pawn (e9→f9).

</details>

---

## Bugs Identified by User

### Bug 1: Green exposes rook and loses it to Red early
Look at Green's sequence: m5k5 (pawn), m10l10 (pawn), Bn6m5 (bishop), m6k6 (pawn), l10k10 (pawn), Bm5j8 (bishop), Nn10l9 (knight), m8k8 (pawn), Bn9l7 (bishop), Nn5l6 (knight).

Green pushes lots of pawns. Where is the rook exposure? Red's Be2→n11 snipe is suspicious. Did Red's bishop (on n11 or h5) threaten/capture Green's rook?

### Bug 2: Blue pushes undefended pawns
Blue's moves: b9d9, b6d6, Na5c6, b5c5, Ba6c4, Na10c11, Bc4i10, d9e9, d6e6, b10d10, e9f9.

Multiple pawn pushes: b9d9, b6d6, b5c5, d9e9, d6e6, b10d10, e9f9. Are these defended? Blue is pushing pawns into the center while moving pieces away — especially d6e6 which advances into exposed territory.

### Bug 3: Red's knight undevelops at end of log
Move 11: Red plays Ni3→j1. The knight RETREATS to its starting square. Red's eval is 5142cp (confident), so the engine thinks this is a GOOD move. Why?

Possible causes:
- At depth 7, Red gets 2 MAX moves. Maybe the knight on i3 was "in the way" of some deeper plan?
- The hybrid scoring might model opponents as being about to capture the knight?
- TT fresh per move means the engine has no memory of development value
- PST for knight at j1 (rank 0, file 9) = -5cp. PST at i3 (rank 2, file 8) = +5cp. So the PST alone discourages this by 10cp. Something in the search overrides it.

---

## Your Task

1. **For each bug**, explain the likely root cause in the engine. Is it:
   - PST values?
   - BRS paranoid modeling (LIKELIHOOD_BASE_TARGETS_ROOT = 0.7)?
   - Search depth limitation (only sees 2 root moves in 7 plies)?
   - Progressive narrowing pruning good opponent moves?
   - King safety eval issues?
   - Something else?

2. **Find any other issues** in the game you can spot beyond the 3 listed bugs.

3. **Propose specific code fixes** with file paths and what to change. Prioritize fixes that would have the biggest impact.

4. **Rate the severity** of each bug: Critical / Major / Minor.

Focus on actionable diagnosis. The codebase is in Rust at `odin-engine/src/`. The key files are:
- `eval/pst.rs` — piece-square tables
- `eval/mod.rs` — eval_for_player formula
- `eval/king_safety.rs` — king safety component
- `eval/multi_player.rs` — threat/lead penalty
- `eval/material.rs` — material counting
- `eval/values.rs` — piece values
- `search/brs.rs` — BRS search with alpha-beta
- `search/board_scanner.rs` — hybrid reply scoring, progressive narrowing
