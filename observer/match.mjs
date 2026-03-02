#!/usr/bin/env node
// match.mjs — Two-engine match manager for Project Odin
//
// Plays N games between two engine binaries (engine A vs engine B),
// rotating which engine gets which color seat. Reports Elo difference
// and SPRT result. Logs structured game data for future NNUE training.
//
// Usage: node match.mjs [match_config.json]

import { readFileSync, writeFileSync, appendFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Engine, parseLine, PLAYERS } from './lib/engine.mjs';
import { calculateElo, formatElo } from './elo.mjs';
import { sprtInit, sprtUpdate, sprtStatus } from './sprt.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const configPath = process.argv[2] || join(__dirname, 'match_config.json');
const config = JSON.parse(readFileSync(configPath, 'utf8'));

// ---------------------------------------------------------------------------
// Seat rotation — 6 unique 2-of-4 pairings for balanced color exposure
// ---------------------------------------------------------------------------
const ROTATIONS = [
  { A: ['Red', 'Blue'],    B: ['Yellow', 'Green'] },
  { A: ['Yellow', 'Green'], B: ['Red', 'Blue'] },
  { A: ['Red', 'Yellow'],  B: ['Blue', 'Green'] },
  { A: ['Blue', 'Green'],  B: ['Red', 'Yellow'] },
  { A: ['Red', 'Green'],   B: ['Blue', 'Yellow'] },
  { A: ['Blue', 'Yellow'], B: ['Red', 'Green'] },
];

function getRotation(gameNum) {
  return ROTATIONS[(gameNum - 1) % ROTATIONS.length];
}

