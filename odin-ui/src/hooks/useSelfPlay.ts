// Self-play orchestration hook (Stage 18).
// Runs a series of all-engine games and collects statistics.

import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import type { Player } from '../types/board';
import type { SlotConfig, UseGameStateResult } from './useGameState';

export interface GameResult {
  winner: Player | null;  // null = draw
  moveCount: number;
  durationMs: number;
}

export interface UseSelfPlayResult {
  isRunning: boolean;
  targetGames: number;
  setTargetGames: (n: number) => void;
  completedGames: number;
  gameResults: GameResult[];
  start: () => void;
  stop: () => void;
  reset: () => void;
  winRates: Record<Player | 'draw', number>;
  avgLength: number;
  avgDurationMs: number;
}

export function useSelfPlay(game: UseGameStateResult): UseSelfPlayResult {
  const [isRunning, setIsRunning] = useState(false);
  const [targetGames, setTargetGamesState] = useState(10);
  const [completedGames, setCompletedGames] = useState(0);
  const [gameResults, setGameResults] = useState<GameResult[]>([]);
  // Refs for async/effect access
  const isRunningRef = useRef(false);
  const targetGamesRef = useRef(10);
  const completedGamesRef = useRef(0);
  const gameStartRef = useRef(0);

  // Save user's original config to restore on stop
  const savedConfigRef = useRef<SlotConfig | null>(null);
  const savedDelayRef = useRef(500);

  const setTargetGames = useCallback((n: number) => {
    setTargetGamesState(n);
    targetGamesRef.current = n;
  }, []);

  // When a game ends during self-play, record result and start next
  useEffect(() => {
    if (!game.isGameOver || !isRunningRef.current) return;

    const result: GameResult = {
      winner: game.gameWinner,
      moveCount: game.moveList.length,
      durationMs: Date.now() - gameStartRef.current,
    };

    setGameResults((prev) => [...prev, result]);
    const newCompleted = completedGamesRef.current + 1;
    completedGamesRef.current = newCompleted;
    setCompletedGames(newCompleted);

    if (newCompleted < targetGamesRef.current) {
      // Start next game after brief delay
      setTimeout(() => {
        if (isRunningRef.current) {
          gameStartRef.current = Date.now();
          game.newGame();
        }
      }, 300);
    } else {
      // All games completed — stop
      setIsRunning(false);
      isRunningRef.current = false;
      // Restore user config
      if (savedConfigRef.current) {
        game.setSlotConfig(savedConfigRef.current);
      }
      game.setEngineDelay(savedDelayRef.current);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [game.isGameOver]);

  const start = useCallback(() => {
    // Save current user config
    savedConfigRef.current = { ...game.slotConfig };
    savedDelayRef.current = game.engineDelay;

    // Force all-engine + set speed delay
    game.setSlotConfig({ Red: 'engine', Blue: 'engine', Yellow: 'engine', Green: 'engine' });
    game.setEngineDelay(0);

    setIsRunning(true);
    isRunningRef.current = true;
    completedGamesRef.current = 0;
    setCompletedGames(0);
    setGameResults([]);

    gameStartRef.current = Date.now();
    // Use setTimeout to let React flush the slot config change before newGame reads it
    setTimeout(() => {
      game.newGame();
    }, 50);
  }, [game]);

  const stop = useCallback(() => {
    setIsRunning(false);
    isRunningRef.current = false;

    // Restore original config
    if (savedConfigRef.current) {
      game.setSlotConfig(savedConfigRef.current);
      savedConfigRef.current = null;
    }
    game.setEngineDelay(savedDelayRef.current);
  }, [game]);

  const reset = useCallback(() => {
    if (isRunningRef.current) {
      stop();
    }
    setCompletedGames(0);
    completedGamesRef.current = 0;
    setGameResults([]);
  }, [stop]);

  // Computed stats
  const winRates = useMemo(() => {
    const zero = { Red: 0, Blue: 0, Yellow: 0, Green: 0, draw: 0 } as Record<Player | 'draw', number>;
    if (gameResults.length === 0) return zero;

    const counts = { Red: 0, Blue: 0, Yellow: 0, Green: 0, draw: 0 };
    for (const r of gameResults) {
      if (r.winner === null) counts.draw++;
      else counts[r.winner]++;
    }
    const total = gameResults.length;
    return {
      Red: (counts.Red / total) * 100,
      Blue: (counts.Blue / total) * 100,
      Yellow: (counts.Yellow / total) * 100,
      Green: (counts.Green / total) * 100,
      draw: (counts.draw / total) * 100,
    };
  }, [gameResults]);

  const avgLength = useMemo(
    () =>
      gameResults.length > 0
        ? Math.round(gameResults.reduce((sum, r) => sum + r.moveCount, 0) / gameResults.length)
        : 0,
    [gameResults],
  );

  const avgDurationMs = useMemo(
    () =>
      gameResults.length > 0
        ? Math.round(gameResults.reduce((sum, r) => sum + r.durationMs, 0) / gameResults.length)
        : 0,
    [gameResults],
  );

  return {
    isRunning,
    targetGames,
    setTargetGames,
    completedGames,
    gameResults,
    start,
    stop,
    reset,
    winRates,
    avgLength,
    avgDurationMs,
  };
}
