// Hybrid Controller — Stage 11
//
// Orchestrates BRS (Phase 1: tactical filter) then MCTS (Phase 2: strategic
// search). BRS knowledge (history table + prior policy) is passed to MCTS
// for warm-start via Progressive History and Gumbel-informed priors.
//
// Implements the frozen `Searcher` trait so it can be used as a drop-in
// replacement for BrsSearcher in the protocol handler.

use std::time::Instant;

use crate::eval::{BootstrapEvaluator, EvalProfile};
use crate::gamestate::GameState;
use crate::movegen::is_in_check;
use crate::movegen::Move;
use crate::protocol::EngineOptions;

use super::brs::BrsSearcher;
use super::mcts::MctsSearcher;
use super::time_manager::{TimeContext, TimeManager};
use super::{SearchBudget, SearchResult, Searcher};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Moves within this many centipawns of the best BRS score survive for MCTS.
const TACTICAL_MARGIN: i16 = 150;

/// Softmax temperature for converting BRS scores to MCTS priors (ADR-016).
const PRIOR_TEMPERATURE: f64 = 50.0;

/// Minimum number of surviving moves passed to MCTS.
const MIN_SURVIVORS: usize = 2;

/// If total time budget is below this, skip MCTS entirely.
const TIME_PRESSURE_MS: u64 = 100;

/// Approximate MCTS simulation throughput (release build) for time-to-sims
/// conversion. From Stage 10 performance data: ~8000 sims/sec.
const MCTS_SIMS_PER_SEC: u64 = 8_000;

/// Capture ratio threshold for classifying positions as tactical.
/// Positions with >= 30% capture moves are considered tactical.
const TACTICAL_CAPTURE_RATIO: f64 = 0.30;

/// BRS time fraction for tactical positions (many captures/checks).
const BRS_FRACTION_TACTICAL: f64 = 0.30;

/// BRS time fraction for quiet positions (few captures).
const BRS_FRACTION_QUIET: f64 = 0.10;

/// If BRS score spread among survivors is below this, MCTS gets extra budget.
const BRS_SPREAD_THRESHOLD: i16 = 50;

/// Maximum BRS depth when budget is depth-only (no time constraint).
/// Depth 8 takes ~120ms — well within 15% of a 5s budget (750ms).
const BRS_MAX_DEPTH: u8 = 8;

/// Default MCTS sims when budget is depth-only.
const MCTS_DEFAULT_SIMS: u64 = 2_000;

// ---------------------------------------------------------------------------
// Position classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionType {
    /// Many captures, checks, threats — give BRS more time.
    Tactical,
    /// Calm position — lean on MCTS.
    Quiet,
    /// Few pieces on board — need deeper search.
    Endgame,
    /// Exactly 1 legal move — instant return.
    Forced,
}

/// Classify a position using multiple signals.
///
/// Priority: Forced > Endgame > Tactical > Quiet.
fn classify_position(legal_moves: &[Move], position: &GameState) -> PositionType {
    // 1. Forced: exactly 0 or 1 legal move
    if legal_moves.len() <= 1 {
        return PositionType::Forced;
    }

    // 2. Endgame: few total pieces on board (from 64 starting)
    let total_pieces = position.board().piece_count();
    if total_pieces <= 16 {
        return PositionType::Endgame;
    }

    // 3. Tactical: in check
    if is_in_check(position.current_player(), position.board()) {
        return PositionType::Tactical;
    }

    // 4. Tactical: high capture ratio
    let captures = legal_moves.iter().filter(|m| m.is_capture()).count();
    let ratio = captures as f64 / legal_moves.len() as f64;
    if ratio >= TACTICAL_CAPTURE_RATIO {
        return PositionType::Tactical;
    }

    // 5. Tactical: few legal moves (constrained/near-forced)
    if legal_moves.len() < 5 {
        return PositionType::Tactical;
    }

    PositionType::Quiet
}

// ---------------------------------------------------------------------------
// Softmax for prior computation
// ---------------------------------------------------------------------------

/// Softmax with max-subtraction for numerical stability.
fn softmax_f64(scores: &[f64], temperature: f64) -> Vec<f32> {
    if scores.is_empty() {
        return Vec::new();
    }
    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = scores
        .iter()
        .map(|&s| ((s - max_score) / temperature).exp())
        .collect();
    let sum: f64 = exps.iter().sum();
    if sum == 0.0 {
        let uniform = 1.0 / scores.len() as f32;
        return vec![uniform; scores.len()];
    }
    exps.iter().map(|&e| (e / sum) as f32).collect()
}

