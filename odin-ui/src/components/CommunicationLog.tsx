// Communication log panel showing raw engine protocol messages.
// Separated from DebugConsole — keeps raw log + command input.

import { useRef, useEffect, useState } from 'react';
import '../styles/CommunicationLog.css';

interface CommunicationLogProps {
  lines: string[];
  onSendCommand: (cmd: string) => void;
}

export default function CommunicationLog({ lines, onSendCommand }: CommunicationLogProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [cmdInput, setCmdInput] = useState('');
  const [collapsed, setCollapsed] = useState(false);

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
    <div className="communication-log">
      <h3
        className="collapsible-header"
        onClick={() => setCollapsed(!collapsed)}
      >
        Communication Log {collapsed ? '\u25B6' : '\u25BC'}
      </h3>

      {!collapsed && (
        <>
          <div className="comm-log-entries" ref={scrollRef}>
            {lines.map((line, i) => (
              <div key={i} className={classifyLine(line)}>
                {line}
              </div>
            ))}
          </div>

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
        </>
      )}
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
