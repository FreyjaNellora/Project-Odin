// Game state management for the UI.
// Maintains a local board (rendering cache), move list, and turn tracking.
// Contains ZERO game logic — the engine validates all moves.

import { useState, useCallback, useRef } from 'react';
import type { Piece, Player } from '../types/board';
import { PLAYERS } from '../types/board';
import type { EngineMessage, InfoData } from '../types/protocol';
import {
  startingPosition,
  squareName,
  parseSquare,
  fileOf,
  rankOf,
  squareFrom,
} from '../lib/board-constants';

export type SlotRole = 'human' | 'engine';
export type SlotConfig = Record<Player, SlotRole>;

export const DEFAULT_SLOT_CONFIG: SlotConfig = {
  Red: 'human',
  Blue: 'engine',
  Yellow: 'engine',
  Green: 'engine',
};

export type GameMode = 'ffa' | 'lks';

export type EvalProfileSetting = 'standard' | 'aggressive';

export type PromotionChoice = 'w' | 'r' | 'b' | 'n';

export interface PendingPromotion {
  baseMove: string;
  square: number;
  player: Player;
}

/** A move with its associated search info snapshot (captured at bestmove time). */
export interface MoveEntry {
  move: string;
  player: Player;
  info: InfoData | null;
}

export interface UseGameStateResult {
  board: (Piece | null)[];
  moveList: string[];
  moveHistory: MoveEntry[];
  currentPlayer: Player;
  scores: [number, number, number, number];
  isGameOver: boolean;
  gameWinner: Player | null;
  selectedSquare: number | null;
  lastMoveFrom: number | null;
  lastMoveTo: number | null;
  latestInfo: InfoData | null;
  error: string | null;
  slotConfig: SlotConfig;
  engineDelay: number;
  isPaused: boolean;
  gameInProgress: boolean;
  gameMode: GameMode;
  evalProfile: EvalProfileSetting;
  terrainMode: boolean;
  chess960: boolean;
  maxRounds: number;
  resolvedEvalProfile: 'standard' | 'aggressive';
  pendingPromotion: PendingPromotion | null;
  handleSquareClick: (sq: number) => void;
  requestEngineMove: () => void;
  newGame: () => void;
  handleEngineMessage: (msg: EngineMessage) => void;
  setSlotConfig: (config: SlotConfig) => void;
  setEngineDelay: (ms: number) => void;
  setGameMode: (mode: GameMode) => void;
  setEvalProfile: (profile: EvalProfileSetting) => void;
  setTerrainMode: (on: boolean) => void;
  setChess960: (on: boolean) => void;
  setMaxRounds: (n: number) => void;
  togglePause: () => void;
  resolvePromotion: (piece: PromotionChoice) => void;
  cancelPromotion: () => void;
  canUndo: boolean;
  canRedo: boolean;
  undo: () => void;
  redo: () => void;
}