// ---------------------------------------------------------------------------
// Play one game between two engines
// ---------------------------------------------------------------------------
async function playMatchGame(engineAPath, engineBPath, gameNum) {
  const rotation = getRotation(gameNum);
  const seatToEngine = {};
  for (const color of rotation.A) seatToEngine[color] = 'A';
  for (const color of rotation.B) seatToEngine[color] = 'B';

  // Spawn fresh engine instances per game (clean state, no TT contamination)
  const engineA = new Engine(engineAPath);
  const engineB = new Engine(engineBPath);
  await Promise.all([engineA.handshake(), engineB.handshake()]);

  // Configure both engines
  for (const eng of [engineA, engineB]) {
    if (config.game_mode) eng.send(`setoption name gamemode value ${config.game_mode}`);
    if (config.eval_profile) eng.send(`setoption name evalprofile value ${config.eval_profile}`);
    if (config.engine_options) {
      for (const [name, value] of Object.entries(config.engine_options)) {
        eng.send(`setoption name ${name} value ${value}`);
      }
    }
  }

  const engines = { A: engineA, B: engineB };
  const moveList = [];
  const moves = [];
  const eliminations = [];
  let currentPlayer = 'Red';
  let gameOver = false;
  let winner = null;
  let ply = 0;

  // Stage 13: Time control clocks (per-player remaining time in ms)
  const clocks = {};
  const tc = config.time_control;
  if (tc) {
    for (const p of PLAYERS) clocks[p] = tc.initial_ms;
  }

  while (!gameOver && ply < (config.stop_at?.max_ply ?? 200)) {
    const engineLabel = seatToEngine[currentPlayer];
    const engine = engines[engineLabel];

    // Send position and go
    const posCmd = moveList.length === 0
      ? 'position startpos'
      : `position startpos moves ${moveList.join(' ')}`;
    engine.send(posCmd);

    // Build go command: time control or fixed depth (Stage 13)
    let goCmd;
    if (tc) {
      const inc = tc.increment_ms ?? 0;
      goCmd = `go wtime ${clocks.Red} winc ${inc} btime ${clocks.Blue} binc ${inc} ytime ${clocks.Yellow} yinc ${inc} gtime ${clocks.Green} ginc ${inc}`;
    } else {
      goCmd = `go depth ${config.depth}`;
    }
    engine.send(goCmd);

    // Collect output until bestmove
    const rawLines = [];
    let bestmove = null;
    let lastSearch = null;

    while (true) {
      const line = await engine.readLine();
      rawLines.push(line);
      const p = parseLine(line);

      if (p.type === 'eliminated') {
        eliminations.push({ player: p.color, reason: p.reason, at_ply: ply });
      } else if (p.type === 'gameover') {
        winner = p.winner;
        gameOver = true;
      } else if (p.type === 'nextturn') {
        // Protocol tells us next player — we track it ourselves
      } else if (p.type === 'search_info') {
        lastSearch = p;
      } else if (p.type === 'bestmove') {
        bestmove = p.move;
        break;
      }
    }

    if (bestmove) {
      const moveTimeMs = lastSearch?.time_ms ?? 0;

      moves.push({
        ply,
        player: currentPlayer,
        engine: engineLabel,
        move: bestmove,
        score_cp: lastSearch?.score_cp ?? null,
        depth: lastSearch?.depth ?? null,
        nodes: lastSearch?.nodes ?? null,
        nps: lastSearch?.nps ?? null,
        time_ms: moveTimeMs,
        pv: lastSearch?.pv ?? null,
        phase: lastSearch?.phase ?? null,
        position_moves: moveList.join(' '),
        v1: lastSearch?.v1 ?? null,
        v2: lastSearch?.v2 ?? null,
        v3: lastSearch?.v3 ?? null,
        v4: lastSearch?.v4 ?? null,
      });

      // Stage 13: Update clock for timed games
      if (tc) {
        clocks[currentPlayer] -= moveTimeMs;
        clocks[currentPlayer] += (tc.increment_ms ?? 0);

        // Time forfeit check
        if (clocks[currentPlayer] <= 0) {
          eliminations.push({ player: currentPlayer, reason: 'timeout', at_ply: ply });
          clocks[currentPlayer] = 0;
        }
      }

      moveList.push(bestmove);
      ply++;

      // Advance to next non-eliminated player
      const eliminatedSet = new Set(eliminations.map((e) => e.player));
      let next = PLAYERS[(PLAYERS.indexOf(currentPlayer) + 1) % 4];
      for (let i = 0; i < 3; i++) {
        if (!eliminatedSet.has(next)) break;
        next = PLAYERS[(PLAYERS.indexOf(next) + 1) % 4];
      }
      currentPlayer = next;
    }

    if (gameOver) break;
  }

  // Clean up engines
  engineA.close();
  engineB.close();

  // Determine which engine won
  let winnerEngine = null;
  if (winner && seatToEngine[winner]) {
    winnerEngine = seatToEngine[winner];
  }

  return {
    format_version: 1,
    game_id: gameNum,
    timestamp: new Date().toISOString(),
    config: {
      engine_a: config.engine_a,
      engine_b: config.engine_b,
      depth: config.depth,
      time_control: config.time_control ?? null,
      game_mode: config.game_mode,
      eval_profile: config.eval_profile,
    },
    seat_assignment: seatToEngine,
    result: {
      winner: winner ?? null,
      winner_engine: winnerEngine,
      eliminations,
      total_ply: ply,
    },
    moves,
  };
}

// ---------------------------------------------------------------------------
// Datagen helpers — sample positions and backfill game results
// ---------------------------------------------------------------------------
const PLAYER_INDEX = { Red: 0, Blue: 1, Yellow: 2, Green: 3 };

function computeGameResult(gameRecord) {
  const result = [0.0, 0.0, 0.0, 0.0];
  if (gameRecord.result.winner) {
    result[PLAYER_INDEX[gameRecord.result.winner]] = 1.0;
  } else {
    // Draw at ply cap: surviving players share equally
    const eliminatedSet = new Set(gameRecord.result.eliminations.map((e) => e.player));
    const survivors = PLAYERS.filter((p) => !eliminatedSet.has(p));
    const share = survivors.length > 0 ? 1.0 / survivors.length : 0.25;
    for (const p of survivors) result[PLAYER_INDEX[p]] = share;
  }
  return result;
}

