import { describe, it, expect } from 'vitest';
import {
  isValidSquare,
  fileOf,
  rankOf,
  squareFrom,
  squareName,
  parseSquare,
  isLightSquare,
  startingPosition,
  pieceSymbol,
  PLAYER_COLORS,
} from './board-constants';
import { BOARD_SIZE, TOTAL_SQUARES, VALID_SQUARE_COUNT } from '../types/board';

describe('isValidSquare', () => {
  it('counts exactly 160 valid squares', () => {
    let count = 0;
    for (let i = 0; i < TOTAL_SQUARES; i++) {
      if (isValidSquare(i)) count++;
    }
    expect(count).toBe(VALID_SQUARE_COUNT);
  });

  it('counts exactly 36 invalid corners', () => {
    let count = 0;
    for (let i = 0; i < TOTAL_SQUARES; i++) {
      if (!isValidSquare(i)) count++;
    }
    expect(count).toBe(36);
  });

  it('marks bottom-left corner squares as invalid', () => {
    // Files 0-2, ranks 0-2
    for (let f = 0; f <= 2; f++) {
      for (let r = 0; r <= 2; r++) {
        expect(isValidSquare(squareFrom(f, r))).toBe(false);
      }
    }
  });

  it('marks bottom-right corner squares as invalid', () => {
    // Files 11-13, ranks 0-2
    for (let f = 11; f <= 13; f++) {
      for (let r = 0; r <= 2; r++) {
        expect(isValidSquare(squareFrom(f, r))).toBe(false);
      }
    }
  });

  it('marks top-left corner squares as invalid', () => {
    // Files 0-2, ranks 11-13
    for (let f = 0; f <= 2; f++) {
      for (let r = 11; r <= 13; r++) {
        expect(isValidSquare(squareFrom(f, r))).toBe(false);
      }
    }
  });

  it('marks top-right corner squares as invalid', () => {
    // Files 11-13, ranks 11-13
    for (let f = 11; f <= 13; f++) {
      for (let r = 11; r <= 13; r++) {
        expect(isValidSquare(squareFrom(f, r))).toBe(false);
      }
    }
  });

  it('marks center squares as valid', () => {
    // d4 (file=3, rank=3) should be valid
    expect(isValidSquare(squareFrom(3, 3))).toBe(true);
    // g7 (file=6, rank=6) should be valid
    expect(isValidSquare(squareFrom(6, 6))).toBe(true);
  });

  it('marks edge-adjacent-to-corner squares as valid', () => {
    // d1 (file=3, rank=0) — edge of Red's back rank, next to corner
    expect(isValidSquare(squareFrom(3, 0))).toBe(true);
    // a4 (file=0, rank=3) — edge of Blue's back rank
    expect(isValidSquare(squareFrom(0, 3))).toBe(true);
  });

  it('rejects out-of-bounds indices', () => {
    expect(isValidSquare(-1)).toBe(false);
    expect(isValidSquare(TOTAL_SQUARES)).toBe(false);
    expect(isValidSquare(999)).toBe(false);
  });
});

describe('coordinate helpers', () => {
  it('fileOf extracts correct file', () => {
    expect(fileOf(squareFrom(5, 7))).toBe(5);
    expect(fileOf(squareFrom(0, 0))).toBe(0);
    expect(fileOf(squareFrom(13, 13))).toBe(13);
  });

  it('rankOf extracts correct rank', () => {
    expect(rankOf(squareFrom(5, 7))).toBe(7);
    expect(rankOf(squareFrom(0, 0))).toBe(0);
    expect(rankOf(squareFrom(13, 13))).toBe(13);
  });

  it('squareFrom creates correct index', () => {
    expect(squareFrom(0, 0)).toBe(0);
    expect(squareFrom(13, 0)).toBe(13);
    expect(squareFrom(0, 1)).toBe(BOARD_SIZE);
    expect(squareFrom(5, 7)).toBe(7 * BOARD_SIZE + 5);
  });

  it('squareName produces correct notation', () => {
    expect(squareName(squareFrom(0, 0))).toBe('a1');
    expect(squareName(squareFrom(3, 0))).toBe('d1');
    expect(squareName(squareFrom(13, 13))).toBe('n14');
    expect(squareName(squareFrom(6, 6))).toBe('g7');
  });

  it('parseSquare parses valid notation', () => {
    expect(parseSquare('a1')).toBe(squareFrom(0, 0));
    expect(parseSquare('d1')).toBe(squareFrom(3, 0));
    expect(parseSquare('n14')).toBe(squareFrom(13, 13));
    expect(parseSquare('g7')).toBe(squareFrom(6, 6));
  });

  it('parseSquare returns -1 for invalid input', () => {
    expect(parseSquare('')).toBe(-1);
    expect(parseSquare('z1')).toBe(-1);
    expect(parseSquare('a')).toBe(-1);
    expect(parseSquare('a0')).toBe(-1);
    expect(parseSquare('a15')).toBe(-1);
  });

  it('squareName and parseSquare round-trip all valid squares', () => {
    for (let i = 0; i < TOTAL_SQUARES; i++) {
      if (isValidSquare(i)) {
        const name = squareName(i);
        const parsed = parseSquare(name);
        expect(parsed).toBe(i);
      }
    }
  });
});