export function useGameState(
  sendCommand: (cmd: string) => Promise<void>,
): UseGameStateResult {
  const [board, setBoard] = useState<(Piece | null)[]>(startingPosition);
  const [moveList, setMoveList] = useState<string[]>([]);
  const [moveHistory, setMoveHistory] = useState<MoveEntry[]>([]);
  const [currentPlayer, setCurrentPlayer] = useState<Player>('Red');
  const [scores, setScores] = useState<[number, number, number, number]>([0, 0, 0, 0]);
  const [isGameOver, setIsGameOver] = useState(false);
  const [gameWinner, setGameWinner] = useState<Player | null>(null);
  const [selectedSquare, setSelectedSquare] = useState<number | null>(null);
  const [lastMoveFrom, setLastMoveFrom] = useState<number | null>(null);
  const [lastMoveTo, setLastMoveTo] = useState<number | null>(null);
  const [latestInfo, setLatestInfo] = useState<InfoData | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Per-slot player configuration (Stage 18)
  const [slotConfig, setSlotConfigState] = useState<SlotConfig>(DEFAULT_SLOT_CONFIG);
  const [engineDelay, setEngineDelayState] = useState(500);
  const [isPaused, setIsPaused] = useState(false);

  // Promotion state: when set, the UI shows a piece selection dialog
  const [pendingPromotion, setPendingPromotion] = useState<PendingPromotion | null>(null);

  // Game settings state (Stage 8)
  const [gameMode, setGameModeState] = useState<GameMode>('ffa');
  const [evalProfile, setEvalProfileState] = useState<EvalProfileSetting>('standard');
  const [terrainMode, setTerrainModeState] = useState(false);
  const [chess960, setChess960State] = useState(false);
  const [maxRounds, setMaxRoundsState] = useState(0); // 0 = unlimited
  const gameModeRef = useRef<GameMode>('ffa');
  const evalProfileRef = useRef<EvalProfileSetting>('standard');
  const terrainModeRef = useRef(false);
  const chess960Ref = useRef(false);
  const maxRoundsRef = useRef(0);

  // Track pending move validation state
  const pendingMoveRef = useRef<string | null>(null);
  const awaitingBestmoveRef = useRef(false);
  // Ref mirror of moveList for use in async chains (avoids stale closures)
  const moveListRef = useRef<string[]>([]);
  // Auto-play: when true, engine plays continuously
  const autoPlayRef = useRef(false);
  // Ref mirror for slot config (accessed in async/timeout callbacks)
  const slotConfigRef = useRef<SlotConfig>(DEFAULT_SLOT_CONFIG);
  const engineDelayRef = useRef(500);
  // Track current player in a ref for async access
  const currentPlayerRef = useRef<Player>('Red');
  // Track eliminated players to skip them in turn advancement and remove their kings
  const eliminatedPlayersRef = useRef<Set<Player>>(new Set());
  // Next turn as reported by the engine (authoritative, beats local computation)
  const pendingNextTurnRef = useRef<Player | null>(null);
  // Latest info ref for snapshot capture at bestmove time
  const latestInfoRef = useRef<InfoData | null>(null);
  // Mirror of board state for synchronous access in async callbacks (piece notation lookup).
  const boardRef = useRef<(Piece | null)[]>(startingPosition());
  // Undo/redo stacks (Stage 18). Redo stores {move, historyEntry} pairs.
  const redoMovesRef = useRef<string[]>([]);
  const redoHistoryRef = useRef<MoveEntry[]>([]);
  // When true, the next bestmove (and its preceding nextturn/info) should be discarded.
  // Set when newGame() is called while a search is in flight — the engine's stale bestmove
  // from the old search must not be processed as a move in the new game.
  const ignoreNextBestmoveRef = useRef(false);

  // Setter that syncs state + ref (Stage 18: per-slot config)
  const setSlotConfig = useCallback((config: SlotConfig) => {
    setSlotConfigState(config);
    slotConfigRef.current = config;
    // Stop any in-flight auto-play when config changes
    autoPlayRef.current = false;
    setIsPaused(false);
  }, []);

  const setEngineDelay = useCallback((ms: number) => {
    setEngineDelayState(ms);
    engineDelayRef.current = ms;
  }, []);

  const setGameMode = useCallback((mode: GameMode) => {
    setGameModeState(mode);
    gameModeRef.current = mode;
  }, []);

  const setEvalProfile = useCallback((profile: EvalProfileSetting) => {
    setEvalProfileState(profile);
    evalProfileRef.current = profile;
  }, []);

  const setTerrainMode = useCallback((on: boolean) => {
    setTerrainModeState(on);
    terrainModeRef.current = on;
  }, []);

  const setChess960 = useCallback((on: boolean) => {
    setChess960State(on);
    chess960Ref.current = on;
  }, []);

  const setMaxRounds = useCallback((n: number) => {
    setMaxRoundsState(n);
    maxRoundsRef.current = n;
  }, []);

  // Resolved eval profile (direct — no auto mode)
  const resolvedEvalProfile: 'standard' | 'aggressive' = evalProfile;

  /** Advance to the next non-eliminated player in rotation. Returns the new player. */
  const advancePlayer = useCallback((): Player => {
    let candidate = PLAYERS[(PLAYERS.indexOf(currentPlayerRef.current) + 1) % 4];
    // Skip over any eliminated players (at most 3 skips before wrapping back)
    for (let i = 0; i < 3; i++) {
      if (!eliminatedPlayersRef.current.has(candidate)) break;
      candidate = PLAYERS[(PLAYERS.indexOf(candidate) + 1) % 4];
    }
    currentPlayerRef.current = candidate;
    setCurrentPlayer(candidate);
    return candidate;
  }, []);

  /** Check if the engine should auto-play the given player's turn. */
  const shouldEnginePlay = useCallback((player: Player): boolean => {
    return slotConfigRef.current[player] === 'engine';
  }, []);

  /** Send position + go using the ref-based move list (for auto-play chains). */
  const sendGoFromRef = useCallback(() => {
    // Guard: don't send if we're already waiting for a bestmove response.
    // Prevents duplicate searches when pause/resume overlaps with an in-flight search.
    if (awaitingBestmoveRef.current) return;

    const moves = moveListRef.current;
    const posCmd =
      moves.length > 0
        ? `position startpos moves ${moves.join(' ')}`
        : 'position startpos';

    awaitingBestmoveRef.current = true;
    sendCommand(posCmd)
      .then(() => sendCommand('go'))
      .catch(() => {
        setError('Failed to request engine move');
        awaitingBestmoveRef.current = false;
        autoPlayRef.current = false;
      });
  }, [sendCommand]);

  /** Schedule the engine to play the next turn if auto-play is active. */
  const maybeChainEngineMove = useCallback((nextPlayer: Player) => {
    if (autoPlayRef.current && shouldEnginePlay(nextPlayer)) {
      // Round limit check: 1 round ≈ 4 ply. 0 = unlimited.
      const limit = maxRoundsRef.current;
      if (limit > 0 && moveListRef.current.length >= limit * 4) {
        autoPlayRef.current = false;
        setIsPaused(true);
        return;
      }
      setTimeout(() => {
        if (autoPlayRef.current) {
          sendGoFromRef();
        }
      }, engineDelayRef.current);
    }
  }, [shouldEnginePlay, sendGoFromRef]);

  /** Apply a move to the local board display (rendering cache only). */
  const applyMoveToBoard = useCallback((moveStr: string) => {
    const parsed = parseMoveString(moveStr);
    if (!parsed) return;

    const from = parseSquare(parsed.from);
    const to = parseSquare(parsed.to);
    if (from === -1 || to === -1) return;

    setBoard((prev) => {
      const next = [...prev];
      const piece = next[from];
      if (!piece) return next;

      // Remove piece from source
      next[from] = null;

      // Handle promotion
      if (parsed.promo) {
        const promotionType = charToPieceType(parsed.promo);
        if (promotionType) {
          next[to] = { pieceType: promotionType, owner: piece.owner };
        } else {
          next[to] = piece;
        }
      } else {
        next[to] = piece;
      }

      // Display-side castling: king moved 2+ squares along its back rank
      if (piece.pieceType === 'King') {
        const isVertical = piece.owner === 'Red' || piece.owner === 'Yellow';
        const moveDist = isVertical
          ? fileOf(to) - fileOf(from)
          : rankOf(to) - rankOf(from);

        if (Math.abs(moveDist) >= 2) {
          if (isVertical) {
            // Red/Yellow: king moves along file, rook swaps file
            const rank = rankOf(from);
            if (moveDist > 0) {
              const rookFrom = findRookForCastle(next, piece.owner, rank, true);
              if (rookFrom !== -1) {
                next[squareFrom(fileOf(to) - 1, rank)] = next[rookFrom];
                next[rookFrom] = null;
              }
            } else {
              const rookFrom = findRookForCastle(next, piece.owner, rank, false);
              if (rookFrom !== -1) {
                next[squareFrom(fileOf(to) + 1, rank)] = next[rookFrom];
                next[rookFrom] = null;
              }
            }
          } else {
            // Blue/Green: king moves along rank, rook swaps rank
            const file = fileOf(from);
            if (moveDist > 0) {
              const rookFrom = findRookForCastle(next, piece.owner, rankOf(from), true);
              if (rookFrom !== -1) {
                next[squareFrom(file, rankOf(to) - 1)] = next[rookFrom];
                next[rookFrom] = null;
              }
            } else {
              const rookFrom = findRookForCastle(next, piece.owner, rankOf(from), false);
              if (rookFrom !== -1) {
                next[squareFrom(file, rankOf(to) + 1)] = next[rookFrom];
                next[rookFrom] = null;
              }
            }
          }
        }
      }

      // Display-side en passant: pawn moved diagonally to empty square.
      // Diagonal means BOTH file and rank changed (handles all 4 orientations).
      const isDiagonal = fileOf(from) !== fileOf(to) && rankOf(from) !== rankOf(to);
      if (piece.pieceType === 'Pawn' && isDiagonal && prev[to] === null) {
        // In 4-player chess the captured pawn can be on either adjacent square:
        // (toFile, fromRank) — vertical-moving pawn (Red/Yellow captured by Blue/Green)
        // (fromFile, toRank) — lateral-moving pawn (Blue/Green captured by Red/Yellow)
        const cand1 = squareFrom(fileOf(to), rankOf(from));
        const cand2 = squareFrom(fileOf(from), rankOf(to));
        if (prev[cand1]?.pieceType === 'Pawn' && prev[cand1]?.owner !== piece.owner) {
          next[cand1] = null;
        } else if (prev[cand2]?.pieceType === 'Pawn' && prev[cand2]?.owner !== piece.owner) {
          next[cand2] = null;
        }
      }

      boardRef.current = next;
      return next;
    });

    setLastMoveFrom(from);
    setLastMoveTo(to);
  }, []);

  /** Handle a square click for move input. */
  const handleSquareClick = useCallback(
    (sq: number) => {
      if (isGameOver) return;
      setError(null);

      // If current player is engine-controlled, clicking pauses auto-play
      if (slotConfig[currentPlayer] === 'engine') {
        autoPlayRef.current = false;
        setIsPaused(true);
        return;
      }

      // User interaction stops auto-play chain
      autoPlayRef.current = false;

      if (selectedSquare === null) {
        setSelectedSquare(sq);
      } else if (selectedSquare === sq) {
        setSelectedSquare(null);
      } else {
        // Second click: attempt move
        const moveStr = squareName(selectedSquare) + squareName(sq);

        // Check if this looks like a pawn promotion (display heuristic).
        // Promotion ranks match engine: Red=8, Yellow=5, Blue=file 8, Green=file 5.
        const piece = board[selectedSquare];
        if (piece?.pieceType === 'Pawn') {
          const toRank = rankOf(sq);
          const toFile = fileOf(sq);
          const isPromoRank =
            (piece.owner === 'Red' && toRank === 8) ||
            (piece.owner === 'Yellow' && toRank === 5) ||
            (piece.owner === 'Blue' && toFile === 8) ||
            (piece.owner === 'Green' && toFile === 5);
          if (isPromoRank) {
            // Show promotion piece selection dialog instead of sending immediately
            setPendingPromotion({ baseMove: moveStr, square: sq, player: piece.owner });
            setSelectedSquare(null);
            return;
          }
        }

        // Non-promotion move: send to engine for validation.
        // Follow with `isready` — if no error arrives before `readyok`,
        // the move was accepted.
        pendingMoveRef.current = moveStr;
        const allMoves = [...moveListRef.current, moveStr];
        const posCmd = `position startpos moves ${allMoves.join(' ')}`;

        sendCommand(posCmd)
          .then(() => sendCommand('isready'))
          .catch(() => {
            setError('Failed to send command to engine');
            pendingMoveRef.current = null;
          });

        setSelectedSquare(null);
      }
    },
    [selectedSquare, board, isGameOver, sendCommand, slotConfig, currentPlayer],
  );

  /** Resolve a pending promotion by appending the chosen piece suffix and sending. */
  const resolvePromotion = useCallback(
    (piece: PromotionChoice) => {
      if (!pendingPromotion) return;
      const finalMove = pendingPromotion.baseMove + piece;
      setPendingPromotion(null);

      pendingMoveRef.current = finalMove;
      const allMoves = [...moveListRef.current, finalMove];
      const posCmd = `position startpos moves ${allMoves.join(' ')}`;

      sendCommand(posCmd)
        .then(() => sendCommand('isready'))
        .catch(() => {
          setError('Failed to send command to engine');
          pendingMoveRef.current = null;
        });
    },
    [pendingPromotion, sendCommand],
  );

  /** Cancel the pending promotion (user changed their mind). */
  const cancelPromotion = useCallback(() => {
    setPendingPromotion(null);
  }, []);

  /** Request the engine to play a move for the current player. */
  const requestEngineMove = useCallback(() => {
    if (isGameOver) return;
    setError(null);
    setIsPaused(false);
    autoPlayRef.current = true;
    sendGoFromRef();
  }, [isGameOver, sendGoFromRef]);

  /** Toggle pause/resume for auto-play. */
  const togglePause = useCallback(() => {
    setIsPaused((prev) => {
      if (prev) {
        // Resuming: restart auto-play if mode requires it
        const player = currentPlayerRef.current;
        if (shouldEnginePlay(player)) {
          autoPlayRef.current = true;
          // If a search is already in flight, just set autoPlayRef and let the
          // bestmove handler chain the next move naturally via maybeChainEngineMove.
          if (!awaitingBestmoveRef.current) {
            setTimeout(() => {
              if (autoPlayRef.current) {
                sendGoFromRef();
              }
            }, engineDelayRef.current);
          }
        }
        return false;
      } else {
        // Pausing: stop auto-play
        autoPlayRef.current = false;
        return true;
      }
    });
  }, [shouldEnginePlay, sendGoFromRef]);

  /** Start a new game. Reads game settings from state and sends commands in strict order. */
  const newGame = useCallback(
    () => {
      // If a search is in flight, send `stop` so the engine finishes quickly.
      // Mark the upcoming stale bestmove (and its nextturn/info) for discard.
      if (awaitingBestmoveRef.current) {
        ignoreNextBestmoveRef.current = true;
        sendCommand('stop').catch(() => {});
      }

      const freshBoard = startingPosition();
      setBoard(freshBoard);
      boardRef.current = freshBoard;
      setMoveList([]);
      setMoveHistory([]);
      moveListRef.current = [];
      setCurrentPlayer('Red');
      currentPlayerRef.current = 'Red';
      setScores([0, 0, 0, 0]);
      setIsGameOver(false);
      setGameWinner(null);
      setSelectedSquare(null);
      setLastMoveFrom(null);
      setLastMoveTo(null);
      setLatestInfo(null);
      setError(null);
      pendingMoveRef.current = null;
      awaitingBestmoveRef.current = false;
      autoPlayRef.current = false;
      setIsPaused(false);
      eliminatedPlayersRef.current = new Set();
      pendingNextTurnRef.current = null;
      latestInfoRef.current = null;
      redoMovesRef.current = [];
      redoHistoryRef.current = [];
      setPendingPromotion(null);

      // Send settings in strict order: gamemode -> evalprofile -> terrain -> chess960 -> position -> isready
      const mode = gameModeRef.current;
      const profile = evalProfileRef.current;
      const terrain = terrainModeRef.current;
      const c960 = chess960Ref.current;

      sendCommand(`setoption name GameMode value ${mode}`)
        .then(() => sendCommand(`setoption name EvalProfile value ${profile}`))
        .then(() => sendCommand(`setoption name Terrain value ${terrain ? 'true' : 'false'}`))
        .then(() => sendCommand(`setoption name Chess960 value ${c960 ? 'true' : 'false'}`))
        .then(() => sendCommand('position startpos'))
        .then(() => sendCommand('isready'))
        .then(() => {
          // After new game is ready, auto-start if mode requires engine to play Red
          if (shouldEnginePlay('Red')) {
            autoPlayRef.current = true;
            setTimeout(() => {
              if (autoPlayRef.current) {
                sendGoFromRef();
              }
            }, engineDelayRef.current);
          }
        })
        .catch(() => setError('Failed to start new game'));
    },
    [sendCommand, shouldEnginePlay, sendGoFromRef],
  );

  /** Process engine messages for game state updates. */
  const handleEngineMessage = useCallback(
    (msg: EngineMessage) => {
      switch (msg.type) {
        case 'error': {
          setError(msg.message);
          if (pendingMoveRef.current) {
            pendingMoveRef.current = null;
            // Re-send the valid position to restore engine state
            const moves = moveListRef.current;
            const posCmd =
              moves.length > 0
                ? `position startpos moves ${moves.join(' ')}`
                : 'position startpos';
            sendCommand(posCmd).catch(() => {});
          }
          break;
        }
        case 'info': {
          // Discard stale info from a stopped search (new game in progress).
          if (ignoreNextBestmoveRef.current) break;
          setLatestInfo(msg.data);
          latestInfoRef.current = msg.data;
          // Use FFA game scores for the scoreboard (capture points, checkmate bonuses).
          // Only update when ffaScores is explicitly present — MCTS info lines omit
          // s1-s4 tokens and their float v1-v4 values (win probabilities) would
          // parseInt to 0, wiping the scoreboard.
          if (msg.data.ffaScores) {
            setScores(msg.data.ffaScores);
          }
          break;
        }
        case 'readyok': {
          // If we have a pending user move, `readyok` confirms the position
          // was accepted (no error arrived before it).
          if (pendingMoveRef.current) {
            const move = pendingMoveRef.current;
            pendingMoveRef.current = null;
            const newMoves = [...moveListRef.current, move];
            moveListRef.current = newMoves;
            setMoveList(newMoves);
            // New move branches — clear redo stack
            redoMovesRef.current = [];
            redoHistoryRef.current = [];
            // Snapshot player + board NOW, before applyMoveToBoard and advancePlayer
            // update the refs. React 18 batching defers the updater, so reading refs
            // inside the updater would see the post-update values (wrong player, wrong board).
            const movingPlayer = currentPlayerRef.current;
            const movingBoard = boardRef.current.slice();
            setMoveHistory((prev) => [
              ...prev,
              { move: formatMoveForDisplay(move, movingBoard), player: movingPlayer, info: null },
            ]);
            applyMoveToBoard(move);
            const nextPlayer = advancePlayer();

            // In semi-auto mode, after user's move, auto-play opponent turns
            if (shouldEnginePlay(nextPlayer)) {
              autoPlayRef.current = true;
              maybeChainEngineMove(nextPlayer);
            }
          }
          break;
        }
        case 'eliminated': {
          // Mark player as eliminated so future turn advancement skips them.
          eliminatedPlayersRef.current.add(msg.player);
          // Remove their king from the display board (engine called remove_king internally).
          setBoard((prev) => {
            const next = [...prev];
            for (let i = 0; i < next.length; i++) {
              if (next[i]?.owner === msg.player && next[i]?.pieceType === 'King') {
                next[i] = null;
              }
            }
            boardRef.current = next;
            return next;
          });
          break;
        }
        case 'nextturn': {
          // Discard stale nextturn from a stopped search (new game in progress).
          if (ignoreNextBestmoveRef.current) break;
          // Store the engine's authoritative next-turn so bestmove can sync to it.
          pendingNextTurnRef.current = msg.player;
          break;
        }
        case 'gameover': {
          setIsGameOver(true);
          setGameWinner(msg.winner);
          autoPlayRef.current = false;
          break;
        }
        case 'bestmove': {
          // Discard stale bestmove from a stopped search (new game in progress).
          // Don't touch awaitingBestmoveRef — it may have been set by a new sendGoFromRef.
          if (ignoreNextBestmoveRef.current) {
            ignoreNextBestmoveRef.current = false;
            break;
          }
          if (awaitingBestmoveRef.current) {
            awaitingBestmoveRef.current = false;
            const engineMove = msg.move;
            const newMoves = [...moveListRef.current, engineMove];
            moveListRef.current = newMoves;
            setMoveList(newMoves);
            // New move branches — clear redo stack
            redoMovesRef.current = [];
            redoHistoryRef.current = [];
            // Capture the latest info snapshot with this move.
            // Also snapshot player + board NOW, before applyMoveToBoard and the
            // nextPlayer assignment update the refs. React 18 batching defers the
            // updater, so reading refs inside it would see the post-update values.
            const infoSnapshot = latestInfoRef.current
              ? { ...latestInfoRef.current }
              : null;
            const movingPlayer = currentPlayerRef.current;
            const movingBoard = boardRef.current.slice();

            setMoveHistory((prev) => [
              ...prev,
              { move: formatMoveForDisplay(engineMove, movingBoard), player: movingPlayer, info: infoSnapshot },
            ]);
            applyMoveToBoard(engineMove);

            // Use the engine's authoritative next-turn if available; otherwise compute locally.
            let nextPlayer: Player;
            if (pendingNextTurnRef.current !== null) {
              nextPlayer = pendingNextTurnRef.current;
              currentPlayerRef.current = nextPlayer;
              setCurrentPlayer(nextPlayer);
              pendingNextTurnRef.current = null;
            } else {
              nextPlayer = advancePlayer();
            }

            // Chain next engine move if auto-play is active
            maybeChainEngineMove(nextPlayer);
          }
          break;
        }
        default:
          break;
      }
    },
    [sendCommand, applyMoveToBoard, advancePlayer, sendGoFromRef, shouldEnginePlay, maybeChainEngineMove],
  );

  /** Undo the last move. Rebuilds board by replaying remaining moves. */
  const undo = useCallback(() => {
    if (moveListRef.current.length === 0) return;
    if (awaitingBestmoveRef.current) return; // Don't undo during search

    // Stop auto-play
    autoPlayRef.current = false;
    setIsPaused(false);

    // Pop last move + history entry onto redo stacks
    const moves = [...moveListRef.current];
    const poppedMove = moves.pop()!;
    redoMovesRef.current = [...redoMovesRef.current, poppedMove];

    // Pop last history entry
    setMoveHistory((prev) => {
      const popped = prev[prev.length - 1];
      if (popped) {
        redoHistoryRef.current = [...redoHistoryRef.current, popped];
      }
      return prev.slice(0, -1);
    });

    // Update move list
    moveListRef.current = moves;
    setMoveList(moves);

    // Rebuild board from scratch
    const freshBoard = startingPosition();
    let builtBoard = freshBoard;
    for (const m of moves) {
      builtBoard = replayMoveOnBoard(builtBoard, m);
    }
    setBoard(builtBoard);
    boardRef.current = builtBoard;

    // Determine current player: the player who made the undone move
    // (it's now their turn again). Use moveHistory to find who moved.
    // Since we set moveHistory via setState (async), use the ref-based move count.
    // With N moves remaining, player index = N % 4 (R=0, B=1, Y=2, G=3).
    const playerIdx = moves.length % 4;
    const nextPlayer = PLAYERS[playerIdx];
    currentPlayerRef.current = nextPlayer;
    setCurrentPlayer(nextPlayer);

    // Update last move highlight
    if (moves.length > 0) {
      const lastMove = moves[moves.length - 1];
      const parsed = parseMoveString(lastMove);
      if (parsed) {
        setLastMoveFrom(parseSquare(parsed.from));
        setLastMoveTo(parseSquare(parsed.to));
      }
    } else {
      setLastMoveFrom(null);
      setLastMoveTo(null);
    }

    // Clear game-over state (undoing from a finished game)
    setIsGameOver(false);
    setGameWinner(null);
    setSelectedSquare(null);
    setError(null);

    // Sync engine position
    const posCmd = moves.length > 0
      ? `position startpos moves ${moves.join(' ')}`
      : 'position startpos';
    sendCommand(posCmd)
      .then(() => sendCommand('isready'))
      .catch(() => setError('Failed to sync engine after undo'));
  }, [sendCommand]);

  /** Redo a previously undone move. */
  const redo = useCallback(() => {
    if (redoMovesRef.current.length === 0) return;
    if (awaitingBestmoveRef.current) return;

    autoPlayRef.current = false;
    setIsPaused(false);

    // Pop from redo stacks
    const redoMoves = [...redoMovesRef.current];
    const redoHistory = [...redoHistoryRef.current];
    const move = redoMoves.pop()!;
    const histEntry = redoHistory.pop();
    redoMovesRef.current = redoMoves;
    redoHistoryRef.current = redoHistory;

    // Push to move list
    const newMoves = [...moveListRef.current, move];
    moveListRef.current = newMoves;
    setMoveList(newMoves);

    if (histEntry) {
      setMoveHistory((prev) => [...prev, histEntry]);
    }

    // Apply to board
    applyMoveToBoard(move);

    // Advance player
    const playerIdx = newMoves.length % 4;
    const nextPlayer = PLAYERS[playerIdx];
    currentPlayerRef.current = nextPlayer;
    setCurrentPlayer(nextPlayer);

    // Sync engine position
    const posCmd = `position startpos moves ${newMoves.join(' ')}`;
    sendCommand(posCmd)
      .then(() => sendCommand('isready'))
      .catch(() => setError('Failed to sync engine after redo'));
  }, [sendCommand, applyMoveToBoard]);

  const canUndo = moveList.length > 0 && !awaitingBestmoveRef.current;
  const canRedo = redoMovesRef.current.length > 0 && !awaitingBestmoveRef.current;

  return {
    board,
    moveList,
    moveHistory,
    currentPlayer,
    scores,
    isGameOver,
    gameWinner,
    selectedSquare,
    lastMoveFrom,
    lastMoveTo,
    latestInfo,
    error,
    slotConfig,
    engineDelay,
    isPaused,
    gameInProgress: moveList.length > 0,
    gameMode,
    evalProfile,
    terrainMode,
    chess960,
    maxRounds,
    resolvedEvalProfile,
    pendingPromotion,
    handleSquareClick,
    requestEngineMove,
    newGame,
    handleEngineMessage,
    setSlotConfig,
    setEngineDelay,
    setGameMode,
    setEvalProfile,
    setTerrainMode,
    setChess960,
    setMaxRounds,
    togglePause,
    resolvePromotion,
    cancelPromotion,
    canUndo,
    canRedo,
    undo,
    redo,
  };
}

