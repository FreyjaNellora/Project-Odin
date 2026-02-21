// Debug console showing raw engine output with parsed info summary.

import { useRef, useEffect, useState } from 'react';
import { InfoData } from '../types/protocol';
import '../styles/DebugConsole.css';

interface DebugConsoleProps {
  lines: string[];
  latestInfo: InfoData | null;
  onSendCommand: (cmd: string) => void;
}

export default function DebugConsole({ lines, latestInfo, onSendCommand }: DebugConsoleProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [cmdInput, setCmdInput] = useState('');

  // Auto-scroll to bottom on new lines
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (cmdInput.trim()) {
      onSendCommand(cmdInput.trim());
      setCmdInput('');
    }
  };

  return (
    <div className="debug-console">
      <h3>Engine Output</h3>

      {/* Parsed info summary */}
      {latestInfo && (
        <div className="info-summary">
          {latestInfo.depth !== undefined && <span>Depth: {latestInfo.depth}</span>}
          {latestInfo.scoreCp !== undefined && <span>Score: {latestInfo.scoreCp}cp</span>}
          {latestInfo.nodes !== undefined && <span>Nodes: {latestInfo.nodes.toLocaleString()}</span>}
          {latestInfo.nps !== undefined && <span>NPS: {latestInfo.nps.toLocaleString()}</span>}
          {latestInfo.timeMs !== undefined && <span>Time: {latestInfo.timeMs}ms</span>}
          {latestInfo.phase && <span>Phase: {latestInfo.phase.toUpperCase()}</span>}
          {latestInfo.brsSurviving !== undefined && <span>Surviving: {latestInfo.brsSurviving}</span>}
          {latestInfo.mctsSims !== undefined && <span>Sims: {latestInfo.mctsSims.toLocaleString()}</span>}
          {latestInfo.pv && latestInfo.pv.length > 0 && (
            <span className="pv">PV: {latestInfo.pv.join(' ')}</span>
          )}
        </div>
      )}

      {/* Raw log */}
      <div className="debug-log" ref={scrollRef}>
        {lines.map((line, i) => (
          <div key={i} className={classifyLine(line)}>
            {line}
          </div>
        ))}
      </div>

      {/* Manual command input */}
      <form className="cmd-input" onSubmit={handleSubmit}>
        <input
          type="text"
          value={cmdInput}
          onChange={(e) => setCmdInput(e.target.value)}
          placeholder="Type engine command..."
          spellCheck={false}
        />
        <button type="submit">Send</button>
      </form>
    </div>
  );
}

/** Classify a log line for CSS styling. */
function classifyLine(line: string): string {
  if (line.startsWith('info string Error:')) return 'log-error';
  if (line.startsWith('bestmove')) return 'log-bestmove';
  if (line === 'readyok' || line === 'odinok') return 'log-routine';
  if (line.startsWith('info')) return 'log-info';
  if (line.startsWith('id ')) return 'log-routine';
  return 'log-default';
}
