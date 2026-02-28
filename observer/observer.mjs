#!/usr/bin/env node
// observer.mjs — Automated gameplay observer for Project Odin
//
// Spawns the engine, plays N games, captures all protocol output,
// writes structured reports. No analysis — just records what happened.
//
// Usage: node observer.mjs [config.json]

import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const configPath = process.argv[2] || join(__dirname, 'config.json');
const config = JSON.parse(readFileSync(configPath, 'utf8'));

const PLAYERS = ['Red', 'Blue', 'Yellow', 'Green'];

// ---------------------------------------------------------------------------
// Engine process wrapper — line-based stdin/stdout communication
// ---------------------------------------------------------------------------
class Engine {
  #proc;
  #rl;
  #lineQueue = [];
  #lineResolve = null;

  constructor(enginePath) {
    this.#proc = spawn(enginePath, [], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    this.#proc.on('error', (err) => {
      console.error(`Engine failed to start: ${err.message}`);
      process.exit(1);
    });
    this.#rl = createInterface({ input: this.#proc.stdout });
    this.#rl.on('line', (line) => {
      if (this.#lineResolve) {
        const r = this.#lineResolve;
        this.#lineResolve = null;
        r(line);
      } else {
        this.#lineQueue.push(line);
      }
    });
  }

  send(cmd) {
    this.#proc.stdin.write(cmd + '\n');
  }

  async readLine() {
    if (this.#lineQueue.length > 0) return this.#lineQueue.shift();
    return new Promise((resolve) => { this.#lineResolve = resolve; });
  }

  /** Read lines until one starts with `prefix`. Returns all lines including the match. */
  async readUntil(prefix) {
    const lines = [];
    while (true) {
      const line = await this.readLine();
      lines.push(line);
      if (line.startsWith(prefix)) break;
    }
    return lines;
  }

  async handshake() {
    this.send('odin');
    await this.readUntil('odinok');
    this.send('isready');
    await this.readUntil('readyok');
  }

  close() {
    this.send('quit');
    setTimeout(() => this.#proc.kill(), 500);
  }
}

// ---------------------------------------------------------------------------
// Parse one protocol line into a typed object
// ---------------------------------------------------------------------------
function parseLine(line) {
  if (line.startsWith('info string eliminated ')) {
    const rest = line.slice('info string eliminated '.length);
    const color = rest.split(' ')[0];
    const reason = rest.split(' ').slice(1).join(' ') || null;
    return { type: 'eliminated', color, reason, raw: line };
  }
  if (line.startsWith('info string gameover ')) {
    return { type: 'gameover', winner: line.slice('info string gameover '.length).trim(), raw: line };
  }
  if (line.startsWith('info string nextturn ')) {
    return { type: 'nextturn', player: line.slice('info string nextturn '.length).trim(), raw: line };
  }
  if (line.startsWith('info string')) {
    return { type: 'info_string', raw: line };
  }
  if (line.startsWith('bestmove ')) {
    return { type: 'bestmove', move: line.slice('bestmove '.length).trim(), raw: line };
  }
  if (line.startsWith('info ')) {
    // Search info — pull out key fields
    const o = { type: 'search_info', raw: line };
    const t = line.split(' ');
    for (let i = 1; i < t.length; i++) {
      switch (t[i]) {
        case 'depth':    o.depth = +t[++i]; break;
        case 'seldepth': o.seldepth = +t[++i]; break;
        case 'score':    if (t[i+1] === 'cp') { o.score_cp = +t[i+2]; i += 2; } break;
        case 'v1': o.v1 = +t[++i]; break;
        case 'v2': o.v2 = +t[++i]; break;
        case 'v3': o.v3 = +t[++i]; break;
        case 'v4': o.v4 = +t[++i]; break;
        case 'nodes': o.nodes = +t[++i]; break;
        case 'nps':   o.nps = +t[++i]; break;
        case 'time':  o.time_ms = +t[++i]; break;
        case 'phase': o.phase = t[++i]; break;
        case 'pv':    o.pv = t.slice(i+1).join(' '); i = t.length; break;
      }
    }
    return o;
  }
  return { type: 'unknown', raw: line };
}

// ---------------------------------------------------------------------------
// Play one game — returns a structured record
// ---------------------------------------------------------------------------
async function playGame(engine, gameNum) {
  const record = {
    game: gameNum,
    settings: {
      depth: config.depth,
      game_mode: config.game_mode,
      eval_profile: config.eval_profile,
    },
    plies: [],          // every ply: { ply, player, move, eval, depth, nodes, pv, raw_lines }
    eliminations: [],   // { player, reason, at_ply }
    winner: null,
    total_ply: 0,
  };

  const moveList = [];
  let currentPlayer = 'Red';
  let gameOver = false;
  let ply = 0;

  // Configure engine
  if (config.game_mode)    engine.send(`setoption name gamemode value ${config.game_mode}`);
  if (config.eval_profile) engine.send(`setoption name evalprofile value ${config.eval_profile}`);

  while (!gameOver && ply < (config.stop_at?.max_ply ?? 200)) {
    // Send position
    const posCmd = moveList.length === 0
      ? 'position startpos'
      : `position startpos moves ${moveList.join(' ')}`;
    engine.send(posCmd);
    engine.send(`go depth ${config.depth}`);

    // Collect all output until bestmove or gameover-without-bestmove
    const rawLines = [];
    let bestmove = null;
    let lastSearch = null;

    while (true) {
      const line = await engine.readLine();
      rawLines.push(line);
      const p = parseLine(line);

      if (p.type === 'eliminated') {
        record.eliminations.push({ player: p.color, reason: p.reason, at_ply: ply });
      } else if (p.type === 'gameover') {
        record.winner = p.winner;
        gameOver = true;
      } else if (p.type === 'nextturn') {
        currentPlayer = p.player;
      } else if (p.type === 'search_info') {
        lastSearch = p;
      } else if (p.type === 'bestmove') {
        bestmove = p.move;
        break;
      }
    }

    if (bestmove) {
      record.plies.push({
        ply,
        player: currentPlayer,
        move: bestmove,
        eval: lastSearch?.score_cp ?? null,
        depth: lastSearch?.depth ?? null,
        nodes: lastSearch?.nodes ?? null,
        nps: lastSearch?.nps ?? null,
        time_ms: lastSearch?.time_ms ?? null,
        values: lastSearch ? [lastSearch.v1, lastSearch.v2, lastSearch.v3, lastSearch.v4] : null,
        pv: lastSearch?.pv ?? null,
        raw_lines: rawLines,
      });

      moveList.push(bestmove);
      ply++;

      // Advance to next non-eliminated player
      const eliminated = new Set(record.eliminations.map((e) => e.player));
      let next = PLAYERS[(PLAYERS.indexOf(currentPlayer) + 1) % 4];
      for (let i = 0; i < 3; i++) {
        if (!eliminated.has(next)) break;
        next = PLAYERS[(PLAYERS.indexOf(next) + 1) % 4];
      }
      currentPlayer = next;
    }

    if (gameOver) break;
  }

  record.total_ply = ply;
  return record;
}

// ---------------------------------------------------------------------------
// Generate a plain-text summary report
// ---------------------------------------------------------------------------
function summary(games) {
  const lines = [];
  lines.push('# Observer Report');
  lines.push(`Generated: ${new Date().toISOString()}`);
  lines.push(`Games: ${games.length} | Depth: ${config.depth} | Mode: ${config.game_mode} | Profile: ${config.eval_profile}`);
  lines.push('');

  // Wins
  const wins = {};
  for (const g of games) { wins[g.winner ?? 'none'] = (wins[g.winner ?? 'none'] || 0) + 1; }
  lines.push('## Results');
  for (const [p, c] of Object.entries(wins)) lines.push(`- ${p}: ${c} win(s)`);
  lines.push('');

  // Per game
  for (const g of games) {
    lines.push(`## Game ${g.game}  —  Winner: ${g.winner ?? 'none'}  |  ${g.total_ply} ply`);
    if (g.eliminations.length) {
      lines.push(`Eliminations: ${g.eliminations.map((e) => `${e.player} (${e.reason ?? '?'}, ply ${e.at_ply})`).join(', ')}`);
    }
    lines.push('');

    // Per-player opening moves (first 10)
    for (const p of PLAYERS) {
      const moves = g.plies.filter((m) => m.player === p);
      if (!moves.length) continue;
      const opening = moves.slice(0, 10);
      const moveStr = opening.map((m) => m.move).join(' ');
      const evalStr = opening.map((m) => m.eval ?? '?').join(', ');
      lines.push(`**${p}** (${moves.length} moves): \`${moveStr}\``);
      lines.push(`  evals: ${evalStr}`);
    }
    lines.push('');

    // Full move log (compact)
    lines.push('<details><summary>Full move log</summary>');
    lines.push('');
    for (const m of g.plies) {
      lines.push(`${m.ply}. ${m.player}: ${m.move} (${m.eval ?? '?'}cp, d${m.depth ?? '?'}, ${m.nodes ?? '?'}n)`);
    }
    lines.push('</details>');
    lines.push('');
  }

  return lines.join('\n');
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  const outputDir = resolve(__dirname, config.output_dir);
  if (!existsSync(outputDir)) mkdirSync(outputDir, { recursive: true });

  const enginePath = resolve(__dirname, config.engine);
  console.log(`Engine: ${enginePath}`);
  console.log(`Games: ${config.games} | Depth: ${config.depth} | Mode: ${config.game_mode} | Profile: ${config.eval_profile}`);
  console.log('');

  const engine = new Engine(enginePath);
  await engine.handshake();
  console.log('Engine ready.\n');

  // Enable protocol logging
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  const logFileName = `${config.game_mode}_${config.eval_profile}_d${config.depth}_${config.games}games_${timestamp}.log`;
  const logFilePath = join(outputDir, logFileName);
  engine.send(`setoption name LogFile value ${logFilePath}`);
  console.log(`Protocol log: ${logFilePath}\n`);

  const games = [];
  for (let i = 1; i <= config.games; i++) {
    process.stdout.write(`Game ${i}/${config.games} ... `);
    const record = await playGame(engine, i);
    games.push(record);
    console.log(`${record.total_ply} ply, winner: ${record.winner ?? 'none'}`);

    // Write per-game JSON
    writeFileSync(join(outputDir, `game_${String(i).padStart(3, '0')}.json`), JSON.stringify(record, null, 2));
  }

  // Disable protocol logging before quit
  engine.send('setoption name LogFile value none');
  engine.close();

  // Write combined data + summary
  writeFileSync(join(outputDir, 'all_games.json'), JSON.stringify(games, null, 2));
  const summaryPath = join(outputDir, 'summary.md');
  writeFileSync(summaryPath, summary(games));

  console.log(`\nDone. Reports in ${outputDir}`);
  console.log(`Summary: ${summaryPath}`);
}

main().catch((err) => {
  console.error('Observer error:', err);
  process.exit(1);
});