// ---------------------------------------------------------------------------
// HybridController
// ---------------------------------------------------------------------------

/// Two-phase search controller: BRS tactical filter -> MCTS strategic search.
///
/// Phase 1: BRS runs at reduced budget to score all root moves.
/// Moves within `TACTICAL_MARGIN` of the best score survive.
///
/// Phase 2: MCTS runs on remaining budget with BRS-informed priors
/// and progressive history warm-start from BRS's history table.
pub struct HybridController {
    brs: BrsSearcher,
    mcts: MctsSearcher,
    info_cb: Option<Box<dyn FnMut(String)>>,
    /// Time context set by the protocol layer before each search (Stage 13).
    /// Consumed (taken) during search() via `.take()`.
    time_context: Option<TimeContext>,
    /// Score from the previous search, used for near-elimination detection.
    last_score: Option<i16>,
    // --- Tunable parameter overrides (Stage 13) ---
    tactical_margin_override: Option<i16>,
    brs_frac_tactical_override: Option<f64>,
    brs_frac_quiet_override: Option<f64>,
    mcts_sims_override: Option<u64>,
    brs_max_depth_override: Option<u8>,
}

impl HybridController {
    /// Create a new HybridController. Creates both sub-searchers internally.
    pub fn new(profile: EvalProfile) -> Self {
        let brs = BrsSearcher::new(Box::new(BootstrapEvaluator::new(profile)));
        let mcts = MctsSearcher::new(Box::new(BootstrapEvaluator::new(profile)));
        Self {
            brs,
            mcts,
            info_cb: None,
            time_context: None,
            last_score: None,
            tactical_margin_override: None,
            brs_frac_tactical_override: None,
            brs_frac_quiet_override: None,
            mcts_sims_override: None,
            brs_max_depth_override: None,
        }
    }

    /// Replace the info callback (called before each search by the protocol).
    pub fn set_info_callback(&mut self, cb: Box<dyn FnMut(String)>) {
        self.info_cb = Some(cb);
    }

    /// Set the time context for the next search. Called by protocol handler
    /// before `search()` when time controls are active.
    pub fn set_time_context(&mut self, ctx: TimeContext) {
        self.time_context = Some(ctx);
    }

    /// Apply tunable parameter overrides from engine options.
    pub fn apply_options(&mut self, opts: &EngineOptions) {
        self.tactical_margin_override = opts.tactical_margin;
        self.brs_frac_tactical_override = opts.brs_fraction_tactical;
        self.brs_frac_quiet_override = opts.brs_fraction_quiet;
        self.mcts_sims_override = opts.mcts_default_sims;
        self.brs_max_depth_override = opts.brs_max_depth;
    }

    /// Effective tactical margin (override or default constant).
    fn effective_tactical_margin(&self) -> i16 {
        self.tactical_margin_override.unwrap_or(TACTICAL_MARGIN)
    }

    /// Effective BRS max depth (override or default constant).
    fn effective_brs_max_depth(&self) -> u8 {
        self.brs_max_depth_override.unwrap_or(BRS_MAX_DEPTH)
    }

    /// Effective MCTS default sims (override or default constant).
    fn effective_mcts_default_sims(&self) -> u64 {
        self.mcts_sims_override.unwrap_or(MCTS_DEFAULT_SIMS)
    }

    /// Effective BRS fraction for a position type.
    fn effective_brs_fraction(&self, pos_type: PositionType) -> f64 {
        match pos_type {
            PositionType::Tactical | PositionType::Endgame => {
                self.brs_frac_tactical_override
                    .unwrap_or(BRS_FRACTION_TACTICAL)
            }
            PositionType::Quiet => {
                self.brs_frac_quiet_override.unwrap_or(BRS_FRACTION_QUIET)
            }
            PositionType::Forced => 0.0,
        }
    }

    /// Filter root moves to those within the tactical margin of the best score.
    /// Always keeps at least MIN_SURVIVORS moves.
    fn filter_survivors(&self, root_scores: &[(Move, i16)]) -> Vec<(Move, i16)> {
        if root_scores.is_empty() {
            return Vec::new();
        }

        let margin = self.effective_tactical_margin();
        let best_score = root_scores.iter().map(|(_, s)| *s).max().unwrap();
        let threshold = best_score.saturating_sub(margin);

        let mut survivors: Vec<(Move, i16)> = root_scores
            .iter()
            .filter(|(_, s)| *s >= threshold)
            .copied()
            .collect();

        // Ensure minimum survivor count.
        if survivors.len() < MIN_SURVIVORS && root_scores.len() >= MIN_SURVIVORS {
            let mut sorted: Vec<(Move, i16)> = root_scores.to_vec();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            survivors = sorted.into_iter().take(MIN_SURVIVORS).collect();
        } else if survivors.is_empty() {
            // Fallback: take all if none survive (shouldn't happen).
            survivors = root_scores.to_vec();
        }

        survivors
    }

