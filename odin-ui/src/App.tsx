// Project Odin — Basic UI Shell (Stage 5)
// Root component wiring board, engine, controls, and debug console.

import { useEffect } from 'react';
import './App.css';
import { useEngine } from './hooks/useEngine';
import { useGameState } from './hooks/useGameState';
import BoardDisplay from './components/BoardDisplay';
import GameControls from './components/GameControls';
import DebugConsole from './components/DebugConsole';
import StatusBar from './components/StatusBar';

function App() {
  const engine = useEngine();
  const game = useGameState(engine.sendCommand);

  // Wire engine messages to game state handler
  useEffect(() => {
    engine.onMessage(game.handleEngineMessage);
  }, [engine.onMessage, game.handleEngineMessage]);

  // Auto-spawn engine on mount
  useEffect(() => {
    engine.spawnEngine().catch((e) => {
      console.error('Failed to spawn engine:', e);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="app">
      <StatusBar
        isConnected={engine.isConnected}
        engineName={engine.engineName}
        onConnect={() => engine.spawnEngine()}
      />

      <div className="main-layout">
        <div className="left-panel">
          <GameControls
            currentPlayer={game.currentPlayer}
            scores={game.scores}
            isGameOver={game.isGameOver}
            error={game.error}
            onNewGame={game.newGame}
            onEngineMove={game.requestEngineMove}
          />
        </div>

        <div className="center-panel">
          <BoardDisplay
            board={game.board}
            selectedSquare={game.selectedSquare}
            lastMoveFrom={game.lastMoveFrom}
            lastMoveTo={game.lastMoveTo}
            onSquareClick={game.handleSquareClick}
          />
        </div>

        <div className="right-panel">
          <DebugConsole
            lines={engine.rawLog}
            latestInfo={game.latestInfo}
            onSendCommand={(cmd) => engine.sendCommand(cmd)}
          />
        </div>
      </div>
    </div>
  );
}

export default App;
