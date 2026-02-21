// Game state management for the UI.
// Maintains a local board (rendering cache), move list, and turn tracking.
// Contains ZERO game logic — the engine validates all moves.

import { useState, useCallback, useRef, useEffect } from 'react';
import { Piece, Player, PLAYERS } from '../types/board';
import { EngineMessage, InfoData } from '../types/protocol';
import {
  startingPosition,
  squareName,
  parseSquare,
  fileOf,
  rankOf,
  squareFrom,
} from '../lib/board-constants';

export interface UseGameStateResult {
  board: (Piece | null)[];
  moveList: string[];
  currentPlayer: Player;
  scores: [number, number, number, number];
  isGameOver: boolean;
  selectedSquare: number | null;
  lastMoveFrom: number | null;
  lastMoveTo: number | null;
  latestInfo: InfoData | null;
  error: string | null;
  handleSquareClick: (sq: number) => void;
  requestEngineMove: () => void;
  newGame: (terrain: boolean) => void;
  handleEngineMessage: (msg: EngineMessage) => void;
}

export function useGameState(
  sendCommand: (cmd: string) => Promise<void>,
): UseGameStateResult {
  const [board, setBoard] = useState<(Piece | null)[]>(startingPosition);
  const [moveList, setMoveList] = useState<string[]>([]);
  const [currentPlayer, setCurrentPlayer] = useState<Player>('Red');
  const [scores, setScores] = useState<[number, number, number, number]>([0, 0, 0, 0]);
  const [isGameOver, setIsGameOver] = useState(false);
  const [selectedSquare, setSelectedSquare] = useState<number | null>(null);
  const [lastMoveFrom, setLastMoveFrom] = useState<number | null>(null);
  const [lastMoveTo, setLastMoveTo] = useState<number | null>(null);
  const [latestInfo, setLatestInfo] = useState<InfoData | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Track pending move validation state
  const pendingMoveRef = useRef<string | null>(null);
  const awaitingBestmoveRef = useRef(false);

  /** Advance to the next player in rotation. */
  const advancePlayer = useCallback(() => {
    setCurrentPlayer((prev) => {
      const idx = PLAYERS.indexOf(prev);
      return PLAYERS[(idx + 1) % 4];
    });
  }, []);

  /** Apply a move to the local board display (rendering cache only). */
  const applyMoveToBoard = useCallback((moveStr: string) => {
    const fromName = moveStr.slice(0, 2);
    // Handle moves like "d2d4" (4 chars) or "d7d8q" (5 chars, promotion)
    const toName = moveStr.slice(2, 4);
    const promotionChar = moveStr.length > 4 ? moveStr[4] : null;

    const from = parseSquare(fromName);
    const to = parseSquare(toName);
    if (from === -1 || to === -1) return;

    setBoard((prev) => {
      const next = [...prev];
      const piece = next[from];
      if (!piece) return next;

      // Remove piece from source
      next[from] = null;

      // Handle promotion
      if (promotionChar) {
        const promotionType = charToPieceType(promotionChar);
        if (promotionType) {
          next[to] = { pieceType: promotionType, owner: piece.owner };
        } else {
          next[to] = piece;
        }
      } else {
        next[to] = piece;
      }

      // Display-side castling: king moved 2+ files, also move the rook
      if (piece.pieceType === 'King') {
        const fileDiff = fileOf(to) - fileOf(from);
        if (Math.abs(fileDiff) >= 2) {
          const rank = rankOf(from);
          if (fileDiff > 0) {
            // Kingside: rook from the far side moves next to king
            const rookFrom = findRookForCastle(next, piece.owner, rank, true);
            if (rookFrom !== -1) {
              const rookTo = squareFrom(fileOf(to) - 1, rank);
              next[rookTo] = next[rookFrom];
              next[rookFrom] = null;
            }
          } else {
            // Queenside
            const rookFrom = findRookForCastle(next, piece.owner, rank, false);
            if (rookFrom !== -1) {
              const rookTo = squareFrom(fileOf(to) + 1, rank);
              next[rookTo] = next[rookFrom];
              next[rookFrom] = null;
            }
          }
        }
      }

      // Display-side en passant: pawn moved diagonally to empty square
      if (piece.pieceType === 'Pawn' && fileOf(from) !== fileOf(to) && prev[to] === null) {
        // The captured pawn is on the same file as destination but same rank as source
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

      if (selectedSquare === null) {
        // First click: select a square with a piece
        setSelectedSquare(sq);
      } else if (selectedSquare === sq) {
        // Click same square: deselect
        setSelectedSquare(null);
      } else {
        // Second click: attempt move
        const moveStr = squareName(selectedSquare) + squareName(sq);

        // Check if this looks like a pawn promotion (display heuristic)
        const piece = board[selectedSquare];
        let finalMove = moveStr;
        if (piece?.pieceType === 'Pawn') {
          const toRank = rankOf(sq);
          const toFile = fileOf(sq);
          const isPromoRank =
            (piece.owner === 'Red' && toRank === 13) ||
            (piece.owner === 'Yellow' && toRank === 0) ||
            (piece.owner === 'Blue' && toFile === 13) ||
            (piece.owner === 'Green' && toFile === 0);
          if (isPromoRank) {
            finalMove += 'q'; // Auto-promote to queen
          }
        }

        // Send position with the new move to engine for validation
        pendingMoveRef.current = finalMove;
        const posCmd =
          moveList.length > 0
            ? `position startpos moves ${moveList.join(' ')} ${finalMove}`
            : `position startpos moves ${finalMove}`;

        sendCommand(posCmd).catch(() => {
          setError('Failed to send command to engine');
          pendingMoveRef.current = null;
        });

        setSelectedSquare(null);
      }
    },
    [selectedSquare, board, moveList, isGameOver, sendCommand],
  );

  /** Request the engine to play a move for the current player. */
  const requestEngineMove = useCallback(() => {
    if (isGameOver) return;
    setError(null);

    // Re-send position then go
    const posCmd =
      moveList.length > 0
        ? `position startpos moves ${moveList.join(' ')}`
        : 'position startpos';

    awaitingBestmoveRef.current = true;
    sendCommand(posCmd)
      .then(() => sendCommand('go'))
      .catch(() => {
        setError('Failed to send command to engine');
        awaitingBestmoveRef.current = false;
      });
  }, [moveList, isGameOver, sendCommand]);

  /** Start a new game. */
  const newGame = useCallback(
    (terrain: boolean) => {
      setBoard(startingPosition());
      setMoveList([]);
      setCurrentPlayer('Red');
      setScores([0, 0, 0, 0]);
      setIsGameOver(false);
      setSelectedSquare(null);
      setLastMoveFrom(null);
      setLastMoveTo(null);
      setLatestInfo(null);
      setError(null);
      pendingMoveRef.current = null;
      awaitingBestmoveRef.current = false;

      sendCommand(`setoption name Terrain value ${terrain ? 'true' : 'false'}`)
        .then(() => sendCommand('position startpos'))
        .then(() => sendCommand('isready'))
        .catch(() => setError('Failed to start new game'));
    },
    [sendCommand],
  );

  /** Process engine messages for game state updates. */
  const handleEngineMessage = useCallback(
    (msg: EngineMessage) => {
      switch (msg.type) {
        case 'error': {
          setError(msg.message);
          // If we had a pending move, it was rejected — restore engine state
          if (pendingMoveRef.current) {
            pendingMoveRef.current = null;
            // Re-send the valid position to restore engine state
            const posCmd =
              moveList.length > 0
                ? `position startpos moves ${moveList.join(' ')}`
                : 'position startpos';
            sendCommand(posCmd).catch(() => {});
          }
          break;
        }
        case 'info': {
          setLatestInfo(msg.data);
          if (msg.data.values) {
            setScores(msg.data.values);
          }

          // If we have a pending user move and got an info response (not error),
          // the position was accepted. Commit the move.
          if (pendingMoveRef.current && !awaitingBestmoveRef.current) {
            const move = pendingMoveRef.current;
            pendingMoveRef.current = null;
            setMoveList((prev) => [...prev, move]);
            applyMoveToBoard(move);
            advancePlayer();

            // Now ask the engine for its response move
            awaitingBestmoveRef.current = true;
            sendCommand('go').catch(() => {
              setError('Failed to request engine move');
              awaitingBestmoveRef.current = false;
            });
          }
          break;
        }
        case 'bestmove': {
          if (awaitingBestmoveRef.current) {
            awaitingBestmoveRef.current = false;
            const engineMove = msg.move;
            // If this was a response to the user's move validation,
            // commit the engine's reply
            setMoveList((prev) => [...prev, engineMove]);
            applyMoveToBoard(engineMove);
            advancePlayer();
          }
          break;
        }
        default:
          break;
      }
    },
    [moveList, sendCommand, applyMoveToBoard, advancePlayer],
  );

  return {
    board,
    moveList,
    currentPlayer,
    scores,
    isGameOver,
    selectedSquare,
    lastMoveFrom,
    lastMoveTo,
    latestInfo,
    error,
    handleSquareClick,
    requestEngineMove,
    newGame,
    handleEngineMessage,
  };
}

// --- Helpers ---

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
  // For vertical players (Red/Yellow), scan along the rank
  // For horizontal players (Blue/Green), scan along the file
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
    // Blue: file 0, Green: file 13. Rank is the "file" for them.
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