describe('isLightSquare', () => {
  it('alternates correctly', () => {
    expect(isLightSquare(0, 0)).toBe(true);  // a1: even+even = even
    expect(isLightSquare(1, 0)).toBe(false); // b1: odd+even = odd
    expect(isLightSquare(0, 1)).toBe(false); // a2: even+odd = odd
    expect(isLightSquare(1, 1)).toBe(true);  // b2: odd+odd = even
  });
});

describe('startingPosition', () => {
  it('returns array of 196 elements', () => {
    const board = startingPosition();
    expect(board.length).toBe(TOTAL_SQUARES);
  });

  it('places exactly 64 pieces total', () => {
    const board = startingPosition();
    const pieces = board.filter(p => p !== null);
    expect(pieces.length).toBe(64);
  });

  it('places 16 pieces per player', () => {
    const board = startingPosition();
    const counts = { Red: 0, Blue: 0, Yellow: 0, Green: 0 };
    for (const p of board) {
      if (p) counts[p.owner]++;
    }
    expect(counts.Red).toBe(16);
    expect(counts.Blue).toBe(16);
    expect(counts.Yellow).toBe(16);
    expect(counts.Green).toBe(16);
  });

  it('places Red king on h1 (file=7, rank=0)', () => {
    const board = startingPosition();
    // R N B Q K B N R → index 4 = King → file 3+4 = 7
    const sq = squareFrom(7, 0);
    expect(board[sq]).toEqual({ pieceType: 'King', owner: 'Red' });
  });

  it('places Blue king on a7 (file=0, rank=6)', () => {
    const board = startingPosition();
    const sq = squareFrom(0, 6);
    expect(board[sq]).toEqual({ pieceType: 'King', owner: 'Blue' });
  });

  it('places Yellow king on g14 (file=6, rank=13)', () => {
    const board = startingPosition();
    const sq = squareFrom(6, 13);
    expect(board[sq]).toEqual({ pieceType: 'King', owner: 'Yellow' });
  });

  it('places Green king on n8 (file=13, rank=7)', () => {
    const board = startingPosition();
    // R N B Q K B N R → index 4 = King → rank 3+4 = 7
    const sq = squareFrom(13, 7);
    expect(board[sq]).toEqual({ pieceType: 'King', owner: 'Green' });
  });

  it('places Red pawns on rank 2 (rank=1), files d-k', () => {
    const board = startingPosition();
    for (let f = 3; f <= 10; f++) {
      expect(board[squareFrom(f, 1)]).toEqual({ pieceType: 'Pawn', owner: 'Red' });
    }
  });

  it('places no pieces on invalid corner squares', () => {
    const board = startingPosition();
    for (let i = 0; i < TOTAL_SQUARES; i++) {
      if (!isValidSquare(i)) {
        expect(board[i]).toBeNull();
      }
    }
  });

  it('has 8 pawns per player', () => {
    const board = startingPosition();
    const pawnCounts = { Red: 0, Blue: 0, Yellow: 0, Green: 0 };
    for (const p of board) {
      if (p && p.pieceType === 'Pawn') pawnCounts[p.owner]++;
    }
    expect(pawnCounts.Red).toBe(8);
    expect(pawnCounts.Blue).toBe(8);
    expect(pawnCounts.Yellow).toBe(8);
    expect(pawnCounts.Green).toBe(8);
  });
});

describe('pieceSymbol', () => {
  it('returns a unicode symbol for each piece type', () => {
    expect(pieceSymbol('King')).toBe('\u2654');
    expect(pieceSymbol('Queen')).toBe('\u2655');
    expect(pieceSymbol('Rook')).toBe('\u2656');
    expect(pieceSymbol('Bishop')).toBe('\u2657');
    expect(pieceSymbol('Knight')).toBe('\u2658');
    expect(pieceSymbol('Pawn')).toBe('\u2659');
    expect(pieceSymbol('PromotedQueen')).toBe('\u2655');
  });
});

describe('PLAYER_COLORS', () => {
  it('defines colors for all four players', () => {
    expect(PLAYER_COLORS.Red).toBeDefined();
    expect(PLAYER_COLORS.Blue).toBeDefined();
    expect(PLAYER_COLORS.Yellow).toBeDefined();
    expect(PLAYER_COLORS.Green).toBeDefined();
  });
});
