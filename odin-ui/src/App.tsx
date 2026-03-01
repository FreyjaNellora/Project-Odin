// Project Odin — UI Shell
// Root component wiring board, engine, controls, and info panels.

import { useState, useEffect } from 'react';
import './App.css';
import { useEngine } from './hooks/useEngine';
import { useGameState } from './hooks/useGameState';
import { useSelfPlay } from './hooks/useSelfPlay';
import BoardDisplay from './components/BoardDisplay';
import GameControls from './components/GameControls';
import SelfPlayDashboard from './components/SelfPlayDashboard';
import AnalysisPanel from './components/AnalysisPanel';
import GameLog from './components/GameLog';
import EngineInternals from './components/EngineInternals';
import CommunicationLog from './components/CommunicationLog';
import StatusBar from './components/StatusBar';
import PromotionDialog from './components/PromotionDialog';

function App() {
  const engine = useEngine();
  const game = useGameState(engine.sendCommand);
  const selfPlay = useSelfPlay(game);
  const [showCoords, setShowCoords] = useState(true);

  // Wire engine messages to game state handler
  useEffect(() => {
    engine.onMessage(game.handleEngineMessage);
  }, [engine.onMessage, game.handleEngineMessage]);

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
            slotConfig={game.slotConfig}
            engineDelay={game.engineDelay}
            isPaused={game.isPaused}
            gameMode={game.gameMode}
            evalProfile={game.evalProfile}
            resolvedEvalProfile={game.resolvedEvalProfile}
            terrainMode={game.terrainMode}
            chess960={game.chess960}
            maxRounds={game.maxRounds}
            onNewGame={game.newGame}
            onEngineMove={game.requestEngineMove}
            onSetSlotConfig={game.setSlotConfig}
            onSetEngineDelay={game.setEngineDelay}
            onSetGameMode={game.setGameMode}
            onSetEvalProfile={game.setEvalProfile}
            onSetTerrainMode={game.setTerrainMode}
            onSetChess960={game.setChess960}
            onSetMaxRounds={game.setMaxRounds}
            onTogglePause={game.togglePause}
            canUndo={game.canUndo}
            canRedo={game.canRedo}
            onUndo={game.undo}
            onRedo={game.redo}
          />
          <SelfPlayDashboard
            selfPlay={selfPlay}
            engineConnected={engine.isConnected}
          />
          <div className="board-options">
            <label className="coord-toggle">
              <input
                type="checkbox"
                checked={showCoords}
                onChange={(e) => setShowCoords(e.target.checked)}
              />
              Coords
            </label>
          </div>
        </div>

        <div className="center-panel">
          <BoardDisplay
            board={game.board}
            selectedSquare={game.selectedSquare}
            lastMoveFrom={game.lastMoveFrom}
            lastMoveTo={game.lastMoveTo}
            showCoords={showCoords}
            onSquareClick={game.handleSquareClick}
          />
          {game.pendingPromotion && (
            <PromotionDialog
              player={game.pendingPromotion.player}
              onSelect={game.resolvePromotion}
              onCancel={game.cancelPromotion}
            />
          )}
        </div>

        <div className="right-panel">
          <AnalysisPanel latestInfo={game.latestInfo} />
          <GameLog moveHistory={game.moveHistory} />
          <EngineInternals latestInfo={game.latestInfo} />
          <CommunicationLog
            lines={engine.rawLog}
            onSendCommand={(cmd) => engine.sendCommand(cmd)}
          />
        </div>
      </div>
    </div>
  );
}

export default App;
