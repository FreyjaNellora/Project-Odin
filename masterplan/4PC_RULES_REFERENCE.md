# Four-Player Chess Rules Reference
## Per chess.com Implementation

---

## Board

- 14x14 grid, four 3x3 corners removed = **160 playable squares**
- Files: a-n (14 columns). Ranks: 1-14 (14 rows)
- Removed corners: a1-c3, l1-n3, a12-c14, l12-n14
- The board is cross-shaped: a standard 8x8 center with 3-row extensions on each side

## Starting Positions

| Player | Color | Side | Pieces (back rank) | Pawns |
|--------|-------|------|---------------------|-------|
| Red | Red | South | d1-k1: R N B Q K B N R | d2-k2 |
| Blue | Blue | West | a4-a11: R N B K Q B N R | b4-b11 |
| Yellow | Yellow | North | d14-k14: R N B K Q B N R | d13-k13 |
| Green | Green | East | n4-n11: R N B Q K B N R | m4-m11 |

Blue and Green have K and Q swapped relative to Red/Yellow, maintaining the "queen on her own color" convention from each player's viewpoint.

## Turn Order

Red -> Blue -> Yellow -> Green -> Red -> ... (clockwise)

## Piece Movement

Standard chess movement on the 160-square board. Pieces cannot move through or land on removed corner squares. Knights that would land on a removed corner have that move blocked.

## Pawn Rules

- **FFA:** Promote on the 8th rank relative to starting position (for Red this is rank 9, for Yellow rank 6, etc. -- effectively the middle of the board). Default promotion: 1-point queen (moves like a queen, worth 1 point on capture).
- **Teams:** Promote on the 11th rank. Full underpromotion available.
- Pawns move toward the opposite side of the board from their starting position.
- En passant available (standard rules).
- Double-step from starting rank.

## Castling

Standard rules per player: king and rook unmoved, no pieces between, king does not pass through or into check. Each player has kingside and queenside castling with their own back-rank rooks.

## Check & Checkmate

- Any player can check any opponent's king.
- A move can check multiple kings simultaneously (bonus points in FFA).
- **Critical timing rule:** Checkmate is confirmed only when the affected player's turn arrives. Between the check and that turn, intervening players may alter the position and "rescue" the checked king.

## Elimination

A player is eliminated by: checkmate (when their turn arrives), stalemate (when their turn arrives), resignation, or timeout.

## Scoring (FFA)

| Action | Points |
|--------|--------|
| Capture pawn | +1 |
| Capture knight | +3 |
| Capture bishop | +5 |
| Capture rook | +5 |
| Capture queen (original) | +9 |
| Capture promoted queen (1-pt) | +1 |
| Checkmate active king | +20 |
| Self-stalemate | +20 to stalemated player |
| Check 2 live kings (1 move) | +1 |
| Check 3 live kings (1 move) | +5 |
| Capture dead/grey pieces | 0 |
| Draw (repetition/50-move/insufficient) | +10 each |

## Dead King Walking (DKW)

When a player resigns or times out:
- Pieces turn grey, worth 0 points on capture
- King remains "live" and makes random instant moves (including captures, but earns 0 points)
- Checkmating a dead king still awards points
- Dead kings cannot earn points

## Game End Conditions (FFA)

- Three players eliminated: last player wins by most points
- Two players remain, one leads by 21+ points: "Claim Win" available
- Autoclaim triggers when eliminated 2nd-place leads 3rd-place by 21+ points

## Odin-Specific: Terrain Mode

When a player is eliminated (by any means), their remaining pieces **stay on the board permanently** as immovable, uncapturable terrain for the rest of the game:
- Cannot be captured or moved
- Block movement as if they were walls
- Do NOT include the eliminated king (king is removed)

## Chess960 Adaptation

- Back-rank pieces randomized per standard Chess960 constraints (bishops on opposite colors, king between rooks)
- All four players receive the same randomization, rotated to their respective sides
- Castling rules follow Chess960 conventions (king moves to standard castling target square)