    /// Compute prior policy for MCTS. Returns a Vec aligned to legal_moves:
    /// surviving moves get softmax-computed priors, non-survivors get 0.0.
    fn compute_hybrid_priors(
        legal_moves: &[Move],
        survivors: &[(Move, i16)],
    ) -> Vec<f32> {
        // Build a set of surviving moves and their scores.
        let survivor_scores: Vec<f64> = survivors.iter().map(|(_, s)| *s as f64).collect();
        let survivor_priors = softmax_f64(&survivor_scores, PRIOR_TEMPERATURE);

        // Map to full legal_moves length.
        let mut priors = vec![0.0f32; legal_moves.len()];
        for (si, (smv, _)) in survivors.iter().enumerate() {
            // Find this surviving move in the legal_moves list.
            if let Some(li) = legal_moves.iter().position(|lm| *lm == *smv) {
                priors[li] = survivor_priors[si];
            }
        }
        priors
    }

    /// Allocate BRS budget from the total budget.
    fn allocate_brs_budget(&self, budget: &SearchBudget, pos_type: PositionType) -> SearchBudget {
        let frac = self.effective_brs_fraction(pos_type);
        let max_depth = self.effective_brs_max_depth();
        SearchBudget {
            max_depth: budget.max_depth.map(|d| d.min(max_depth)),
            max_nodes: budget.max_nodes.map(|n| ((n as f64 * frac) as u64).max(1)),
            max_time_ms: budget.max_time_ms.map(|t| ((t as f64 * frac) as u64).max(1)),
        }
    }

    /// Allocate MCTS budget from remaining time/nodes after BRS.
    fn allocate_mcts_budget(
        &self,
        budget: &SearchBudget,
        brs_elapsed_ms: u64,
        brs_nodes: u64,
        spread_is_tight: bool,
    ) -> SearchBudget {
        // Extra fraction to shift to MCTS if BRS can't distinguish moves.
        let extra = if spread_is_tight { 0.10 } else { 0.0 };

        if let Some(total_ms) = budget.max_time_ms {
            let remaining_ms = total_ms.saturating_sub(brs_elapsed_ms);
            // Convert remaining time to sim count.
            let sims = (remaining_ms * MCTS_SIMS_PER_SEC / 1000).max(2);
            SearchBudget {
                max_depth: None,
                max_nodes: Some(sims),
                max_time_ms: Some(remaining_ms),
            }
        } else if let Some(total_nodes) = budget.max_nodes {
            let remaining = total_nodes.saturating_sub(brs_nodes);
            let bonus = (total_nodes as f64 * extra) as u64;
            SearchBudget {
                max_depth: None,
                max_nodes: Some(remaining.saturating_add(bonus).max(2)),
                max_time_ms: None,
            }
        } else {
            // Depth-only: give MCTS a default sim budget.
            let default_sims = self.effective_mcts_default_sims();
            SearchBudget {
                max_depth: None,
                max_nodes: Some(default_sims),
                max_time_ms: None,
            }
        }
    }
}

impl Searcher for HybridController {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult {
        let start = Instant::now();
        let mut pos_clone = position.clone();
        let legal_moves = pos_clone.legal_moves();

        // Edge case: no legal moves (shouldn't be called, but be safe).
        if legal_moves.is_empty() {
            panic!("HybridController::search called with no legal moves");
        }

        // Edge case: single legal move — return immediately (Stage 13: forced move).
        if legal_moves.len() == 1 {
            self.time_context = None; // consume any pending time context
            self.last_score = Some(0);
            return SearchResult {
                best_move: legal_moves[0],
                score: 0,
                depth: 0,
                nodes: 0,
                pv: vec![legal_moves[0]],
            };
        }

        // Classify position (enriched: Tactical/Quiet/Endgame/Forced — Stage 13).
        let pos_type = classify_position(&legal_moves, position);
        let in_check = is_in_check(position.current_player(), position.board());

