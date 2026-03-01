// Game controls: turn indicator, scores, game settings, slot config, speed, new game button.

import type { Player } from '../types/board';
import { PLAYERS } from '../types/board';
import { PLAYER_COLORS } from '../lib/board-constants';
import type { SlotConfig, SlotRole, GameMode, EvalProfileSetting } from '../hooks/useGameState';
import { DEFAULT_SLOT_CONFIG } from '../hooks/useGameState';
import '../styles/GameControls.css';

interface GameControlsProps {
  currentPlayer: Player;
  scores: [number, number, number, number];
  isGameOver: boolean;
  error: string | null;
  slotConfig: SlotConfig;
  engineDelay: number;
  isPaused: boolean;
  gameMode: GameMode;
  evalProfile: EvalProfileSetting;
  resolvedEvalProfile: 'standard' | 'aggressive';
  terrainMode: boolean;
  chess960: boolean;
  maxRounds: number;
  onNewGame: () => void;
  onEngineMove: () => void;
  onSetSlotConfig: (config: SlotConfig) => void;
  onSetEngineDelay: (ms: number) => void;
  onSetGameMode: (mode: GameMode) => void;
  onSetEvalProfile: (profile: EvalProfileSetting) => void;
  onSetTerrainMode: (on: boolean) => void;
  onSetChess960: (on: boolean) => void;
  onSetMaxRounds: (n: number) => void;
  onTogglePause: () => void;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
}


export default function GameControls({
  currentPlayer,
  scores,
  isGameOver,
  error,
  slotConfig,
  engineDelay,
  isPaused,
  gameMode,
  evalProfile,
  resolvedEvalProfile,
  terrainMode,
  chess960,
  maxRounds,
  onNewGame,
  onEngineMove,
  onSetSlotConfig,
  onSetEngineDelay,
  onSetGameMode,
  onSetEvalProfile,
  onSetTerrainMode,
  onSetChess960,
  onSetMaxRounds,
  onTogglePause,
  canUndo,
  canRedo,
  onUndo,
  onRedo,
}: GameControlsProps) {
  const hasEngineSlot = PLAYERS.some((p) => slotConfig[p] === 'engine');
  const allEngine = PLAYERS.every((p) => slotConfig[p] === 'engine');
  const allHuman = PLAYERS.every((p) => slotConfig[p] === 'human');

  const toggleSlot = (player: Player) => {
    const newRole: SlotRole = slotConfig[player] === 'human' ? 'engine' : 'human';
    onSetSlotConfig({ ...slotConfig, [player]: newRole });
  };
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

      {/* FFA Game Scores (capture points, checkmate bonuses) */}
      <div className="scores">
        <div className="scores-header">Score</div>
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
        {chess960 && (
          <span className="config-tag config-tag-960">960</span>
        )}
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

      {/* Terrain & Chess960 toggles (compact row) */}
      <div className="toggle-row">
        <div className="toggle-item">
          <span className="section-label">Terrain</span>
          <button
            className={`btn-toggle ${terrainMode ? 'active' : ''}`}
            onClick={() => onSetTerrainMode(!terrainMode)}
          >
            {terrainMode ? 'On' : 'Off'}
          </button>
        </div>
        <div className="toggle-item">
          <span className="section-label">Chess960</span>
          <button
            className={`btn-toggle ${chess960 ? 'active' : ''}`}
            onClick={() => onSetChess960(!chess960)}
          >
            {chess960 ? 'On' : 'Off'}
          </button>
        </div>
      </div>

      {/* Max Rounds (0 = unlimited, auto-stop for diagnostic use) */}
      <div className="control-section">
        <span className="section-label">
          Max Rounds: {maxRounds === 0 ? '∞' : maxRounds}
        </span>
        <input
          type="range"
          className="speed-slider"
          min={0}
          max={50}
          step={1}
          value={maxRounds}
          onChange={(e) => onSetMaxRounds(Number(e.target.value))}
        />
      </div>

      {/* Per-slot player configuration (Stage 18) */}
      <div className="control-section">
        <span className="section-label">Players</span>
        <div className="slot-config">
          {PLAYERS.map((player) => (
            <div key={player} className="slot-row">
              <span
                className="slot-player"
                style={{ color: PLAYER_COLORS[player] }}
              >
                {player}
              </span>
              <div className="slot-toggle">
                <button
                  className={`btn-slot ${slotConfig[player] === 'human' ? 'active' : ''}`}
                  onClick={() => toggleSlot(player)}
                >
                  {slotConfig[player] === 'human' ? 'Human' : 'Engine'}
                </button>
              </div>
            </div>
          ))}
        </div>
        {/* Quick presets */}
        <div className="slot-presets">
          <button
            className={`btn-preset ${slotConfig.Red === 'human' && slotConfig.Blue === 'engine' && slotConfig.Yellow === 'engine' && slotConfig.Green === 'engine' ? 'active' : ''}`}
            onClick={() => onSetSlotConfig(DEFAULT_SLOT_CONFIG)}
          >
            Play as Red
          </button>
          <button
            className={`btn-preset ${allEngine ? 'active' : ''}`}
            onClick={() => onSetSlotConfig({ Red: 'engine', Blue: 'engine', Yellow: 'engine', Green: 'engine' })}
          >
            Watch
          </button>
          <button
            className={`btn-preset ${allHuman ? 'active' : ''}`}
            onClick={() => onSetSlotConfig({ Red: 'human', Blue: 'human', Yellow: 'human', Green: 'human' })}
          >
            Hot Seat
          </button>
        </div>
      </div>

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
        {slotConfig[currentPlayer] === 'human' && (
          <button
            className="btn-engine"
            onClick={onEngineMove}
            disabled={isGameOver}
          >
            Engine Move
          </button>
        )}

        {hasEngineSlot && (
          <button
            className="btn-pause"
            onClick={onTogglePause}
            disabled={isGameOver}
          >
            {isPaused ? 'Resume' : 'Pause'}
          </button>
        )}

        {/* Undo / Redo (Stage 18) */}
        <div className="undo-redo-row">
          <button
            className="btn-undo"
            onClick={onUndo}
            disabled={!canUndo}
          >
            Undo
          </button>
          <button
            className="btn-redo"
            onClick={onRedo}
            disabled={!canRedo}
          >
            Redo
          </button>
        </div>

        <button className="btn-new" onClick={onNewGame}>
          New Game
        </button>
      </div>
    </div>
  );
}
