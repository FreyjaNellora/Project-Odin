// Protocol message types matching odin-engine's Odin Protocol.
// Used to parse engine stdout responses.

/** Parsed engine output line. */
export type EngineMessage =
  | { type: 'id'; key: string; value: string }
  | { type: 'odinok' }
  | { type: 'readyok' }
  | { type: 'bestmove'; move: string; ponder?: string }
  | { type: 'info'; data: InfoData }
  | { type: 'error'; message: string }
  | { type: 'unknown'; raw: string };

/** Parsed search info data from `info` lines. */
export interface InfoData {
  depth?: number;
  seldepth?: number;
  scoreCp?: number;
  /** Per-player values [Red, Blue, Yellow, Green]. */
  values?: [number, number, number, number];
  nodes?: number;
  nps?: number;
  timeMs?: number;
  pv?: string[];
  phase?: 'brs' | 'mcts';
  brsSurviving?: number;
  mctsSims?: number;
}
