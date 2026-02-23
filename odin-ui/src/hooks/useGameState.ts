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

export type PlayMode = 'manual' | 'semi-auto' | 'full-auto';

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
  selectedSquare: number | null;
  lastMoveFrom: number | null;
  lastMoveTo: number | null;
  latestInfo: InfoData | null;
  error: string | null;
  playMode: PlayMode;
  humanPlayer: Player | null;
  engineDelay: number;
  isPaused: boolean;
  gameInProgress: boolean;
  pendingPromotion: PendingPromotion | null;
  handleSquareClick: (sq: number) => void;
  requestEngineMove: () => void;
  newGame: (terrain: boolean) => void;
  handleEngineMessage: (msg: EngineMessage) => void;
  setPlayMode: (mode: PlayMode) => void;
  setHumanPlayer: (player: Player | null) => void;
  setEngineDelay: (ms: number) => void;
  togglePause: () => void;
  resolvePromotion: (piece: PromotionChoice) => void;
  cancelPromotion: () => void;
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
  const [selectedSquare, setSelectedSquare] = useState<number | null>(null);
  const [lastMoveFrom, setLastMoveFrom] = useState<number | null>(null);
  const [lastMoveTo, setLastMoveTo] = useState<number | null>(null);
  const [latestInfo, setLatestInfo] = useState<InfoData | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Play mode state
  const [playMode, setPlayModeState] = useState<PlayMode>('manual');
  const [humanPlayer, setHumanPlayerState] = useState<Player | null>(null);
  const [engineDelay, setEngineDelayState] = useState(500);
  const [isPaused, setIsPaused] = useState(false);

  // Promotion state: when set, the UI shows a piece selection dialog
  const [pendingPromotion, setPendingPromotion] = useState<PendingPromotion | null>(null);

  // Track pending move validation state
  const pendingMoveRef = useRef<string | null>(null);
  const awaitingBestmoveRef = useRef(false);
  // Ref mirror of moveList for use in async chains (avoids stale closures)
  const moveListRef = useRef<string[]>([]);
  // Auto-play: when true, engine plays continuously
  const autoPlayRef = useRef(false);
  // Ref mirrors for play mode settings (accessed in async/timeout callbacks)
  const playModeRef = useRef<PlayMode>('manual');
  const humanPlayerRef = useRef<Player | null>(null);
  const engineDelayRef = useRef(500);
  // Track current player in a ref for async access
  const currentPlayerRef = useRef<Player>('Red');
  // Track eliminated players to skip them in turn advancement and remove their kings
  const eliminatedPlayersRef = useRef<Set<Player>>(new Set());
  // Next turn as reported by the engine (authoritative, beats local computation)
  const pendingNextTurnRef = useRef<Player | null>(null);
  // Latest info ref for snapshot capture at bestmove time
  const latestInfoRef = useRef<InfoData | null>(null);

  // Setters that sync state + ref
  const setPlayMode = useCallback((mode: PlayMode) => {
    setPlayModeState(mode);
    playModeRef.current = mode;
    // Stop any in-flight auto-play when mode changes
    autoPlayRef.current = false;
    setIsPaused(false);
  }, []);

  const setHumanPlayer = useCallback((player: Player | null) => {
    setHumanPlayerState(player);
    humanPlayerRef.current = player;
    // Stop any in-flight engine chain so it doesn't play through the newly-selected
    // player's turn. The chain resumes naturally on the next maybeChainEngineMove call.
    autoPlayRef.current = false;
  }, []);

  const setEngineDelay = useCallback((ms: number) => {
    setEngineDelayState(ms);
    engineDelayRef.current = ms;
  }, []);

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
    if (playModeRef.current === 'full-auto') return true;
    if (
      playModeRef.current === 'semi-auto' &&
      humanPlayerRef.current !== null &&
      humanPlayerRef.current !== player
    ) return true;
    return false;
  }, []);

  /** Send position + go using the ref-based move list (for auto-play chains). */
  const sendGoFromRef = useCallback(() => {
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
        const capturedSq = squareFrom(fileOf(to), rankOf(from));
        next[capturedSq] = null;
      }

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

      // In full-auto mode, clicking stops auto-play but doesn't allow manual moves
      if (playMode === 'full-auto') {
        autoPlayRef.current = false;
        setIsPaused(true);
        return;
      }

      // In semi-auto mode, only allow clicks on the human player's turn
      if (playMode === 'semi-auto' && humanPlayer !== null && currentPlayer !== humanPlayer) {
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
    [selectedSquare, board, isGameOver, sendCommand, playMode, humanPlayer, currentPlayer],
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
          setTimeout(() => {
            if (autoPlayRef.current) {
              sendGoFromRef();
            }
          }, engineDelayRef.current);
        }
        return false;
      } else {
        // Pausing: stop auto-play
        autoPlayRef.current = false;
        return true;
      }
    });
  }, [shouldEnginePlay, sendGoFromRef]);

  /** Start a new game. */
  const newGame = useCallback(
    (terrain: boolean) => {
      setBoard(startingPosition());
      setMoveList([]);
      setMoveHistory([]);
      moveListRef.current = [];
      setCurrentPlayer('Red');
      currentPlayerRef.current = 'Red';
      setScores([0, 0, 0, 0]);
      setIsGameOver(false);
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
      setPendingPromotion(null);

      sendCommand(`setoption name Terrain value ${terrain ? 'true' : 'false'}`)
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
          setLatestInfo(msg.data);
          latestInfoRef.current = msg.data;
          if (msg.data.values) {
            setScores(msg.data.values);
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
            // User moves have no search info
            setMoveHistory((prev) => [
              ...prev,
              { move, player: currentPlayerRef.current, info: null },
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
            return next;
          });
          break;
        }
        case 'nextturn': {
          // Store the engine's authoritative next-turn so bestmove can sync to it.
          pendingNextTurnRef.current = msg.player;
          break;
        }
        case 'gameover': {
          setIsGameOver(true);
          autoPlayRef.current = false;
          break;
        }
        case 'bestmove': {
          if (awaitingBestmoveRef.current) {
            awaitingBestmoveRef.current = false;
            const engineMove = msg.move;
            const newMoves = [...moveListRef.current, engineMove];
            moveListRef.current = newMoves;
            setMoveList(newMoves);
            // Capture the latest info snapshot with this move
            const infoSnapshot = latestInfoRef.current
              ? { ...latestInfoRef.current }
              : null;
            setMoveHistory((prev) => [
              ...prev,
              { move: engineMove, player: currentPlayerRef.current, info: infoSnapshot },
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

  return {
    board,
    moveList,
    moveHistory,
    currentPlayer,
    scores,
    isGameOver,
    selectedSquare,
    lastMoveFrom,
    lastMoveTo,
    latestInfo,
    error,
    playMode,
    humanPlayer,
    engineDelay,
    isPaused,
    gameInProgress: moveList.length > 0,
    pendingPromotion,
    handleSquareClick,
    requestEngineMove,
    newGame,
    handleEngineMessage,
    setPlayMode,
    setHumanPlayer,
    setEngineDelay,
    togglePause,
    resolvePromotion,
    cancelPromotion,
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
