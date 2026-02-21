// Game controls: turn indicator, scores, play mode, speed, new game buttons.

import type { Player } from '../types/board';
import { PLAYERS } from '../types/board';
import { PLAYER_COLORS } from '../lib/board-constants';
import type { PlayMode } from '../hooks/useGameState';
import '../styles/GameControls.css';

interface GameControlsProps {
  currentPlayer: Player;
  scores: [number, number, number, number];
  isGameOver: boolean;
  error: string | null;
  playMode: PlayMode;
  humanPlayer: Player | null;
  engineDelay: number;
  isPaused: boolean;
  onNewGame: (terrain: boolean) => void;
  onEngineMove: () => void;
  onSetPlayMode: (mode: PlayMode) => void;
  onSetHumanPlayer: (player: Player | null) => void;
  onSetEngineDelay: (ms: number) => void;
  onTogglePause: () => void;
}

const MODE_LABELS: Record<PlayMode, string> = {
  manual: 'Manual',
  'semi-auto': 'Semi-Auto',
  'full-auto': 'Full Auto',
};

export default function GameControls({
  currentPlayer,
  scores,
  isGameOver,
  error,
  playMode,
  humanPlayer,
  engineDelay,
  isPaused,
  onNewGame,
  onEngineMove,
  onSetPlayMode,
  onSetHumanPlayer,
  onSetEngineDelay,
  onTogglePause,
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

      {/* Play mode selector */}
      <div className="control-section">
        <span className="section-label">Mode</span>
        <div className="mode-selector">
          {(['manual', 'semi-auto', 'full-auto'] as PlayMode[]).map((mode) => (
            <button
              key={mode}
              className={`btn-mode ${playMode === mode ? 'active' : ''}`}
              onClick={() => onSetPlayMode(mode)}
            >
              {MODE_LABELS[mode]}
            </button>
          ))}
        </div>
      </div>

      {/* Player selector (semi-auto only) */}
      {playMode === 'semi-auto' && (
        <div className="control-section">
          <span className="section-label">Play as</span>
          <div className="player-selector">
            {PLAYERS.map((player) => (
              <button
                key={player}
                className={`btn-player ${humanPlayer === player ? 'active' : ''}`}
                style={{
                  color: PLAYER_COLORS[player],
                  borderColor: humanPlayer === player ? PLAYER_COLORS[player] : undefined,
                }}
                onClick={() => onSetHumanPlayer(player)}
              >
                {player}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Speed control */}
      <div className="control-section">
        <span className="section-label">
          Delay: {engineDelay >= 1000 ? `${(engineDelay / 1000).toFixed(1)}s` : `${engineDelay}ms`}
        </span>
        <input
          type="range"
          className="speed-slider"
          min={100}
          max={2000}
          step={100}
          value={engineDelay}
          onChange={(e) => onSetEngineDelay(Number(e.target.value))}
        />
      </div>

      {/* Control buttons */}
      <div className="control-buttons">
        {playMode === 'manual' && (
          <button
            className="btn-engine"
            onClick={onEngineMove}
            disabled={isGameOver}
          >
            Engine Move
          </button>
        )}

        {playMode !== 'manual' && (
          <button
            className="btn-pause"
            onClick={onTogglePause}
            disabled={isGameOver}
          >
            {isPaused ? 'Resume' : 'Pause'}
          </button>
        )}

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
