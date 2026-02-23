// SVG board renderer for the 14x14 four-player chess board.
// Renders 160 valid squares (skips 36 invalid corners) with pieces.
// Handles click events for move input — no game logic.
// Supports coordinate labels on squares and mouse-wheel zoom.

import { useState, useCallback, useRef } from 'react';
import type { Piece } from '../types/board';
import { BOARD_SIZE } from '../types/board';
import { isValidSquare, squareFrom } from '../lib/board-constants';
import BoardSquare from './BoardSquare';

/** Pixel size of each square. */
const SQ_SIZE = 46;

/** Total SVG dimension. */
const SVG_SIZE = BOARD_SIZE * SQ_SIZE;

/** Padding around the board for edge labels. */
const PADDING = 20;

/** File letters for coordinate labels. */
const FILE_NAMES = 'abcdefghijklmn';

/** Zoom limits. */
const MIN_ZOOM = 0.5;
const MAX_ZOOM = 2.0;

interface BoardDisplayProps {
  board: (Piece | null)[];
  selectedSquare: number | null;
  lastMoveFrom: number | null;
  lastMoveTo: number | null;
  showCoords: boolean;
  onSquareClick: (squareIndex: number) => void;
}

export default function BoardDisplay({
  board,
  selectedSquare,
  lastMoveFrom,
  lastMoveTo,
  showCoords,
  onSquareClick,
}: BoardDisplayProps) {
  const [zoom, setZoom] = useState(1.0);
  const [transformOrigin, setTransformOrigin] = useState('50% 50%');
  const containerRef = useRef<HTMLDivElement>(null);

  const handleWheel = useCallback(
    (e: React.WheelEvent<HTMLDivElement>) => {
      e.preventDefault();
      const container = containerRef.current;
      if (!container) return;

      // Mouse position relative to container for transform-origin
      const rect = container.getBoundingClientRect();
      const mx = ((e.clientX - rect.left) / rect.width) * 100;
      const my = ((e.clientY - rect.top) / rect.height) * 100;
      setTransformOrigin(`${mx}% ${my}%`);

      const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
      setZoom((prev) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, prev * factor)));
    },
    [],
  );

  const viewBox = `-${PADDING} -${PADDING} ${SVG_SIZE + PADDING * 2} ${SVG_SIZE + PADDING * 2}`;

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
          showCoords={showCoords}
          onClick={() => onSquareClick(sq)}
        />
      );
    }
  }

  return (
    <div className="board-container" ref={containerRef} onWheel={handleWheel}>
      <svg
        viewBox={viewBox}
        xmlns="http://www.w3.org/2000/svg"
        style={{
          transform: `scale(${zoom})`,
          transformOrigin,
        }}
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
