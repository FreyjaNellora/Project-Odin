// Game controls: turn indicator, scores, game settings, play mode, speed, new game button.

import type { Player } from '../types/board';
import { PLAYERS } from '../types/board';
import { PLAYER_COLORS } from '../lib/board-constants';
import type { PlayMode, GameMode, EvalProfileSetting } from '../hooks/useGameState';
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
  gameMode: GameMode;
  evalProfile: EvalProfileSetting;
  resolvedEvalProfile: 'standard' | 'aggressive';
  terrainMode: boolean;
  onNewGame: () => void;
  onEngineMove: () => void;
  onSetPlayMode: (mode: PlayMode) => void;
  onSetHumanPlayer: (player: Player | null) => void;
  onSetEngineDelay: (ms: number) => void;
  onSetGameMode: (mode: GameMode) => void;
  onSetEvalProfile: (profile: EvalProfileSetting) => void;
  onSetTerrainMode: (on: boolean) => void;
  onTogglePause: () => void;
}

const MODE_LABELS: Record<PlayMode, string> = {
  manual: 'Manual',
  'semi-auto': 'Semi-Auto',
  'full-auto': 'Full Auto',
};

/** Display label for the Auto eval profile option (shows resolved value). */
function autoLabel(resolved: 'standard' | 'aggressive'): string {
  const inner = resolved === 'aggressive' ? 'Aggro' : 'Std';
  return `Auto (${inner})`;
}

export default function GameControls({
  currentPlayer,
  scores,
  isGameOver,
  error,
  playMode,
  humanPlayer,
  engineDelay,
  isPaused,
  gameMode,
  evalProfile,
  resolvedEvalProfile,
  terrainMode,
  onNewGame,
  onEngineMove,
  onSetPlayMode,
  onSetHumanPlayer,
  onSetEngineDelay,
  onSetGameMode,
  onSetEvalProfile,
  onSetTerrainMode,
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

      {/* Config tags — show active resolved config */}
      <div className="config-tags">
        <span className={`config-tag config-tag-mode`}>
          {gameMode === 'ffa' ? 'FFA' : 'LKS'}
        </span>
        <span className={`config-tag config-tag-profile`}>
          {resolvedEvalProfile === 'aggressive' ? 'Aggressive' : 'Standard'}
        </span>
        <span className={`config-tag config-tag-terrain`}>
          {terrainMode ? 'Terrain' : 'Normal'}
        </span>
      </div>

      {/* Error display */}
      {error && <div className="error-display">{error}</div>}

      {/* Game over indicator */}
      {isGameOver && <div className="game-over">Game Over</div>}

      {/* Game Mode selector */}
      <div className="control-section">
        <span className="section-label">Game Mode</span>
        <div className="mode-selector">
          <button
            className={`btn-mode ${gameMode === 'ffa' ? 'active' : ''}`}
            onClick={() => onSetGameMode('ffa')}
          >
            FFA
          </button>
          <button
            className={`btn-mode ${gameMode === 'lks' ? 'active' : ''}`}
            onClick={() => onSetGameMode('lks')}
          >
            LKS
          </button>
        </div>
      </div>

      {/* Eval Profile selector */}
      <div className="control-section">
        <span className="section-label">Eval Profile</span>
        <div className="mode-selector">
          <button
            className={`btn-mode ${evalProfile === 'auto' ? 'active' : ''}`}
            onClick={() => onSetEvalProfile('auto')}
          >
            {autoLabel(resolvedEvalProfile)}
          </button>
          <button
            className={`btn-mode ${evalProfile === 'standard' ? 'active' : ''}`}
            onClick={() => onSetEvalProfile('standard')}
          >
            Standard
          </button>
          <button
            className={`btn-mode ${evalProfile === 'aggressive' ? 'active' : ''}`}
            onClick={() => onSetEvalProfile('aggressive')}
          >
            Aggressive
          </button>
        </div>
      </div>

      {/* Terrain toggle */}
      <div className="control-section">
        <span className="section-label">Terrain</span>
        <div className="mode-selector">
          <button
            className={`btn-mode ${!terrainMode ? 'active' : ''}`}
            onClick={() => onSetTerrainMode(false)}
          >
            Off
          </button>
          <button
            className={`btn-mode ${terrainMode ? 'active' : ''}`}
            onClick={() => onSetTerrainMode(true)}
          >
            On
          </button>
        </div>
      </div>

      {/* Play mode selector */}
      <div className="control-section">
        <span className="section-label">Play Mode</span>
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

        <button className="btn-new" onClick={onNewGame}>
          New Game
        </button>
      </div>
    </div>
  );
}
