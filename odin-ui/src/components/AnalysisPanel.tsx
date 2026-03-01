// Analysis panel showing parsed engine info with prominent NPS display.

import type { InfoData } from '../types/protocol';
import '../styles/AnalysisPanel.css';

interface AnalysisPanelProps {
  latestInfo: InfoData | null;
}

export default function AnalysisPanel({ latestInfo }: AnalysisPanelProps) {
  if (!latestInfo) {
    return (
      <div className="analysis-panel">
        <h3>Analysis</h3>
        <div className="analysis-empty">No search data yet</div>
      </div>
    );
  }

  return (
    <div className="analysis-panel">
      <h3>Analysis</h3>

      {/* Prominent NPS display */}
      {latestInfo.nps !== undefined && (
        <div className="nps-display">
          {latestInfo.nps.toLocaleString()} <span className="nps-label">NPS</span>
        </div>
      )}

      <div className="analysis-grid">
        {latestInfo.depth !== undefined && (
          <div className="analysis-item">
            <span className="analysis-key">Depth</span>
            <span className="analysis-value">
              {latestInfo.depth}
              {latestInfo.seldepth !== undefined && `/${latestInfo.seldepth}`}
              {latestInfo.stopReason && ` (${latestInfo.stopReason})`}
            </span>
          </div>
        )}
        {latestInfo.scoreCp !== undefined && (
          <div className="analysis-item">
            <span className="analysis-key">Score</span>
            <span className="analysis-value">{latestInfo.scoreCp}cp</span>
          </div>
        )}
        {latestInfo.nodes !== undefined && (
          <div className="analysis-item">
            <span className="analysis-key">Nodes</span>
            <span className="analysis-value">{latestInfo.nodes.toLocaleString()}</span>
          </div>
        )}
        {latestInfo.timeMs !== undefined && (
          <div className="analysis-item">
            <span className="analysis-key">Time</span>
            <span className="analysis-value">{latestInfo.timeMs}ms</span>
          </div>
        )}
      </div>

      {latestInfo.pv && latestInfo.pv.length > 0 && (
        <div className="analysis-pv">
          <span className="analysis-key">PV</span>
          <span className="pv-moves">{latestInfo.pv.join(' ')}</span>
        </div>
      )}
    </div>
  );
}
