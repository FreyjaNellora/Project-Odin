// Game controls: turn indicator, scores, player status, new game buttons.

import { Player, PLAYERS } from '../types/board';
import { PLAYER_COLORS } from '../lib/board-constants';
import '../styles/GameControls.css';

interface GameControlsProps {
  currentPlayer: Player;
  scores: [number, number, number, number];
  isGameOver: boolean;
  error: string | null;
  onNewGame: (terrain: boolean) => void;
  onEngineMove: () => void;
}

export default function GameControls({
  currentPlayer,
  scores,
  isGameOver,
  error,
  onNewGame,
  onEngineMove,
}: GameControlsProps) {
  return (
    <div className="game-controls">
      {/* Turn indicator */}
      <div className="turn-indicator">
        <span className="label">Turn:</span>
        <span
          className="player-name"
          style={{ color: PLAYER_COLORS[currentPlayer] }}
        >
          {currentPlayer}
        </span>
      </div>

      {/* Scores */}
      <div className="scores">
        {PLAYERS.map((player, i) => (
          <div
            key={player}
            className={`score-row ${player === currentPlayer ? 'active' : ''}`}
          >
            <span
              className="score-player"
              style={{ color: PLAYER_COLORS[player] }}
            >
              {player}
            </span>
            <span className="score-value">{scores[i]}</span>
          </div>
        ))}
      </div>

      {/* Error display */}
      {error && <div className="error-display">{error}</div>}

      {/* Game over indicator */}
      {isGameOver && <div className="game-over">Game Over</div>}

      {/* Control buttons */}
      <div className="control-buttons">
        <button
          className="btn-engine"
          onClick={onEngineMove}
          disabled={isGameOver}
        >
          Engine Move
        </button>
        <button className="btn-new" onClick={() => onNewGame(false)}>
          New Game
        </button>
        <button className="btn-new" onClick={() => onNewGame(true)}>
          New Game (Terrain)
        </button>
      </div>
    </div>
  );
}
