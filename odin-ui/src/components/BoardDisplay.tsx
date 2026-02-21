// SVG board renderer for the 14x14 four-player chess board.
// Renders 160 valid squares (skips 36 invalid corners) with pieces.
// Handles click events for move input — no game logic.

import type { Piece } from '../types/board';
import { BOARD_SIZE } from '../types/board';
import { isValidSquare, squareFrom, fileOf, rankOf } from '../lib/board-constants';
import BoardSquare from './BoardSquare';

/** Pixel size of each square. */
const SQ_SIZE = 46;

/** Total SVG dimension. */
const SVG_SIZE = BOARD_SIZE * SQ_SIZE;

/** File letters for coordinate labels. */
const FILE_NAMES = 'abcdefghijklmn';

interface BoardDisplayProps {
  board: (Piece | null)[];
  selectedSquare: number | null;
  lastMoveFrom: number | null;
  lastMoveTo: number | null;
  onSquareClick: (squareIndex: number) => void;
}

export default function BoardDisplay({
  board,
  selectedSquare,
  lastMoveFrom,
  lastMoveTo,
  onSquareClick,
}: BoardDisplayProps) {
  const squares: JSX.Element[] = [];

  // Render squares from top (rank 13) to bottom (rank 0)
  for (let rank = BOARD_SIZE - 1; rank >= 0; rank--) {
    for (let file = 0; file < BOARD_SIZE; file++) {
      const sq = squareFrom(file, rank);
      if (!isValidSquare(sq)) continue;

      const x = file * SQ_SIZE;
      // Visual Y: rank 13 at top (y=0), rank 0 at bottom
      const y = (BOARD_SIZE - 1 - rank) * SQ_SIZE;

      squares.push(
        <BoardSquare
          key={sq}
          file={file}
          rank={rank}
          x={x}
          y={y}
          size={SQ_SIZE}
          piece={board[sq]}
          isSelected={selectedSquare === sq}
          isLastMove={lastMoveFrom === sq || lastMoveTo === sq}
          onClick={() => onSquareClick(sq)}
        />
      );
    }
  }

  return (
    <div className="board-container">
      <svg
        viewBox={`-20 -20 ${SVG_SIZE + 40} ${SVG_SIZE + 40}`}
        xmlns="http://www.w3.org/2000/svg"
      >
        {/* Board background */}
        <rect x={0} y={0} width={SVG_SIZE} height={SVG_SIZE} fill="#2a2a2a" rx={2} />

        {/* Squares and pieces */}
        {squares}

        {/* File labels (bottom) */}
        {Array.from({ length: BOARD_SIZE }, (_, file) => (
          <text
            key={`file-${file}`}
            x={file * SQ_SIZE + SQ_SIZE / 2}
            y={SVG_SIZE + 16}
            textAnchor="middle"
            fontSize={12}
            fill="#999"
          >
            {FILE_NAMES[file]}
          </text>
        ))}

        {/* Rank labels (left) */}
        {Array.from({ length: BOARD_SIZE }, (_, rank) => (
          <text
            key={`rank-${rank}`}
            x={-10}
            y={(BOARD_SIZE - 1 - rank) * SQ_SIZE + SQ_SIZE / 2 + 4}
            textAnchor="middle"
            fontSize={12}
            fill="#999"
          >
            {rank + 1}
          </text>
        ))}
      </svg>
    </div>
  );
}