// --- Helpers ---

/** Parse a move string like "e2e4", "a10c9", "d13d12", "d7d8q" into components.
 *  Handles multi-digit ranks (10-14) on a 14x14 board. */
function parseMoveString(moveStr: string): { from: string; to: string; promo?: string } | null {
  const match = moveStr.match(/^([a-n]\d{1,2})([a-n]\d{1,2})([qrbnw])?$/);
  if (!match) return null;
  return { from: match[1], to: match[2], promo: match[3] };
}

/** Return the standard piece-letter prefix for display (empty string for pawns). */
function pieceLetterPrefix(piece: Piece | null): string {
  if (!piece) return '';
  switch (piece.pieceType) {
    case 'King':          return 'K';
    case 'Queen':
    case 'PromotedQueen': return 'Q';
    case 'Rook':          return 'R';
    case 'Bishop':        return 'B';
    case 'Knight':        return 'N';
    default:              return '';   // Pawn: no prefix (standard SAN style)
  }
}

/** Format a raw coordinate move string (e.g. "e1f3") with a piece prefix for display.
 *  Looks up the piece on the board at the from-square before the move is applied. */
function formatMoveForDisplay(moveStr: string, board: (Piece | null)[]): string {
  const parsed = parseMoveString(moveStr);
  if (!parsed) return moveStr;
  const from = parseSquare(parsed.from);
  if (from === -1) return moveStr;
  return pieceLetterPrefix(board[from]) + moveStr;
}

