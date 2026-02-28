// elo.mjs — Elo difference calculation for 4-player FFA engine matches
//
// Reduces 4-player FFA results to pairwise A vs B comparison:
//   - Game winner belongs to engine A's seats → A win (1.0)
//   - Game winner belongs to engine B's seats → B win (0.0)
//   - No winner (ply limit) → draw (0.5)
//
// Usage:
//   import { calculateElo, expectedScore } from './elo.mjs';
//   const result = calculateElo([{ winner: 'A' }, { winner: 'B' }, { winner: 'draw' }]);

/**
 * Expected score given an Elo difference.
 * @param {number} eloDiff - Elo difference (A - B).
 * @returns {number} Expected score in [0, 1].
 */
export function expectedScore(eloDiff) {
  return 1 / (1 + Math.pow(10, -eloDiff / 400));
}

/**
 * Convert a win probability to an Elo difference.
 * @param {number} p - Win probability in (0, 1).
 * @returns {number} Elo difference.
 */
export function scoreToElo(p) {
  if (p <= 0) return -Infinity;
  if (p >= 1) return Infinity;
  return -400 * Math.log10(1 / p - 1);
}

/**
 * Calculate Elo difference and confidence interval from match results.
 *
 * @param {Array<{winner: 'A' | 'B' | 'draw'}>} results - Per-game outcomes.
 * @returns {{
 *   elo_diff: number,
 *   ci_low: number,
 *   ci_high: number,
 *   wins: number,
 *   losses: number,
 *   draws: number,
 *   n: number,
 *   actual_score: number
 * }}
 */
export function calculateElo(results) {
  const n = results.length;
  let wins = 0;
  let losses = 0;
  let draws = 0;

  for (const r of results) {
    if (r.winner === 'A') wins++;
    else if (r.winner === 'B') losses++;
    else draws++;
  }

  const actual_score = (wins + 0.5 * draws) / n;
  const elo_diff = scoreToElo(actual_score);

  // 95% confidence interval via normal approximation
  const variance = (actual_score * (1 - actual_score)) / n;
  const se = Math.sqrt(variance);
  const ci_low_p = Math.max(actual_score - 1.96 * se, 0.001);
  const ci_high_p = Math.min(actual_score + 1.96 * se, 0.999);

  const ci_low = scoreToElo(ci_low_p);
  const ci_high = scoreToElo(ci_high_p);

  return { elo_diff, ci_low, ci_high, wins, losses, draws, n, actual_score };
}

/**
 * Format Elo result as a human-readable string.
 * @param {{elo_diff: number, ci_low: number, ci_high: number, wins: number, losses: number, draws: number, n: number, actual_score: number}} result
 * @returns {string}
 */
export function formatElo(result) {
  const { elo_diff, ci_low, ci_high, wins, losses, draws, n, actual_score } = result;
  const lines = [
    `Elo difference: ${isFinite(elo_diff) ? elo_diff.toFixed(1) : (elo_diff > 0 ? '+Inf' : '-Inf')}`,
    `95% CI: [${isFinite(ci_low) ? ci_low.toFixed(1) : '-Inf'}, ${isFinite(ci_high) ? ci_high.toFixed(1) : '+Inf'}]`,
    `Score: ${actual_score.toFixed(3)} (W:${wins} L:${losses} D:${draws} / ${n} games)`,
  ];
  if (n < 10) lines.push('WARNING: sample size < 10, results may be unreliable');
  if (!isFinite(elo_diff)) lines.push('WARNING: 0% or 100% win rate, Elo is unbounded');
  return lines.join('\n');
}
