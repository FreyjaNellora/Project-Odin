// Parse engine stdout lines into typed messages.
// Matches exact formats from odin-engine/src/protocol/emitter.rs.

import type { EngineMessage, InfoData } from '../types/protocol';

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
        const v1 = parseInt(tokens[++i], 10);
        // Expect v2, v3, v4 to follow
        i++; // skip 'v2'
        const v2 = parseInt(tokens[++i], 10);
        i++; // skip 'v3'
        const v3 = parseInt(tokens[++i], 10);
        i++; // skip 'v4'
        const v4 = parseInt(tokens[++i], 10);
        data.values = [v1, v2, v3, v4];
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
    'depth', 'seldepth', 'score', 'v1', 'nodes', 'nps',
    'time', 'pv', 'phase', 'brs_surviving', 'mcts_sims',
  ].includes(token);
}