/** Map a promotion character to a PieceType. */
function charToPieceType(c: string): Piece['pieceType'] | null {
  switch (c.toLowerCase()) {
    case 'q': return 'Queen';
    case 'r': return 'Rook';
    case 'b': return 'Bishop';
    case 'n': return 'Knight';
    case 'w': return 'PromotedQueen';
    default: return null;
  }
}

/** Find a rook for castling display (king/queenside). */
function findRookForCastle(
  board: (Piece | null)[],
  owner: Player,
  rank: number,
  kingside: boolean,
): number {
  const isVertical = owner === 'Red' || owner === 'Yellow';

  if (isVertical) {
    const startFile = kingside ? 10 : 3;
    const endFile = kingside ? 13 : 0;
    const step = kingside ? 1 : -1;
    for (let f = startFile; kingside ? f <= endFile : f >= endFile; f += step) {
      const sq = squareFrom(f, rank);
      const piece = board[sq];
      if (piece && piece.owner === owner && piece.pieceType === 'Rook') {
        return sq;
      }
    }
  } else {
    const file = owner === 'Blue' ? 0 : 13;
    const startRank = kingside ? 10 : 3;
    const endRank = kingside ? 13 : 0;
    const step = kingside ? 1 : -1;
    for (let r = startRank; kingside ? r <= endRank : r >= endRank; r += step) {
      const sq = squareFrom(file, r);
      const piece = board[sq];
      if (piece && piece.owner === owner && piece.pieceType === 'Rook') {
        return sq;
      }
    }
  }
  return -1;
}