function samplePositions(gameRecord, interval) {
  const samples = [];
  const eliminatedSet = new Set();
  let nextSamplePly = 4 + Math.floor(Math.random() * interval);

  for (const move of gameRecord.moves) {
    // Track eliminations up to this ply
    for (const e of gameRecord.result.eliminations) {
      if (e.at_ply <= move.ply) eliminatedSet.add(e.player);
    }

    if (move.ply < 4) continue;                    // Skip first 4 plies
    if (eliminatedSet.has(move.player)) continue;  // Skip eliminated players
    if (move.v1 == null || move.v2 == null || move.v3 == null || move.v4 == null) continue; // Skip if no MCTS values

    if (move.ply >= nextSamplePly) {
      samples.push({
        position_moves: move.position_moves,
        ply: move.ply,
        side_to_move: move.player,
        score_cp: move.score_cp ?? 0,
        v1: move.v1,
        v2: move.v2,
        v3: move.v3,
        v4: move.v4,
        depth: move.depth ?? 0,
        game_id: gameRecord.game_id,
      });
      // Next sample at random interval (interval ± 1 ply)
      nextSamplePly = move.ply + interval + Math.floor(Math.random() * 3) - 1;
    }
  }
  return samples;
}

// ---------------------------------------------------------------------------
// Datagen mode — self-play data generation for NNUE training
// ---------------------------------------------------------------------------
async function runDatagen() {
  const enginePath = resolve(__dirname, config.engine_a);
  if (!existsSync(enginePath)) {
    console.error(`Engine not found: ${enginePath}`);
    process.exit(1);
  }

  const outputFile = resolve(__dirname, config.output_file || 'training_data_gen0.jsonl');
  const interval = config.sample_interval || 4;
  const totalGames = config.games || 1000;

  console.log(`[datagen] Engine: ${enginePath}`);
  console.log(`[datagen] Games: ${totalGames} | Depth: ${config.depth} | Sample interval: ${interval}`);
  console.log(`[datagen] Output: ${outputFile}`);
  console.log('');

  // Clear output file
  writeFileSync(outputFile, '');

  let totalSamples = 0;

  for (let i = 1; i <= totalGames; i++) {
    process.stdout.write(`[datagen] Game ${i}/${totalGames} ... `);

    // Self-play: same engine for both sides
    const gameRecord = await playMatchGame(enginePath, enginePath, i);

    // Compute game result and sample positions
    const gameResult = computeGameResult(gameRecord);
    const samples = samplePositions(gameRecord, interval);

    // Backfill game_result and write JSONL
    if (samples.length > 0) {
      const lines = samples.map((s) => {
        s.game_result = gameResult;
        return JSON.stringify(s);
      }).join('\n') + '\n';
      appendFileSync(outputFile, lines);
    }

    totalSamples += samples.length;
    console.log(`${gameRecord.result.total_ply} ply, winner: ${gameRecord.result.winner ?? 'none'}, samples: ${samples.length} (total: ${totalSamples})`);
  }

  console.log('');
  console.log(`[datagen] Complete: ${totalGames} games, ${totalSamples} samples written to ${outputFile}`);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  // Datagen mode — triggered by config.mode === 'datagen'
  if (config.mode === 'datagen') {
    return runDatagen();
  }

  const outputDir = resolve(__dirname, config.output_dir);
  if (!existsSync(outputDir)) mkdirSync(outputDir, { recursive: true });

  const engineAPath = resolve(__dirname, config.engine_a);
  const engineBPath = resolve(__dirname, config.engine_b);

  // Verify engine binaries exist
  if (!existsSync(engineAPath)) {
    console.error(`Engine A not found: ${engineAPath}`);
    process.exit(1);
  }
  if (!existsSync(engineBPath)) {
    console.error(`Engine B not found: ${engineBPath}`);
    console.error('Hint: copy the baseline binary or run: run_match.bat');
    process.exit(1);
  }

  console.log(`Engine A: ${engineAPath}`);
  console.log(`Engine B: ${engineBPath}`);
  const tcInfo = config.time_control
    ? `TC: ${config.time_control.initial_ms}ms+${config.time_control.increment_ms ?? 0}ms`
    : `Depth: ${config.depth}`;
  console.log(`Games: ${config.games} | ${tcInfo} | Mode: ${config.game_mode} | Profile: ${config.eval_profile}`);
  console.log(`SPRT: H0 elo<=${config.sprt?.elo0 ?? 0}, H1 elo>=${config.sprt?.elo1 ?? 5}, alpha=${config.sprt?.alpha ?? 0.05}, beta=${config.sprt?.beta ?? 0.05}`);
  console.log('');

  // Initialize SPRT
  const sprtState = sprtInit(config.sprt || {});
  const eloResults = [];
  const allGames = [];

  for (let i = 1; i <= config.games; i++) {
    process.stdout.write(`Game ${i}/${config.games} (rotation ${((i - 1) % ROTATIONS.length) + 1}/6) ... `);

    const gameRecord = await playMatchGame(engineAPath, engineBPath, i);
    allGames.push(gameRecord);

    // Write per-game JSON
    const gameFile = join(outputDir, `game_${String(i).padStart(4, '0')}.json`);
    writeFileSync(gameFile, JSON.stringify(gameRecord, null, 2));

    const we = gameRecord.result.winner_engine;
    const gameResult = we === 'A' ? 'A' : we === 'B' ? 'B' : 'draw';
    eloResults.push({ winner: gameResult });

    console.log(`${gameRecord.result.total_ply} ply, winner: ${gameRecord.result.winner ?? 'none'} (engine ${we ?? 'draw'})`);

    // SPRT check
    const sprtResult = sprtUpdate(sprtState, gameResult);
    if (sprtResult.decision !== 'continue') {
      console.log('');
      console.log(`SPRT decision reached after ${sprtResult.games} games!`);
      console.log(sprtStatus(sprtState));
      break;
    }
  }

  // Final results
  console.log('\n' + '='.repeat(60));
  console.log('MATCH RESULTS');
  console.log('='.repeat(60));

  const elo = calculateElo(eloResults);
  console.log(formatElo(elo));
  console.log('');
  console.log(sprtStatus(sprtState));

  // Write match summary
  const summary = {
    timestamp: new Date().toISOString(),
    engine_a: config.engine_a,
    engine_b: config.engine_b,
    config: {
      games: config.games,
      depth: config.depth,
      game_mode: config.game_mode,
      eval_profile: config.eval_profile,
    },
    results: {
      total_games: eloResults.length,
      engine_a_wins: elo.wins,
      engine_b_wins: elo.losses,
      draws: elo.draws,
    },
    elo: {
      elo_diff: elo.elo_diff,
      ci_low: elo.ci_low,
      ci_high: elo.ci_high,
      actual_score: elo.actual_score,
    },
    sprt: {
      decision: sprtState.llr >= sprtState.upperBound ? 'accept_h1'
        : sprtState.llr <= sprtState.lowerBound ? 'accept_h0'
        : 'inconclusive',
      llr: sprtState.llr,
      games_played: eloResults.length,
      lower_bound: sprtState.lowerBound,
      upper_bound: sprtState.upperBound,
    },
  };

  writeFileSync(join(outputDir, 'match_summary.json'), JSON.stringify(summary, null, 2));
  writeFileSync(join(outputDir, 'all_games.json'), JSON.stringify(allGames, null, 2));

  console.log(`\nReports saved to: ${outputDir}`);
}

main().catch((err) => {
  console.error('Match error:', err);
  process.exit(1);
});
