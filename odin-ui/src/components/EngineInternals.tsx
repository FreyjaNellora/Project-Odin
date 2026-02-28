// Engine internals panel showing search-phase-specific data.
// Collapsible panel for BRS/MCTS details, per-player values, etc.

import { useState } from 'react';
import type { InfoData } from '../types/protocol';
import type { Player } from '../types/board';
import { PLAYERS } from '../types/board';
import { PLAYER_COLORS } from '../lib/board-constants';
import '../styles/EngineInternals.css';

interface EngineInternalsProps {
  latestInfo: InfoData | null;
}

export default function EngineInternals({ latestInfo }: EngineInternalsProps) {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="engine-internals">
      <h3
        className="collapsible-header"
        onClick={() => setCollapsed(!collapsed)}
      >
        Engine Internals {collapsed ? '\u25B6' : '\u25BC'}
      </h3>

      {!collapsed && (
        <div className="internals-content">
          {!latestInfo ? (
            <div className="internals-empty">No engine data</div>
          ) : (
            <>
              {/* Search phase */}
              {latestInfo.phase && (
                <div className="internals-row">
                  <span className="internals-key">Phase</span>
                  <span className="internals-value phase-badge">
                    {latestInfo.phase.toUpperCase()}
                  </span>
                </div>
              )}

              {/* BRS-specific */}
              {latestInfo.brsSurviving !== undefined && (
                <div className="internals-row">
                  <span className="internals-key">BRS Surviving</span>
                  <span className="internals-value">{latestInfo.brsSurviving} candidates</span>
                </div>
              )}

              {/* MCTS-specific */}
              {latestInfo.mctsSims !== undefined && (
                <div className="internals-row">
                  <span className="internals-key">MCTS Sims</span>
                  <span className="internals-value">{latestInfo.mctsSims.toLocaleString()}</span>
                </div>
              )}

              {/* Selective depth */}
              {latestInfo.seldepth !== undefined && (
                <div className="internals-row">
                  <span className="internals-key">Sel. Depth</span>
                  <span className="internals-value">{latestInfo.seldepth}</span>
                </div>
              )}

              {/* Per-player values */}
              {latestInfo.values && (
                <div className="internals-values">
                  <span className="internals-key">Per-Player Eval (cp)</span>
                  <div className="values-grid">
                    {PLAYERS.map((player: Player, i: number) => (
                      <div key={player} className="value-cell">
                        <span
                          className="value-player"
                          style={{ color: PLAYER_COLORS[player] }}
                        >
                          {player[0]}
                        </span>
                        <span className="value-score">
                          {latestInfo.values![i]}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
