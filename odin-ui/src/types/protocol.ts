// Protocol message types matching odin-engine's Odin Protocol.
// Used to parse engine stdout responses.

import type { Player } from './board';

/** Parsed engine output line. */
export type EngineMessage =
  | { type: 'id'; key: string; value: string }
  | { type: 'odinok' }
  | { type: 'readyok' }
  | { type: 'bestmove'; move: string; ponder?: string }
  | { type: 'info'; data: InfoData }
  | { type: 'error'; message: string }
  /** A player was eliminated (checkmate/stalemate/DKW king captured). */
  | { type: 'eliminated'; player: Player }
  /** Whose turn comes next after the engine's move (skips eliminated players). */
  | { type: 'nextturn'; player: Player }
  /** Game has ended; winner is the surviving player or null for a draw. */
  | { type: 'gameover'; winner: Player | null }
  | { type: 'unknown'; raw: string };

/** Parsed search info data from `info` lines. */
export interface InfoData {
  depth?: number;
  seldepth?: number;
  scoreCp?: number;
  /** Per-player values [Red, Blue, Yellow, Green]. */
  values?: [number, number, number, number];
  /** FFA game scores [Red, Blue, Yellow, Green] (capture points, checkmate bonuses). */
  ffaScores?: [number, number, number, number];
  nodes?: number;
  nps?: number;
  timeMs?: number;
  pv?: string[];
  phase?: 'brs' | 'mcts';
  brsSurviving?: number;
  mctsSims?: number;
}
