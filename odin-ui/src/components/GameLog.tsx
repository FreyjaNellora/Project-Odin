// Enriched game log showing move history with search info per move.
// Each entry: {moveNum}. {Player}: {move} ({eval}cp, d{depth}, {nodes} nodes)
// Left border colored by player.

import { useRef, useEffect } from 'react';
import type { MoveEntry } from '../hooks/useGameState';
import '../styles/GameLog.css';

/** Player colors for left border. */
const PLAYER_BORDER_COLORS: Record<string, string> = {
  Red: '#cc0000',
  Blue: '#0066cc',
  Yellow: '#ccaa00',
  Green: '#00aa44',
};

interface GameLogProps {
  moveHistory: MoveEntry[];
}

export default function GameLog({ moveHistory }: GameLogProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [moveHistory]);

  return (
    <div className="game-log">
      <h3>Game Log</h3>
      <div className="game-log-entries" ref={scrollRef}>
        {moveHistory.length === 0 && (
          <div className="game-log-empty">No moves yet</div>
        )}
        {moveHistory.map((entry, i) => {
          const moveNum = Math.floor(i / 4) + 1;
          const borderColor = PLAYER_BORDER_COLORS[entry.player] || '#666';
          const info = entry.info;

          let detail = '';
          if (info) {
            const parts: string[] = [];
            if (info.scoreCp !== undefined) parts.push(`${info.scoreCp}cp`);
            if (info.depth !== undefined) parts.push(`d${info.depth}`);
            if (info.nodes !== undefined) parts.push(`${info.nodes.toLocaleString()} nodes`);
            if (parts.length > 0) detail = ` (${parts.join(', ')})`;
          }

          return (
            <div
              key={i}
              className="game-log-entry"
              style={{ borderLeftColor: borderColor }}
            >
              <span className="move-num">{moveNum}.</span>
              <span className="move-player" style={{ color: borderColor }}>
                {entry.player}:
              </span>
              <span className="move-text">{entry.move}</span>
              {detail && <span className="move-detail">{detail}</span>}
            </div>
          );
        })}
      </div>
    </div>
  );
}
