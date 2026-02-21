// Status bar showing engine connection state and name.

interface StatusBarProps {
  isConnected: boolean;
  engineName: string;
  onConnect: () => void;
}

export default function StatusBar({ isConnected, engineName, onConnect }: StatusBarProps) {
  return (
    <div className="status-bar">
      <span className="engine-name">
        {engineName || 'Odin Engine'}
      </span>
      <span className={`connection-status ${isConnected ? 'connected' : 'disconnected'}`}>
        {isConnected ? 'Connected' : 'Disconnected'}
      </span>
      {!isConnected && (
        <button className="connect-btn" onClick={onConnect}>
          Connect
        </button>
      )}
    </div>
  );
}