/** Pure-function board replay: apply a single move and return the new board.
 *  Used by undo to rebuild the board from startingPosition + remaining moves.
 *  Mirrors applyMoveToBoard logic but operates on a plain array, no React state. */
function replayMoveOnBoard(board: (Piece | null)[], moveStr: string): (Piece | null)[] {
  const parsed = parseMoveString(moveStr);
  if (!parsed) return board;
  const from = parseSquare(parsed.from);
  const to = parseSquare(parsed.to);
  if (from === -1 || to === -1) return board;

  const next = [...board];
  const piece = next[from];
  if (!piece) return next;

  next[from] = null;

  if (parsed.promo) {
    const promotionType = charToPieceType(parsed.promo);
    next[to] = promotionType ? { pieceType: promotionType, owner: piece.owner } : piece;
  } else {
    next[to] = piece;
  }

  // Castling
  if (piece.pieceType === 'King') {
    const isVertical = piece.owner === 'Red' || piece.owner === 'Yellow';
    const moveDist = isVertical
      ? fileOf(to) - fileOf(from)
      : rankOf(to) - rankOf(from);
    if (Math.abs(moveDist) >= 2) {
      if (isVertical) {
        const rank = rankOf(from);
        if (moveDist > 0) {
          const rookFrom = findRookForCastle(next, piece.owner, rank, true);
          if (rookFrom !== -1) { next[squareFrom(fileOf(to) - 1, rank)] = next[rookFrom]; next[rookFrom] = null; }
        } else {
          const rookFrom = findRookForCastle(next, piece.owner, rank, false);
          if (rookFrom !== -1) { next[squareFrom(fileOf(to) + 1, rank)] = next[rookFrom]; next[rookFrom] = null; }
        }
      } else {
        const file = fileOf(from);
        if (moveDist > 0) {
          const rookFrom = findRookForCastle(next, piece.owner, rankOf(from), true);
          if (rookFrom !== -1) { next[squareFrom(file, rankOf(to) - 1)] = next[rookFrom]; next[rookFrom] = null; }
        } else {
          const rookFrom = findRookForCastle(next, piece.owner, rankOf(from), false);
          if (rookFrom !== -1) { next[squareFrom(file, rankOf(to) + 1)] = next[rookFrom]; next[rookFrom] = null; }
        }
      }
    }
  }

  // En passant
  const isDiagonal = fileOf(from) !== fileOf(to) && rankOf(from) !== rankOf(to);
  if (piece.pieceType === 'Pawn' && isDiagonal && board[to] === null) {
    const cand1 = squareFrom(fileOf(to), rankOf(from));
    const cand2 = squareFrom(fileOf(from), rankOf(to));
    if (board[cand1]?.pieceType === 'Pawn' && board[cand1]?.owner !== piece.owner) {
      next[cand1] = null;
    } else if (board[cand2]?.pieceType === 'Pawn' && board[cand2]?.owner !== piece.owner) {
      next[cand2] = null;
    }
  }

  return next;
}
