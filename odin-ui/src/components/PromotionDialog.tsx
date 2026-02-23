// Promotion piece selection dialog.
// Shown as an overlay when a pawn reaches a promotion rank.

import { useEffect } from 'react';
import type { Player } from '../types/board';
import type { PromotionChoice } from '../hooks/useGameState';
import { PLAYER_COLORS, pieceSymbol } from '../lib/board-constants';
import './PromotionDialog.css';

const PROMOTION_OPTIONS: { piece: PromotionChoice; type: 'PromotedQueen' | 'Rook' | 'Bishop' | 'Knight' }[] = [
  { piece: 'w', type: 'PromotedQueen' },
  { piece: 'r', type: 'Rook' },
  { piece: 'b', type: 'Bishop' },
  { piece: 'n', type: 'Knight' },
];

interface PromotionDialogProps {
  player: Player;
  onSelect: (piece: PromotionChoice) => void;
  onCancel: () => void;
}

export default function PromotionDialog({ player, onSelect, onCancel }: PromotionDialogProps) {
  // Close on Escape key
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [onCancel]);

  return (
    <div className="promotion-backdrop" onClick={onCancel}>
      <div className="promotion-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="promotion-title">Promote to:</div>
        <div className="promotion-options">
          {PROMOTION_OPTIONS.map(({ piece, type }) => (
            <button
              key={piece}
              className="promotion-option"
              onClick={() => onSelect(piece)}
              title={type === 'PromotedQueen' ? 'Queen' : type}
            >
              <span
                className="promotion-piece"
                style={{ color: PLAYER_COLORS[player] }}
              >
                {pieceSymbol(type)}
              </span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
