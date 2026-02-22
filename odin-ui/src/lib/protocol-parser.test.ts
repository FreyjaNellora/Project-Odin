import { describe, it, expect } from 'vitest';
import { parseEngineOutput } from './protocol-parser';

describe('parseEngineOutput', () => {
  it('parses odinok', () => {
    expect(parseEngineOutput('odinok')).toEqual({ type: 'odinok' });
  });

  it('parses odinok with whitespace', () => {
    expect(parseEngineOutput('  odinok  ')).toEqual({ type: 'odinok' });
  });

  it('parses readyok', () => {
    expect(parseEngineOutput('readyok')).toEqual({ type: 'readyok' });
  });

  it('parses id name', () => {
    const msg = parseEngineOutput('id name Odin 0.4.0');
    expect(msg).toEqual({ type: 'id', key: 'name', value: 'Odin 0.4.0' });
  });

  it('parses id author', () => {
    const msg = parseEngineOutput('id author Project Odin');
    expect(msg).toEqual({ type: 'id', key: 'author', value: 'Project Odin' });
  });

  it('returns unknown for malformed id', () => {
    const msg = parseEngineOutput('id');
    expect(msg.type).toBe('unknown');
  });

  it('parses bestmove without ponder', () => {
    const msg = parseEngineOutput('bestmove d2d4');
    expect(msg).toEqual({ type: 'bestmove', move: 'd2d4', ponder: undefined });
  });

  it('parses bestmove with ponder', () => {
    const msg = parseEngineOutput('bestmove d2d4 ponder d13d11');
    expect(msg).toEqual({ type: 'bestmove', move: 'd2d4', ponder: 'd13d11' });
  });

  it('parses info string error', () => {
    const msg = parseEngineOutput('info string Error: illegal move "zz99"');
    expect(msg).toEqual({ type: 'error', message: 'illegal move "zz99"' });
  });

  it('parses info with full data', () => {
    const line = 'info depth 3 score cp 42 v1 100 v2 80 v3 60 v4 40 nodes 1234 nps 5678 time 100 pv d2d4 a4a6 phase brs brs_surviving 4';
    const msg = parseEngineOutput(line);
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.depth).toBe(3);
      expect(msg.data.scoreCp).toBe(42);
      expect(msg.data.values).toEqual([100, 80, 60, 40]);
      expect(msg.data.nodes).toBe(1234);
      expect(msg.data.nps).toBe(5678);
      expect(msg.data.timeMs).toBe(100);
      expect(msg.data.pv).toEqual(['d2d4', 'a4a6']);
      expect(msg.data.phase).toBe('brs');
      expect(msg.data.brsSurviving).toBe(4);
    }
  });

  it('parses info with minimal data', () => {
    const msg = parseEngineOutput('info depth 1');
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.depth).toBe(1);
      expect(msg.data.scoreCp).toBeUndefined();
      expect(msg.data.values).toBeUndefined();
      expect(msg.data.nodes).toBeUndefined();
      expect(msg.data.pv).toBeUndefined();
    }
  });

  it('parses empty info line', () => {
    const msg = parseEngineOutput('info');
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.depth).toBeUndefined();
    }
  });

  it('parses info with mcts fields', () => {
    const msg = parseEngineOutput('info phase mcts mcts_sims 500');
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.phase).toBe('mcts');
      expect(msg.data.mctsSims).toBe(500);
    }
  });

  it('parses info with pv at end', () => {
    const msg = parseEngineOutput('info depth 2 pv d2d4 a4a6 h13h11');
    expect(msg.type).toBe('info');
    if (msg.type === 'info') {
      expect(msg.data.depth).toBe(2);
      expect(msg.data.pv).toEqual(['d2d4', 'a4a6', 'h13h11']);
    }
  });

  // --- eliminated ---

  it('parses info string eliminated without reason', () => {
    const msg = parseEngineOutput('info string eliminated Red');
    expect(msg).toEqual({ type: 'eliminated', player: 'Red' });
  });

  it('parses info string eliminated with checkmate reason', () => {
    // Regression: parser must extract only the first word, not "Red checkmate".
    const msg = parseEngineOutput('info string eliminated Red checkmate');
    expect(msg).toEqual({ type: 'eliminated', player: 'Red' });
  });

  it('parses info string eliminated with stalemate reason', () => {
    const msg = parseEngineOutput('info string eliminated Blue stalemate');
    expect(msg).toEqual({ type: 'eliminated', player: 'Blue' });
  });

  it('parses eliminated for all player colors', () => {
    for (const color of ['Red', 'Blue', 'Yellow', 'Green']) {
      const msg = parseEngineOutput(`info string eliminated ${color}`);
      expect(msg).toEqual({ type: 'eliminated', player: color });
    }
  });

  it('ignores eliminated with unknown color', () => {
    const msg = parseEngineOutput('info string eliminated Purple');
    expect(msg.type).not.toBe('eliminated');
  });

  // --- nextturn ---

  it('parses info string nextturn', () => {
    const msg = parseEngineOutput('info string nextturn Blue');
    expect(msg).toEqual({ type: 'nextturn', player: 'Blue' });
  });

  it('parses nextturn for all player colors', () => {
    for (const color of ['Red', 'Blue', 'Yellow', 'Green']) {
      const msg = parseEngineOutput(`info string nextturn ${color}`);
      expect(msg).toEqual({ type: 'nextturn', player: color });
    }
  });

  // --- gameover ---

  it('parses info string gameover with winner', () => {
    const msg = parseEngineOutput('info string gameover Green');
    expect(msg).toEqual({ type: 'gameover', winner: 'Green' });
  });

  it('parses info string gameover none (draw)', () => {
    const msg = parseEngineOutput('info string gameover none');
    expect(msg).toEqual({ type: 'gameover', winner: null });
  });

  it('returns unknown for unrecognized lines', () => {
    const msg = parseEngineOutput('something random');
    expect(msg).toEqual({ type: 'unknown', raw: 'something random' });
  });

  it('returns unknown for empty string', () => {
    const msg = parseEngineOutput('');
    expect(msg).toEqual({ type: 'unknown', raw: '' });
  });
});
