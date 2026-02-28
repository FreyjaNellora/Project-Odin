#!/usr/bin/env node
// tune.mjs — Parameter tuning via self-play for Project Odin (Stage 13)
//
// Runs A/B matches between the engine with a tuned parameter value
// vs the engine with defaults. Uses setoption to inject the parameter
// at runtime (no recompilation needed).
//
// Usage:
//   node tune.mjs --param tactical_margin --values 100,150,200,250 --games 50
//   node tune.mjs --param brs_max_depth --values 6,8,10 --games 30 --depth 6
//
// Options:
//   --param <name>        Parameter name (sent as setoption)
//   --values <v1,v2,...>   Comma-separated values to test
//   --games <N>           Games per value (default: 50)
//   --depth <N>           Search depth per move (default: 6)
//   --engine <path>       Engine binary (default: ../target/release/odin-engine.exe)
//   --game-mode <mode>    Game mode: ffa or lks (default: ffa)

import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Engine, parseLine, PLAYERS } from './lib/engine.mjs';
import { calculateElo, formatElo } from './elo.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------
// Parse CLI arguments
// ---------------------------------------------------------------------------
function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {
    param: null,
    values: [],
    games: 50,
    depth: 6,
    engine: resolve(__dirname, '../target/release/odin-engine.exe'),
    gameMode: 'ffa',
  };

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case '--param':
        opts.param = args[++i];
        break;
      case '--values':
        opts.values = args[++i].split(',').map((v) => v.trim());
        break;
      case '--games':
        opts.games = parseInt(args[++i], 10);
        break;
      case '--depth':
        opts.depth = parseInt(args[++i], 10);
        break;
      case '--engine':
        opts.engine = resolve(args[++i]);
        break;
      case '--game-mode':
        opts.gameMode = args[++i];
        break;
      default:
        console.error(`Unknown option: ${args[i]}`);
        process.exit(1);
    }
  }

  if (!opts.param || opts.values.length === 0) {
    console.error('Usage: node tune.mjs --param <name> --values <v1,v2,...> [--games N] [--depth N]');
    process.exit(1);
  }

  return opts;
}

// ---------------------------------------------------------------------------
// Seat rotation (same as match.mjs)
// ---------------------------------------------------------------------------
const ROTATIONS = [
  { A: ['Red', 'Blue'], B: ['Yellow', 'Green'] },
  { A: ['Yellow', 'Green'], B: ['Red', 'Blue'] },
  { A: ['Red', 'Yellow'], B: ['Blue', 'Green'] },
  { A: ['Blue', 'Green'], B: ['Red', 'Yellow'] },
  { A: ['Red', 'Green'], B: ['Blue', 'Yellow'] },
  { A: ['Blue', 'Yellow'], B: ['Red', 'Green'] },
];

// ---------------------------------------------------------------------------
// Play one game: engine A (tuned) vs engine B (default)
// ---------------------------------------------------------------------------
async function playTuneGame(enginePath, paramName, paramValue, gameNum, depth, gameMode) {
  const rotation = ROTATIONS[(gameNum - 1) % ROTATIONS.length];
  const seatToEngine = {};
  for (const color of rotation.A) seatToEngine[color] = 'A';
  for (const color of rotation.B) seatToEngine[color] = 'B';

  const engineA = new Engine(enginePath); // tuned
  const engineB = new Engine(enginePath); // default
  await Promise.all([engineA.handshake(), engineB.handshake()]);

  // Configure
  for (const eng of [engineA, engineB]) {
    eng.send(`setoption name gamemode value ${gameMode}`);
  }
  // Apply the tuned parameter to engine A only
  engineA.send(`setoption name ${paramName} value ${paramValue}`);

  const engines = { A: engineA, B: engineB };
  const moveList = [];
  let currentPlayer = 'Red';
  let gameOver = false;
  let winner = null;
  let ply = 0;
  const eliminatedSet = new Set();

  while (!gameOver && ply < 200) {
    const engineLabel = seatToEngine[currentPlayer];
    const engine = engines[engineLabel];

    const posCmd = moveList.length === 0
      ? 'position startpos'
      : `position startpos moves ${moveList.join(' ')}`;
    engine.send(posCmd);
    engine.send(`go depth ${depth}`);

    let bestmove = null;
    while (true) {
      const line = await engine.readLine();
      const p = parseLine(line);
      if (p.type === 'eliminated') {
        eliminatedSet.add(p.color);
      } else if (p.type === 'gameover') {
        winner = p.winner;
        gameOver = true;
      } else if (p.type === 'bestmove') {
        bestmove = p.move;
        break;
      }
    }

    if (bestmove) {
      moveList.push(bestmove);
      ply++;
      let next = PLAYERS[(PLAYERS.indexOf(currentPlayer) + 1) % 4];
      for (let i = 0; i < 3; i++) {
        if (!eliminatedSet.has(next)) break;
        next = PLAYERS[(PLAYERS.indexOf(next) + 1) % 4];
      }
      currentPlayer = next;
    }
    if (gameOver) break;
  }

  engineA.close();
  engineB.close();

  let winnerEngine = null;
  if (winner && seatToEngine[winner]) winnerEngine = seatToEngine[winner];
  return { winner: winnerEngine, ply };
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  const opts = parseArgs();

  console.log(`Tuning: ${opts.param}`);
  console.log(`Values: ${opts.values.join(', ')}`);
  console.log(`Games per value: ${opts.games} | Depth: ${opts.depth} | Mode: ${opts.gameMode}`);
  console.log(`Engine: ${opts.engine}`);
  console.log('');

  const results = [];

  for (const value of opts.values) {
    console.log(`--- Testing ${opts.param} = ${value} ---`);
    const eloResults = [];

    for (let i = 1; i <= opts.games; i++) {
      process.stdout.write(`  Game ${i}/${opts.games} ... `);
      const game = await playTuneGame(
        opts.engine, opts.param, value, i, opts.depth, opts.gameMode
      );
      const result = game.winner === 'A' ? 'A' : game.winner === 'B' ? 'B' : 'draw';
      eloResults.push({ winner: result });
      console.log(`${game.ply} ply, winner: ${result}`);
    }

    const elo = calculateElo(eloResults);
    console.log(`  ${formatElo(elo)}`);
    console.log('');
    results.push({ param: opts.param, value, elo });
  }

  // Summary
  console.log('='.repeat(60));
  console.log('TUNING RESULTS');
  console.log('='.repeat(60));
  console.log(`Parameter: ${opts.param}`);
  console.log('');

  for (const r of results) {
    const sign = r.elo.elo_diff >= 0 ? '+' : '';
    console.log(
      `  ${opts.param}=${r.value}: ${sign}${r.elo.elo_diff.toFixed(1)} Elo ` +
      `(${r.elo.wins}W/${r.elo.draws}D/${r.elo.losses}L, ` +
      `95% CI: [${r.elo.ci_low.toFixed(1)}, ${r.elo.ci_high.toFixed(1)}])`
    );
  }

  // Recommend best
  const best = results.reduce((a, b) => (a.elo.elo_diff > b.elo.elo_diff ? a : b));
  console.log('');
  console.log(`Recommended: ${opts.param} = ${best.value} (${best.elo.elo_diff >= 0 ? '+' : ''}${best.elo.elo_diff.toFixed(1)} Elo)`);
}

main().catch((err) => {
  console.error('Tuning error:', err);
  process.exit(1);
});
