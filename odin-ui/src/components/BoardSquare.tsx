// Renders a single board square with optional piece, highlighting, and coordinate label.

import type { Piece } from '../types/board';
import {
  isLightSquare,
  LIGHT_SQUARE,
  DARK_SQUARE,
  SELECTED_HIGHLIGHT,
  LAST_MOVE_HIGHLIGHT,
  squareName,
  squareFrom,
} from '../lib/board-constants';
import PieceIcon from './PieceIcon';

/** Subtle coordinate label color — dark text on light squares, light text on dark squares. */
const COORD_LABEL_LIGHT = 'rgba(0, 0, 0, 0.35)';
const COORD_LABEL_DARK = 'rgba(255, 255, 255, 0.35)';

interface BoardSquareProps {
  file: number;
  rank: number;
  x: number;
  y: number;
  size: number;
  piece: Piece | null;
  isSelected: boolean;
  isLastMove: boolean;
  showCoords: boolean;
  onClick: () => void;
}

export default function BoardSquare({
  file,
  rank,
  x,
  y,
  size,
  piece,
  isSelected,
  isLastMove,
  showCoords,
  onClick,
}: BoardSquareProps) {
  const isLight = isLightSquare(file, rank);
  const bgColor = isLight ? LIGHT_SQUARE : DARK_SQUARE;
  const coordColor = isLight ? COORD_LABEL_LIGHT : COORD_LABEL_DARK;

  return (
    <g onClick={onClick} style={{ cursor: 'pointer' }}>
      {/* Square background */}
      <rect x={x} y={y} width={size} height={size} fill={bgColor} />

      {/* Last move highlight */}
      {isLastMove && (
        <rect x={x} y={y} width={size} height={size} fill={LAST_MOVE_HIGHLIGHT} />
      )}

      {/* Selected square highlight */}
      {isSelected && (
        <rect x={x} y={y} width={size} height={size} fill={SELECTED_HIGHLIGHT} />
      )}

      {/* Coordinate label (bottom-left corner of square) */}
      {showCoords && (
        <text
          x={x + 2}
          y={y + size - 2}
          fontSize={8}
          fill={coordColor}
          style={{ pointerEvents: 'none', userSelect: 'none' }}
        >
          {squareName(squareFrom(file, rank))}
        </text>
      )}

      {/* Piece */}
      {piece && <PieceIcon piece={piece} x={x} y={y} size={size} />}
    </g>
  );
}
