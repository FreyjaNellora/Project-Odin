// sprt.mjs — Sequential Probability Ratio Test for engine match early stopping
//
// Implements Wald's SPRT with Bernoulli likelihood model.
//   H0: Elo difference <= elo0 (new engine is NOT better)
//   H1: Elo difference >= elo1 (new engine IS better by at least elo1)
//
// Usage:
//   import { sprtInit, sprtUpdate, sprtStatus } from './sprt.mjs';
//   const state = sprtInit({ elo0: 0, elo1: 5, alpha: 0.05, beta: 0.05 });
//   const result = sprtUpdate(state, 'A');  // A won
//   if (result.decision !== 'continue') { /* stop match */ }

import { expectedScore } from './elo.mjs';

/**
 * Initialize SPRT state.
 *
 * @param {{elo0: number, elo1: number, alpha: number, beta: number}} config
 * @returns {{
 *   p0: number, p1: number,
 *   lowerBound: number, upperBound: number,
 *   llr: number, wins: number, losses: number, draws: number
 * }}
 */
export function sprtInit({ elo0 = 0, elo1 = 5, alpha = 0.05, beta = 0.05 } = {}) {
  const p0 = expectedScore(elo0);
  const p1 = expectedScore(elo1);

  // Wald's sequential boundaries (natural log)
  const lowerBound = Math.log(beta / (1 - alpha));
  const upperBound = Math.log((1 - beta) / alpha);

  return {
    p0,
    p1,
    lowerBound,
    upperBound,
    llr: 0,
    wins: 0,
    losses: 0,
    draws: 0,
    elo0,
    elo1,
    alpha,
    beta,
  };
}

/**
 * Update SPRT state with a game result.
 *
 * @param {Object} state - SPRT state from sprtInit.
 * @param {'A' | 'B' | 'draw'} result - Game result (A win, B win, or draw).
 * @returns {{decision: 'continue' | 'accept_h1' | 'accept_h0', llr: number, games: number}}
 */
export function sprtUpdate(state, result) {
  // Convert result to score: A win = 1.0, B win = 0.0, draw = 0.5
  let s;
  if (result === 'A') {
    state.wins++;
    s = 1.0;
  } else if (result === 'B') {
    state.losses++;
    s = 0.0;
  } else {
    state.draws++;
    s = 0.5;
  }

  // Bernoulli LLR increment:
  //   LLR += s * ln(p1/p0) + (1-s) * ln((1-p1)/(1-p0))
  const logRatio1 = Math.log(state.p1 / state.p0);
  const logRatio0 = Math.log((1 - state.p1) / (1 - state.p0));
  state.llr += s * logRatio1 + (1 - s) * logRatio0;

  const games = state.wins + state.losses + state.draws;

  let decision = 'continue';
  if (state.llr >= state.upperBound) {
    decision = 'accept_h1'; // New engine IS better
  } else if (state.llr <= state.lowerBound) {
    decision = 'accept_h0'; // New engine is NOT better
  }

  return { decision, llr: state.llr, games };
}

/**
 * Get human-readable SPRT status.
 *
 * @param {Object} state - SPRT state.
 * @returns {string}
 */
export function sprtStatus(state) {
  const games = state.wins + state.losses + state.draws;
  const lines = [
    `SPRT(${state.elo0}, ${state.elo1}): LLR = ${state.llr.toFixed(3)} [${state.lowerBound.toFixed(3)}, ${state.upperBound.toFixed(3)}]`,
    `Games: ${games} (W:${state.wins} L:${state.losses} D:${state.draws})`,
  ];

  if (state.llr >= state.upperBound) {
    lines.push(`Result: H1 accepted — new engine is better (Elo >= ${state.elo1})`);
  } else if (state.llr <= state.lowerBound) {
    lines.push(`Result: H0 accepted — no improvement detected (Elo <= ${state.elo0})`);
  } else {
    const progress = ((state.llr - state.lowerBound) / (state.upperBound - state.lowerBound) * 100).toFixed(0);
    lines.push(`Result: inconclusive (${progress}% toward H1)`);
  }

  return lines.join('\n');
}