        // Stage 13: Time allocation via TimeManager when time controls are active.
        let effective_budget = if let Some(ctx) = self.time_context.take() {
            let is_tactical =
                pos_type == PositionType::Tactical || pos_type == PositionType::Endgame;
            let allocated_ms = TimeManager::allocate(
                ctx.remaining_ms,
                ctx.increment_ms,
                ctx.ply,
                ctx.movestogo,
                legal_moves.len(),
                is_tactical,
                in_check,
                self.last_score,
            );

            // Emit time allocation info.
            if let Some(ref mut cb) = self.info_cb {
                cb(format!(
                    "info string time_alloc total={}ms type={:?} remaining={}ms inc={}ms",
                    allocated_ms, pos_type, ctx.remaining_ms, ctx.increment_ms,
                ));
            }

            SearchBudget {
                max_depth: budget.max_depth,
                max_nodes: budget.max_nodes,
                max_time_ms: Some(allocated_ms),
            }
        } else {
            budget // no time context → use budget as-is
        };

        // Edge case: time pressure — BRS only, skip MCTS.
        if let Some(t) = effective_budget.max_time_ms {
            if t < TIME_PRESSURE_MS {
                // Give BRS the full budget.
                if let Some(cb) = self.info_cb.take() {
                    self.brs.set_info_callback(cb);
                }
                let result = self.brs.search(position, effective_budget);
                self.info_cb = self.brs.take_info_callback();
                self.last_score = Some(result.score);
                return result;
            }
        }

        // ---------------------------------------------------------------
        // Phase 1: BRS tactical filter
        // ---------------------------------------------------------------
        let brs_budget = self.allocate_brs_budget(&effective_budget, pos_type);

        if let Some(cb) = self.info_cb.take() {
            self.brs.set_info_callback(cb);
        }
        let brs_result = self.brs.search(position, brs_budget);
        self.info_cb = self.brs.take_info_callback();

        let brs_elapsed_ms = start.elapsed().as_millis() as u64;

        // Extract BRS knowledge.
        let root_scores = self.brs.root_move_scores();
        let history = self.brs.history_table();

        // Survivor filter (uses effective tactical margin).
        let survivors = if let Some(scores) = root_scores {
            self.filter_survivors(scores)
        } else {
            // No root scores (search aborted before completing depth 1).
            // Fall through to MCTS with all moves.
            legal_moves.iter().map(|&m| (m, 0i16)).collect()
        };

        // If only one survivor, return immediately.
        if survivors.len() == 1 {
            self.last_score = Some(survivors[0].1);
            return SearchResult {
                best_move: survivors[0].0,
                score: survivors[0].1,
                depth: brs_result.depth,
                nodes: brs_result.nodes,
                pv: vec![survivors[0].0],
            };
        }

        // Emit phase transition info.
        let best_survivor_score = survivors.iter().map(|(_, s)| *s).max().unwrap_or(0);
        let worst_survivor_score = survivors.iter().map(|(_, s)| *s).min().unwrap_or(0);
        let spread_is_tight =
            best_survivor_score.saturating_sub(worst_survivor_score) < BRS_SPREAD_THRESHOLD;

        let margin = self.effective_tactical_margin();
        if let Some(ref mut cb) = self.info_cb {
            cb(format!(
                "info string hybrid phase1 done survivors {} threshold {}cp spread {}cp time {}ms",
                survivors.len(),
                margin,
                best_survivor_score - worst_survivor_score,
                brs_elapsed_ms,
            ));
        }

        // ---------------------------------------------------------------
        // Handoff: compute priors and pass BRS knowledge to MCTS
        // ---------------------------------------------------------------
        let priors = Self::compute_hybrid_priors(&legal_moves, &survivors);
        self.mcts.set_prior_policy(&priors);

        if let Some(h) = history {
            self.mcts.set_history_table(h);
        }

        // ---------------------------------------------------------------
        // Phase 2: MCTS strategic search
        // ---------------------------------------------------------------
        let mcts_budget = self.allocate_mcts_budget(
            &effective_budget,
            brs_elapsed_ms,
            brs_result.nodes,
            spread_is_tight,
        );

        if let Some(cb) = self.info_cb.take() {
            self.mcts.set_info_callback(cb);
        }
        let mcts_result = self.mcts.search(position, mcts_budget);
        self.info_cb = self.mcts.take_info_callback();

        // Combine results: use MCTS's best move but include BRS nodes in total.
        let result = SearchResult {
            best_move: mcts_result.best_move,
            score: mcts_result.score,
            depth: mcts_result.depth,
            nodes: brs_result.nodes + mcts_result.nodes,
            pv: mcts_result.pv,
        };
        self.last_score = Some(result.score);
        result
    }
}
