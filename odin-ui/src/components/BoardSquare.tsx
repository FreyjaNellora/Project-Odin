// Renders a single board square with optional piece and highlighting.

import type { Piece } from '../types/board';
import {
  isLightSquare,
  LIGHT_SQUARE,
  DARK_SQUARE,
  SELECTED_HIGHLIGHT,
  LAST_MOVE_HIGHLIGHT,
} from '../lib/board-constants';
import PieceIcon from './PieceIcon';

interface BoardSquareProps {
  file: number;
  rank: number;
  x: number;
  y: number;
  size: number;
  piece: Piece | null;
  isSelected: boolean;
  isLastMove: boolean;
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
  onClick,
}: BoardSquareProps) {
  const bgColor = isLightSquare(file, rank) ? LIGHT_SQUARE : DARK_SQUARE;

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

      {/* Piece */}
      {piece && <PieceIcon piece={piece} x={x} y={y} size={size} />}
    </g>
  );
}
