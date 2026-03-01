// Parse engine stdout lines into typed messages.
// Matches exact formats from odin-engine/src/protocol/emitter.rs.

import type { EngineMessage, InfoData } from '../types/protocol';
import type { Player } from '../types/board';

function isValidPlayerColor(s: string): s is Player {
  return s === 'Red' || s === 'Blue' || s === 'Yellow' || s === 'Green';
}

/** Parse a single engine output line into a structured message. */
export function parseEngineOutput(line: string): EngineMessage {
  const trimmed = line.trim();

  if (trimmed === 'odinok') {
    return { type: 'odinok' };
  }
  if (trimmed === 'readyok') {
    return { type: 'readyok' };
  }

  // id name <name> or id author <author>
  if (trimmed.startsWith('id ')) {
    const rest = trimmed.slice(3);
    const spaceIdx = rest.indexOf(' ');
    if (spaceIdx !== -1) {
      return {
        type: 'id',
        key: rest.slice(0, spaceIdx),
        value: rest.slice(spaceIdx + 1),
      };
    }
    return { type: 'unknown', raw: trimmed };
  }

  // bestmove <move> [ponder <move>]
  if (trimmed.startsWith('bestmove ')) {
    const parts = trimmed.split(/\s+/);
    const move = parts[1];
    const ponder = parts[2] === 'ponder' ? parts[3] : undefined;
    return { type: 'bestmove', move, ponder };
  }

  // info string eliminated <color> [reason]
  // The engine may append a reason word (e.g. "checkmate", "stalemate") — extract
  // only the first token so "Red checkmate" still parses as player Red.
  if (trimmed.startsWith('info string eliminated ')) {
    const rest = trimmed.slice('info string eliminated '.length).trim();
    const color = rest.split(/\s+/)[0];
    if (isValidPlayerColor(color)) {
      return { type: 'eliminated', player: color };
    }
  }

  // info string nextturn <color>
  if (trimmed.startsWith('info string nextturn ')) {
    const color = trimmed.slice('info string nextturn '.length).trim();
    if (isValidPlayerColor(color)) {
      return { type: 'nextturn', player: color };
    }
  }

  // info string gameover <color|none>
  if (trimmed.startsWith('info string gameover ')) {
    const winner = trimmed.slice('info string gameover '.length).trim();
    return { type: 'gameover', winner: isValidPlayerColor(winner) ? winner : null };
  }

  // info string Error: <msg>
  if (trimmed.startsWith('info string Error: ')) {
    return { type: 'error', message: trimmed.slice('info string Error: '.length) };
  }

  // info [key value ...]
  if (trimmed.startsWith('info')) {
    return { type: 'info', data: parseInfoData(trimmed) };
  }

  return { type: 'unknown', raw: trimmed };
}

/** Parse the data fields from an `info` line. */
function parseInfoData(line: string): InfoData {
  const data: InfoData = {};
  const tokens = line.split(/\s+/);
  let i = 1; // skip "info"

  while (i < tokens.length) {
    switch (tokens[i]) {
      case 'depth':
        data.depth = parseInt(tokens[++i], 10);
        break;
      case 'seldepth':
        data.seldepth = parseInt(tokens[++i], 10);
        break;
      case 'score':
        if (tokens[i + 1] === 'cp') {
          i++;
          data.scoreCp = parseInt(tokens[++i], 10);
        }
        break;
      case 'v1': {
        // parseFloat handles both BRS integer centipawns (4443) and
        // MCTS float win probabilities (0.753).
        const v1 = parseFloat(tokens[++i]);
        // Expect v2, v3, v4 to follow
        i++; // skip 'v2'
        const v2 = parseFloat(tokens[++i]);
        i++; // skip 'v3'
        const v3 = parseFloat(tokens[++i]);
        i++; // skip 'v4'
        const v4 = parseFloat(tokens[++i]);
        data.values = [v1, v2, v3, v4];
        break;
      }
      case 's1': {
        const s1 = parseInt(tokens[++i], 10);
        i++; // skip 's2'
        const s2 = parseInt(tokens[++i], 10);
        i++; // skip 's3'
        const s3 = parseInt(tokens[++i], 10);
        i++; // skip 's4'
        const s4 = parseInt(tokens[++i], 10);
        data.ffaScores = [s1, s2, s3, s4];
        break;
      }
      case 'nodes':
        data.nodes = parseInt(tokens[++i], 10);
        break;
      case 'nps':
        data.nps = parseInt(tokens[++i], 10);
        break;
      case 'time':
        data.timeMs = parseInt(tokens[++i], 10);
        break;
      case 'pv': {
        const pv: string[] = [];
        i++;
        while (i < tokens.length && !isInfoKeyword(tokens[i])) {
          pv.push(tokens[i]);
          i++;
        }
        data.pv = pv;
        continue; // don't increment i again
      }
      case 'phase':
        data.phase = tokens[++i] as 'brs' | 'mcts';
        break;
      case 'brs_surviving':
        data.brsSurviving = parseInt(tokens[++i], 10);
        break;
      case 'mcts_sims':
        data.mctsSims = parseInt(tokens[++i], 10);
        break;
      default:
        break;
    }
    i++;
  }

  return data;
}

/** Check if a token is a known info keyword (used to terminate pv parsing). */
function isInfoKeyword(token: string): boolean {
  return [
    'depth', 'seldepth', 'score', 'v1', 's1', 'nodes', 'nps',
    'time', 'pv', 'phase', 'brs_surviving', 'mcts_sims',
  ].includes(token);
}
