// Self-play dashboard — runs batches of all-engine games and shows stats (Stage 18).

import type { Player } from '../types/board';
import { PLAYERS } from '../types/board';
import { PLAYER_COLORS } from '../lib/board-constants';
import type { UseSelfPlayResult } from '../hooks/useSelfPlay';
import '../styles/SelfPlayDashboard.css';

interface SelfPlayDashboardProps {
  selfPlay: UseSelfPlayResult;
  engineConnected: boolean;
}

export default function SelfPlayDashboard({ selfPlay, engineConnected }: SelfPlayDashboardProps) {
  const {
    isRunning,
    targetGames,
    setTargetGames,
    completedGames,
    speed,
    setSpeed,
    start,
    stop,
    reset,
    winRates,
    avgLength,
    avgDurationMs,
    gameResults,
  } = selfPlay;

  const progress = targetGames > 0 ? (completedGames / targetGames) * 100 : 0;

  return (
    <div className="self-play-dashboard">
      <div className="sp-header">Self-Play</div>

      {/* Config row */}
      <div className="sp-config">
        <div className="sp-config-item">
          <span className="sp-label">Games</span>
          <input
            type="number"
            className="sp-input"
            min={1}
            max={1000}
            value={targetGames}
            onChange={(e) => setTargetGames(Math.max(1, Number(e.target.value) || 1))}
            disabled={isRunning}
          />
        </div>

      </div>

      {/* Action buttons */}
      <div className="sp-buttons">
        {!isRunning ? (
          <button
            className="btn-sp-start"
            onClick={start}
            disabled={!engineConnected}
          >
            Start
          </button>
        ) : (
          <button className="btn-sp-stop" onClick={stop}>
            Stop
          </button>
        )}
        <button
          className="btn-sp-reset"
          onClick={reset}
          disabled={isRunning || gameResults.length === 0}
        >
          Reset
        </button>
      </div>

      {/* Progress bar */}
      {(isRunning || gameResults.length > 0) && (
        <div className="sp-progress">
          <div className="sp-progress-bar">
            <div
              className="sp-progress-fill"
              style={{ width: `${Math.min(100, progress)}%` }}
            />
          </div>
          <span className="sp-progress-text">
            {completedGames}/{targetGames}
          </span>
        </div>
      )}

      {/* Win rates */}
      {gameResults.length > 0 && (
        <div className="sp-stats">
          <div className="sp-stats-header">Win Rates</div>
          {PLAYERS.map((player: Player) => (
            <div key={player} className="sp-win-row">
              <span
                className="sp-win-player"
                style={{ color: PLAYER_COLORS[player] }}
              >
                {player}
              </span>
              <div className="sp-win-bar-bg">
                <div
                  className="sp-win-bar-fill"
                  style={{
                    width: `${winRates[player]}%`,
                    backgroundColor: PLAYER_COLORS[player],
                  }}
                />
              </div>
              <span className="sp-win-pct">{winRates[player].toFixed(1)}%</span>
            </div>
          ))}
          <div className="sp-win-row">
            <span className="sp-win-player sp-draw">Draw</span>
            <div className="sp-win-bar-bg">
              <div
                className="sp-win-bar-fill sp-draw-bar"
                style={{ width: `${winRates.draw}%` }}
              />
            </div>
            <span className="sp-win-pct">{winRates.draw.toFixed(1)}%</span>
          </div>

          {/* Averages */}
          <div className="sp-averages">
            <div className="sp-avg-item">
              <span className="sp-label">Avg Length</span>
              <span className="sp-avg-value">{avgLength} moves</span>
            </div>
            <div className="sp-avg-item">
              <span className="sp-label">Avg Duration</span>
              <span className="sp-avg-value">
                {avgDurationMs >= 1000
                  ? `${(avgDurationMs / 1000).toFixed(1)}s`
                  : `${avgDurationMs}ms`}
              </span>
            </div>
            {avgLength > 0 && (
              <div className="sp-avg-item">
                <span className="sp-label">Avg Time/Move</span>
                <span className="sp-avg-value">
                  {Math.round(avgDurationMs / avgLength)}ms
                </span>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
