// MCTS Strategic Search — Stage 10
//
// Standalone Gumbel MCTS searcher implementing the frozen `Searcher` trait.
// Uses Gumbel-Top-k + Sequential Halving at the root, PUCT tree policy
// for non-root selection, 4-player MaxN backpropagation, and progressive
// widening. Not integrated with BRS — that's Stage 11.

use std::sync::Arc;
use std::time::Instant;

use crate::board::{Board, PieceStatus, Player};
use crate::eval::nnue::accumulator::AccumulatorStack;
use crate::eval::nnue::{forward_pass, weights::NnueWeights};
use crate::eval::{Evaluator, PIECE_EVAL_VALUES};
use crate::gamestate::{GameState, PlayerStatus};
use crate::movegen::Move;
use crate::util::SplitMix64;

use super::{SearchBudget, SearchResult, Searcher};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MCTS_TOP_K: usize = 16;
const MCTS_C_PRIOR: f32 = 1.5;
const MCTS_PRIOR_TEMPERATURE: f32 = 50.0;
const MCTS_PW_W: f64 = 2.0;
const MCTS_PW_B: f64 = 0.5;
const MCTS_PH_WEIGHT: f32 = 0.1;
const MCTS_TIME_CHECK_INTERVAL: u64 = 64;
const MCTS_SCORE_CAP: i16 = 9_999;

const TOTAL_SQUARES: usize = 196;
const PIECE_TYPE_COUNT: usize = 7;
const PLAYER_COUNT: usize = 4;

// ---------------------------------------------------------------------------
// Step 1: MctsNode
// ---------------------------------------------------------------------------

pub(crate) struct MctsNode {
    pub move_to_here: Option<Move>,
    pub player_to_move: Player,
    pub visit_count: u32,
    pub value_sum: [f64; 4],
    pub prior: f32,
    pub gumbel: f32,
    pub children: Vec<MctsNode>,
    pub is_expanded: bool,
    pub is_terminal: bool,
    /// Total legal moves at this position (for progressive widening reference).
    pub total_children: u16,
}

impl MctsNode {
    pub fn new_root(player_to_move: Player) -> Self {
        Self {
            move_to_here: None,
            player_to_move,
            visit_count: 0,
            value_sum: [0.0; 4],
            prior: 1.0,
            gumbel: 0.0,
            children: Vec::new(),
            is_expanded: false,
            is_terminal: false,
            total_children: 0,
        }
    }

    pub fn new_child(mv: Move, player_to_move: Player, prior: f32) -> Self {
        Self {
            move_to_here: Some(mv),
            player_to_move,
            visit_count: 0,
            value_sum: [0.0; 4],
            prior,
            gumbel: 0.0,
            children: Vec::new(),
            is_expanded: false,
            is_terminal: false,
            total_children: 0,
        }
    }

    /// Average value for the given player. Returns 0.0 if unvisited.
    pub fn q_value(&self, player_idx: usize) -> f64 {
        if self.visit_count == 0 {
            return 0.0;
        }
        self.value_sum[player_idx] / self.visit_count as f64
    }
}

// ---------------------------------------------------------------------------
// Step 2: Prior policy computation
// ---------------------------------------------------------------------------

/// Softmax with max-subtraction for numerical stability.
fn softmax(scores: &[f64], temperature: f64) -> Vec<f32> {
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
        // Uniform fallback
        let uniform = 1.0 / scores.len() as f32;
        return vec![uniform; scores.len()];
    }
    exps.iter().map(|&e| (e / sum) as f32).collect()
}

/// Compute move priors using softmax over MVV-LVA ordering scores.
/// Pre-NNUE: captures scored by victim - attacker/10 + 100; quiets at 10.
/// Dead (DKW) pieces get minimal capture value (1.0) instead of full material.
pub(crate) fn compute_priors(moves: &[Move], temperature: f32, board: &Board) -> Vec<f32> {
    if moves.is_empty() {
        return Vec::new();
    }
    let scores: Vec<f64> = moves
        .iter()
        .map(|mv| {
            if mv.is_capture() {
                let is_dead = board
                    .piece_at(mv.to_sq())
                    .map(|p| p.status != PieceStatus::Alive)
                    .unwrap_or(false);
                let victim_val = if is_dead {
                    1.0
                } else {
                    mv.captured()
                        .map(|pt| PIECE_EVAL_VALUES[pt.index()] as f64)
                        .unwrap_or(0.0)
                };
                let attacker_val = PIECE_EVAL_VALUES[mv.piece_type().index()] as f64;
                victim_val - attacker_val / 10.0 + 100.0
            } else {
                10.0
            }
        })
        .collect();
    softmax(&scores, temperature as f64)
}

// ---------------------------------------------------------------------------
// Step 3: Gumbel noise sampling + Top-k selection
// ---------------------------------------------------------------------------

/// Sample from Gumbel(0, 1) distribution: -ln(-ln(U)) where U ~ Uniform(0,1).
fn sample_gumbel(rng: &mut SplitMix64) -> f32 {
    let u = rng.next_f64();
    -(-u.ln()).ln() as f32
}

/// Select top-k children by g(a) + log(pi(a)). Returns indices into children.
pub(crate) fn top_k_selection(children: &[MctsNode], k: usize) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..children.len()).collect();
    indices.sort_by(|&a, &b| {
        let score_a =
            children[a].gumbel + (children[a].prior.max(1e-10) as f64).ln() as f32;
        let score_b =
            children[b].gumbel + (children[b].prior.max(1e-10) as f64).ln() as f32;
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    indices.truncate(k);
    indices
}

// ---------------------------------------------------------------------------
// Step 5: Non-root tree policy (PUCT)
// ---------------------------------------------------------------------------

/// Progressive widening limit: max selectable children = floor(W * max(1,N)^B).
fn pw_limit(visit_count: u32, pw_w: f64, pw_b: f64) -> usize {
    let n = visit_count.max(1) as f64;
    let limit = (pw_w * n.powf(pw_b)).floor() as usize;
    limit.max(2)
}

/// PUCT score for a child node from the parent's player perspective.
fn puct_score(
    child: &MctsNode,
    parent_player_idx: usize,
    c_prior: f32,
    history_table: Option<&HistoryTable>,
    ph_weight: f32,
) -> f64 {
    if child.visit_count == 0 {
        return child.prior as f64 + 1e6;
    }
    let q = child.value_sum[parent_player_idx] / child.visit_count as f64;
    let exploration = c_prior as f64 * child.prior as f64 / (1.0 + child.visit_count as f64);
    let ph = if let Some(history) = history_table {
        if let Some(mv) = child.move_to_here {
            let h = history[parent_player_idx][mv.piece_type().index()]
                [mv.to_sq() as usize] as f64;
            ph_weight as f64 * h / (child.visit_count as f64 + 1.0)
        } else {
            0.0
        }
    } else {
        0.0
    };
    q + exploration + ph
}

/// Select child index using PUCT, respecting progressive widening limits.
fn select_child_idx(
    node: &MctsNode,
    c_prior: f32,
    pw_w: f64,
    pw_b: f64,
    history_table: Option<&HistoryTable>,
    ph_weight: f32,
) -> usize {
    let parent_player_idx = node.player_to_move.index();
    let selectable = pw_limit(node.visit_count, pw_w, pw_b).min(node.children.len());
    node.children[..selectable]
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let sa = puct_score(a, parent_player_idx, c_prior, history_table, ph_weight);
            let sb = puct_score(b, parent_player_idx, c_prior, history_table, ph_weight);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Step 6: Expansion + leaf evaluation
// ---------------------------------------------------------------------------

/// Expand a leaf node: generate legal moves, compute priors, create children
/// sorted by prior descending. Does NOT apply progressive widening to creation
/// — PW limits which children are selectable in select_child_idx.
fn expand_node(node: &mut MctsNode, gs: &mut GameState, prior_temperature: f32) {
    if gs.is_game_over() {
        node.is_terminal = true;
        node.is_expanded = true;
        return;
    }
    let legal_moves = gs.legal_moves();
    if legal_moves.is_empty() {
        node.is_terminal = true;
        node.is_expanded = true;
        return;
    }
    let priors = compute_priors(&legal_moves, prior_temperature, gs.board());
    node.total_children = legal_moves.len() as u16;

    // Create indexed pairs sorted by prior descending
    let mut indexed: Vec<(usize, f32)> = priors.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for &(orig_idx, prior) in &indexed {
        let mv = legal_moves[orig_idx];
        let mut child_gs = gs.clone();
        child_gs.apply_move(mv);
        let next_player = child_gs.current_player();
        let child = MctsNode::new_child(mv, next_player, prior);
        node.children.push(child);
    }
    node.is_expanded = true;
}

// ---------------------------------------------------------------------------
// Step 7: Backpropagation (4-player MaxN)
// ---------------------------------------------------------------------------

/// Backpropagate leaf values along the selection path.
/// `path` is [target_root_child_idx, child_of_that, child_of_child, ...].
fn backpropagate(root: &mut MctsNode, path: &[usize], leaf_values: [f64; 4]) {
    // Update root
    root.visit_count += 1;
    for (i, &val) in leaf_values.iter().enumerate() {
        root.value_sum[i] += val;
    }
    // Walk down the path, updating each node
    let mut node = &mut *root;
    for &child_idx in path {
        node = &mut node.children[child_idx];
        node.visit_count += 1;
        for (i, &val) in leaf_values.iter().enumerate() {
            node.value_sum[i] += val;
        }
    }
}

// ---------------------------------------------------------------------------
// Step 9: Single simulation
// ---------------------------------------------------------------------------

/// Bundled simulation config to avoid too-many-arguments.
struct SimConfig<'a> {
    c_prior: f32,
    pw_w: f64,
    pw_b: f64,
    history_table: Option<&'a HistoryTable>,
    ph_weight: f32,
    max_depth: u8,
    prior_temperature: f32,
}

/// Run one simulation from a specific root child. Returns depth reached.
fn run_simulation(
    root: &mut MctsNode,
    target_child_idx: usize,
    root_gs: &GameState,
    evaluator: &dyn Evaluator,
    cfg: &SimConfig<'_>,
    mut acc_stack: Option<&mut AccumulatorStack>,
    nnue_weights: Option<&NnueWeights>,
) -> u8 {
    // Build selection path starting with forced root child
    let mut path: Vec<usize> = vec![target_child_idx];

    // Clone and apply the root child's move
    let mut gs = root_gs.clone();
    let child_mv = root.children[target_child_idx]
        .move_to_here
        .expect("root child must have a move");

    // NNUE: push BEFORE apply_move (needs board_before)
    if let (Some(ref mut stack), Some(w)) = (&mut acc_stack, nnue_weights) {
        stack.push(child_mv, gs.board(), w);
    }
    let move_result = gs.apply_move(child_mv);
    // Elimination-aware refresh: apply_move can remove kings from the board
    // that push() didn't account for. Force full refresh on all perspectives.
    if !move_result.eliminations.is_empty() {
        if let Some(ref mut stack) = acc_stack {
            stack.current_mut().needs_refresh = [true; 4];
        }
    }

    let mut depth: u8 = 1;

    // Navigate to the current node via path
    // Selection: descend through expanded, non-terminal nodes
    {
        let mut node_ref = &root.children[target_child_idx];
        while node_ref.is_expanded
            && !node_ref.is_terminal
            && !node_ref.children.is_empty()
            && depth < cfg.max_depth
        {
            let child_idx =
                select_child_idx(node_ref, cfg.c_prior, cfg.pw_w, cfg.pw_b, cfg.history_table, cfg.ph_weight);
            let mv = node_ref.children[child_idx]
                .move_to_here
                .expect("child must have a move");

            // NNUE: push BEFORE apply_move
            if let (Some(ref mut stack), Some(w)) = (&mut acc_stack, nnue_weights) {
                stack.push(mv, gs.board(), w);
            }
            let move_result = gs.apply_move(mv);
            if !move_result.eliminations.is_empty() {
                if let Some(ref mut stack) = acc_stack {
                    stack.current_mut().needs_refresh = [true; 4];
                }
            }

            path.push(child_idx);
            depth += 1;
            node_ref = &node_ref.children[child_idx];
        }
    }

    // Now get mutable access to the leaf for expansion
    let leaf = get_node_mut(root, &path);

    // Expansion: if not expanded and not terminal, expand
    if !leaf.is_expanded && !leaf.is_terminal {
        expand_node(leaf, &mut gs, cfg.prior_temperature);
    }

    // Evaluation: use NNUE if available, otherwise bootstrap
    let leaf_values: [f64; 4] = if let (Some(ref mut stack), Some(weights)) = (&mut acc_stack, nnue_weights) {
        stack.refresh_if_needed(gs.board(), weights);
        let root_player = root_gs.board().side_to_move();
        let (_, mcts_vals) = forward_pass(stack.current(), weights, root_player);
        // Override eliminated players with near-zero values
        let mut result = mcts_vals;
        for &p in &Player::ALL {
            if gs.player_status(p) == PlayerStatus::Eliminated {
                result[p.index()] = 0.001;
            }
        }
        result
    } else {
        evaluator.eval_4vec(&gs)
    };

    // Backpropagation
    backpropagate(root, &path, leaf_values);

    // NNUE: pop all pushes to return acc_stack to root depth
    if let Some(ref mut stack) = acc_stack {
        for _ in 0..depth {
            stack.pop();
        }
    }

    depth
}

/// Navigate the tree mutably using a path of child indices.
fn get_node_mut<'a>(root: &'a mut MctsNode, path: &[usize]) -> &'a mut MctsNode {
    let mut node = root;
    for &idx in path {
        node = &mut node.children[idx];
    }
    node
}

// ---------------------------------------------------------------------------
// Step 10: PV extraction
// ---------------------------------------------------------------------------

/// Extract principal variation starting from a specific root child index,
/// then following most-visited children downward.
fn extract_pv(root: &MctsNode, best_child_idx: usize) -> Vec<Move> {
    let mut pv = Vec::new();
    // Start with the selected best child
    let start = &root.children[best_child_idx];
    if let Some(mv) = start.move_to_here {
        pv.push(mv);
    }
    // Follow most-visited children downward
    let mut node = start;
    while !node.children.is_empty() {
        if let Some(best) = node.children.iter().max_by_key(|c| c.visit_count) {
            if best.visit_count == 0 {
                break;
            }
            if let Some(mv) = best.move_to_here {
                pv.push(mv);
            }
            node = best;
        } else {
            break;
        }
    }
    pv
}

// ---------------------------------------------------------------------------
// Step 11: Temperature selection
// ---------------------------------------------------------------------------

/// Select a root child index using temperature-based sampling.
/// temperature=0.0 → deterministic (most-visited).
/// temperature>0 → sample proportional to N^(1/T).
fn temperature_select(
    root: &MctsNode,
    candidates: &[usize],
    temperature: f64,
    rng: &mut SplitMix64,
) -> usize {
    if temperature == 0.0 || candidates.len() <= 1 {
        return *candidates
            .iter()
            .max_by_key(|&&idx| root.children[idx].visit_count)
            .unwrap_or(&0);
    }
    let inv_temp = 1.0 / temperature;
    let weights: Vec<f64> = candidates
        .iter()
        .map(|&idx| (root.children[idx].visit_count as f64).powf(inv_temp))
        .collect();
    let sum: f64 = weights.iter().sum();
    if sum == 0.0 {
        return candidates[0];
    }
    let mut r = rng.next_f64() * sum;
    for (i, &w) in weights.iter().enumerate() {
        r -= w;
        if r <= 0.0 {
            return candidates[i];
        }
    }
    *candidates.last().unwrap()
}

// ---------------------------------------------------------------------------
// Step 6 helper: Score conversion
// ---------------------------------------------------------------------------

/// Convert a Q-value in [0,1] (sigmoid space) back to centipawns.
/// Inverse sigmoid: cp = K * ln(q / (1-q)), clamped to ±MCTS_SCORE_CAP.
/// K must match SIGMOID_K in eval/mod.rs (currently 4000).
fn q_to_centipawns(q: f64) -> i16 {
    if q <= 0.001 {
        return -MCTS_SCORE_CAP;
    }
    if q >= 0.999 {
        return MCTS_SCORE_CAP;
    }
    let cp = 4000.0 * (q / (1.0 - q)).ln();
    (cp as i16).clamp(-MCTS_SCORE_CAP, MCTS_SCORE_CAP)
}

/// Logistic sigmoid: 1 / (1 + exp(-x)).
fn sigma(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

// ---------------------------------------------------------------------------
// Step 12: Stage 11 stubs — HistoryTable type alias
// ---------------------------------------------------------------------------

/// History table type alias matching BRS format.
/// Indexed by [player][piece_type][to_square].
pub type HistoryTable = [[[i32; TOTAL_SQUARES]; PIECE_TYPE_COUNT]; PLAYER_COUNT];

// ---------------------------------------------------------------------------
// Step 13: MctsSearcher struct + Searcher trait implementation
// ---------------------------------------------------------------------------

/// Standalone Gumbel MCTS searcher.
///
/// Implements the frozen `Searcher` trait. Uses Gumbel-Top-k with Sequential
/// Halving at the root, PUCT tree policy for non-root selection, 4-player
/// MaxN backpropagation, and progressive widening.
pub struct MctsSearcher {
    evaluator: Box<dyn Evaluator>,
    info_cb: Option<Box<dyn FnMut(String)>>,
    rng: SplitMix64,

    // Configuration
    top_k: usize,
    c_prior: f32,
    prior_temperature: f32,
    pw_w: f64,
    pw_b: f64,
    ph_weight: f32,
    temperature: f64,

    // External state for Stage 11 hybrid integration
    history_table: Option<Box<HistoryTable>>,
    external_priors: Option<Vec<f32>>,

    // NNUE (Stage 16)
    acc_stack: Option<AccumulatorStack>,
    nnue_weights: Option<Arc<NnueWeights>>,
}

impl MctsSearcher {
    /// Create a new MCTS searcher with default configuration.
    pub fn new(evaluator: Box<dyn Evaluator>, nnue_weights: Option<Arc<NnueWeights>>) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xDEAD_BEEF);
        let acc_stack = if nnue_weights.is_some() {
            Some(AccumulatorStack::new())
        } else {
            None
        };
        Self {
            evaluator,
            info_cb: None,
            rng: SplitMix64::new(seed),
            top_k: MCTS_TOP_K,
            c_prior: MCTS_C_PRIOR,
            prior_temperature: MCTS_PRIOR_TEMPERATURE,
            pw_w: MCTS_PW_W,
            pw_b: MCTS_PW_B,
            ph_weight: MCTS_PH_WEIGHT,
            temperature: 0.0,
            history_table: None,
            external_priors: None,
            acc_stack,
            nnue_weights,
        }
    }

    /// Create a new MCTS searcher with an info callback.
    pub fn with_info_callback(
        evaluator: Box<dyn Evaluator>,
        nnue_weights: Option<Arc<NnueWeights>>,
        cb: Box<dyn FnMut(String)>,
    ) -> Self {
        let mut searcher = Self::new(evaluator, nnue_weights);
        searcher.info_cb = Some(cb);
        searcher
    }

    /// Replace the info callback (used between searches).
    pub fn set_info_callback(&mut self, cb: Box<dyn FnMut(String)>) {
        self.info_cb = Some(cb);
    }

    /// Create with a fixed seed for deterministic tests.
    pub fn with_seed(evaluator: Box<dyn Evaluator>, nnue_weights: Option<Arc<NnueWeights>>, seed: u64) -> Self {
        let mut searcher = Self::new(evaluator, nnue_weights);
        searcher.rng = SplitMix64::new(seed);
        searcher
    }

    /// Stage 11 stub: provide external priors from a neural network or BRS.
    pub fn set_prior_policy(&mut self, priors: &[f32]) {
        self.external_priors = Some(priors.to_vec());
    }

    /// Stage 11 stub: provide BRS history table for progressive history.
    pub fn set_history_table(&mut self, history: &HistoryTable) {
        self.history_table = Some(Box::new(*history));
    }

    /// Take the info callback out of the searcher (returns ownership).
    /// Used by HybridController to move the callback between sub-searchers.
    pub fn take_info_callback(&mut self) -> Option<Box<dyn FnMut(String)>> {
        self.info_cb.take()
    }
}

impl Searcher for MctsSearcher {
    fn search(&mut self, position: &GameState, budget: SearchBudget) -> SearchResult {
        let start = Instant::now();
        let root_player = position.current_player();
        let root_player_idx = root_player.index();

        // Init NNUE accumulator from root position
        if let (Some(stack), Some(w)) = (&mut self.acc_stack, &self.nnue_weights) {
            stack.init_from_board(position.board(), w);
        }

        // Generate legal moves
        let mut gs_clone = position.clone();
        let legal_moves = gs_clone.legal_moves();

        // Single legal move — return immediately
        if legal_moves.len() <= 1 {
            let mv = legal_moves
                .first()
                .copied()
                .expect("position must have at least one legal move");
            return SearchResult {
                best_move: mv,
                score: 0,
                depth: 0,
                nodes: 0,
                pv: vec![mv],
            };
        }

        // Create root node
        let mut root = MctsNode::new_root(root_player);

        // Expand root fully (no progressive widening at root).
        // Use external priors (from HybridController) if available; else MVV-LVA.
        let priors = if let Some(ext) = self.external_priors.take() {
            debug_assert_eq!(
                ext.len(),
                legal_moves.len(),
                "external_priors length ({}) must match legal_moves length ({})",
                ext.len(),
                legal_moves.len()
            );
            if ext.len() == legal_moves.len() {
                ext
            } else {
                compute_priors(&legal_moves, self.prior_temperature, position.board())
            }
        } else {
            compute_priors(&legal_moves, self.prior_temperature, position.board())
        };
        for (i, &mv) in legal_moves.iter().enumerate() {
            let mut child_gs = position.clone();
            child_gs.apply_move(mv);
            let next_player = child_gs.current_player();
            let mut child = MctsNode::new_child(mv, next_player, priors[i]);
            child.gumbel = sample_gumbel(&mut self.rng);
            root.children.push(child);
        }
        root.is_expanded = true;
        root.total_children = legal_moves.len() as u16;

        // Top-k selection
        let k = self.top_k.min(root.children.len());
        let mut candidates = top_k_selection(&root.children, k);

        // Budget
        let total_sims = budget.max_nodes.unwrap_or(1000);
        let mut total_sims_done: u64 = 0;
        let mut max_depth_reached: u8 = 0;

        // Sequential Halving
        let num_rounds = if candidates.len() > 1 {
            (candidates.len() as f64).log2().ceil() as usize
        } else {
            1
        };
        let sims_per_round = total_sims / num_rounds.max(1) as u64;

        // Build simulation config
        let history_ref = self.history_table.as_deref();
        let sim_cfg = SimConfig {
            c_prior: self.c_prior,
            pw_w: self.pw_w,
            pw_b: self.pw_b,
            history_table: history_ref,
            ph_weight: self.ph_weight,
            max_depth: budget.max_depth.unwrap_or(64),
            prior_temperature: self.prior_temperature,
        };

        for round in 0..num_rounds {
            if candidates.len() <= 1 {
                break;
            }

            let sims_per_candidate =
                (sims_per_round / candidates.len() as u64).max(1);

            for &cand_idx in &candidates {
                for _ in 0..sims_per_candidate {
                    // Budget check: node count
                    if total_sims_done >= total_sims {
                        break;
                    }
                    // Budget check: time (every MCTS_TIME_CHECK_INTERVAL sims)
                    if total_sims_done.is_multiple_of(MCTS_TIME_CHECK_INTERVAL) {
                        if let Some(max_t) = budget.max_time_ms {
                            if start.elapsed().as_millis() as u64 >= max_t {
                                break;
                            }
                        }
                    }

                    let depth = run_simulation(
                        &mut root,
                        cand_idx,
                        position,
                        self.evaluator.as_ref(),
                        &sim_cfg,
                        self.acc_stack.as_mut(),
                        self.nnue_weights.as_deref(),
                    );
                    max_depth_reached = max_depth_reached.max(depth);
                    total_sims_done += 1;
                }

                // Check outer budget too
                if total_sims_done >= total_sims {
                    break;
                }
                if let Some(max_t) = budget.max_time_ms {
                    if start.elapsed().as_millis() as u64 >= max_t {
                        break;
                    }
                }
            }

            // Score and eliminate bottom half
            candidates.sort_by(|&a, &b| {
                let score_a = sigma(
                    root.children[a].gumbel as f64
                        + (root.children[a].prior.max(1e-10) as f64).ln()
                        + root.children[a].q_value(root_player_idx),
                );
                let score_b = sigma(
                    root.children[b].gumbel as f64
                        + (root.children[b].prior.max(1e-10) as f64).ln()
                        + root.children[b].q_value(root_player_idx),
                );
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let half = candidates.len().div_ceil(2);
            candidates.truncate(half);

            // Emit info line after each halving round
            if let Some(cb) = self.info_cb.as_mut() {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                let nps = if elapsed_ms > 0 {
                    total_sims_done * 1000 / elapsed_ms
                } else {
                    0
                };
                let best_cand = candidates[0];
                let q = root.children[best_cand].q_value(root_player_idx);
                let score = q_to_centipawns(q).clamp(-MCTS_SCORE_CAP, MCTS_SCORE_CAP);
                let pv = extract_pv(&root, best_cand);
                let pv_str = if pv.is_empty() {
                    String::from("(none)")
                } else {
                    pv.iter()
                        .map(|m| m.to_algebraic())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                let v = [
                    root.q_value(0),
                    root.q_value(1),
                    root.q_value(2),
                    root.q_value(3),
                ];
                let info_line = format!(
                    "info depth {} score cp {} v1 {:.3} v2 {:.3} v3 {:.3} v4 {:.3} nodes {} nps {} time {} pv {} phase mcts round {}/{}",
                    max_depth_reached,
                    score,
                    v[0], v[1], v[2], v[3],
                    total_sims_done,
                    nps,
                    elapsed_ms,
                    pv_str,
                    round + 1,
                    num_rounds,
                );
                cb(info_line);
            }

            // Tracing
            tracing::debug!(
                round = round + 1,
                total_rounds = num_rounds,
                surviving = candidates.len(),
                sims = total_sims_done,
                "Sequential Halving round complete"
            );
        }

        // If budget was too small for proper halving, ensure all candidates
        // got at least 1 sim. Run remaining budget on unvisited candidates.
        if total_sims_done < total_sims {
            for &cand_idx in &candidates {
                if total_sims_done >= total_sims {
                    break;
                }
                if let Some(max_t) = budget.max_time_ms {
                    if start.elapsed().as_millis() as u64 >= max_t {
                        break;
                    }
                }
                if root.children[cand_idx].visit_count == 0 {
                    let depth = run_simulation(
                        &mut root,
                        cand_idx,
                        position,
                        self.evaluator.as_ref(),
                        &sim_cfg,
                        self.acc_stack.as_mut(),
                        self.nnue_weights.as_deref(),
                    );
                    max_depth_reached = max_depth_reached.max(depth);
                    total_sims_done += 1;
                }
            }
        }

        // Emit top-5 root children by visit count (Stage 18: UI debug panel).
        if let Some(ref mut cb) = self.info_cb {
            let mut visit_pairs: Vec<(String, u32)> = root
                .children
                .iter()
                .filter_map(|c| c.move_to_here.map(|m| (m.to_algebraic(), c.visit_count)))
                .filter(|(_, v)| *v > 0)
                .collect();
            visit_pairs.sort_by(|a, b| b.1.cmp(&a.1));
            visit_pairs.truncate(5);
            if !visit_pairs.is_empty() {
                let visits_str = visit_pairs
                    .iter()
                    .map(|(m, v)| format!("{m}:{v}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                cb(format!("info string mcts_visits {visits_str}"));
            }
        }

        // Select final move
        let best_idx =
            temperature_select(&root, &candidates, self.temperature, &mut self.rng);
        let best_move = root.children[best_idx]
            .move_to_here
            .expect("best candidate must have a move");
        let q = root.children[best_idx].q_value(root_player_idx);
        let score = q_to_centipawns(q);
        let pv = extract_pv(&root, best_idx);

        tracing::debug!(
            best_move = %best_move.to_algebraic(),
            q_value = q,
            score = score,
            visit_count = root.children[best_idx].visit_count,
            total_sims = total_sims_done,
            "MCTS search complete"
        );

        // Clear history table after use — not persistent across searches.
        self.history_table = None;

        SearchResult {
            best_move,
            score,
            depth: max_depth_reached,
            nodes: total_sims_done,
            pv,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Player;

    // -- Step 1: Node basics --

    #[test]
    fn test_node_creation_defaults() {
        let node = MctsNode::new_root(Player::Red);
        assert!(node.move_to_here.is_none());
        assert_eq!(node.player_to_move, Player::Red);
        assert_eq!(node.visit_count, 0);
        assert_eq!(node.value_sum, [0.0; 4]);
        assert!(!node.is_expanded);
        assert!(!node.is_terminal);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_q_value_zero_visits_returns_zero() {
        let node = MctsNode::new_root(Player::Blue);
        for i in 0..4 {
            assert_eq!(node.q_value(i), 0.0);
        }
    }

    #[test]
    fn test_q_value_after_updates() {
        let mut node = MctsNode::new_root(Player::Red);
        node.visit_count = 2;
        node.value_sum = [1.0, 0.6, 0.4, 0.8];
        assert!((node.q_value(0) - 0.5).abs() < 1e-9);
        assert!((node.q_value(1) - 0.3).abs() < 1e-9);
        assert!((node.q_value(2) - 0.2).abs() < 1e-9);
        assert!((node.q_value(3) - 0.4).abs() < 1e-9);
    }

    // -- Step 2: Prior computation --

    #[test]
    fn test_priors_sum_to_approximately_one() {
        // Create some fake moves — we need to test with real Move objects.
        // Use the GameState to get actual legal moves.
        let mut gs = GameState::new_standard_ffa();
        let moves = gs.legal_moves();
        let priors = compute_priors(&moves, MCTS_PRIOR_TEMPERATURE, gs.board());
        let sum: f32 = priors.iter().sum();
        assert!(
            (sum - 1.0).abs() < 0.01,
            "priors sum to {} (expected ~1.0)",
            sum
        );
    }

    #[test]
    fn test_all_priors_positive() {
        let mut gs = GameState::new_standard_ffa();
        let moves = gs.legal_moves();
        let priors = compute_priors(&moves, MCTS_PRIOR_TEMPERATURE, gs.board());
        for (i, &p) in priors.iter().enumerate() {
            assert!(p > 0.0, "prior[{}] = {} (must be > 0)", i, p);
        }
    }

    #[test]
    fn test_softmax_uniform_input() {
        let scores = vec![10.0, 10.0, 10.0, 10.0];
        let result = softmax(&scores, 50.0);
        for &v in &result {
            assert!((v - 0.25).abs() < 0.01);
        }
    }

    // -- Step 3: Gumbel --

    #[test]
    fn test_gumbel_samples_are_finite() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..1000 {
            let g = sample_gumbel(&mut rng);
            assert!(g.is_finite(), "gumbel sample was not finite: {}", g);
        }
    }

    #[test]
    fn test_splitmix64_produces_different_values() {
        let mut rng = SplitMix64::new(0);
        let a = rng.next_u64();
        let b = rng.next_u64();
        let c = rng.next_u64();
        assert_ne!(a, b);
        assert_ne!(b, c);
    }

    #[test]
    fn test_splitmix64_f64_in_range() {
        let mut rng = SplitMix64::new(123);
        for _ in 0..10_000 {
            let v = rng.next_f64();
            assert!(v > 0.0 && v < 1.0, "f64 out of (0,1): {}", v);
        }
    }

    // -- Step 5: Tree policy --

    #[test]
    fn test_unvisited_child_selected_first() {
        let mut parent = MctsNode::new_root(Player::Red);
        // Child 0: visited
        let mut c0 = MctsNode::new_child(
            Move::new(0, 1, crate::board::PieceType::Pawn),
            Player::Blue,
            0.3,
        );
        c0.visit_count = 5;
        c0.value_sum = [2.5, 2.0, 1.5, 1.0];
        // Child 1: unvisited
        let c1 = MctsNode::new_child(
            Move::new(0, 2, crate::board::PieceType::Pawn),
            Player::Blue,
            0.1,
        );
        parent.children.push(c0);
        parent.children.push(c1);
        parent.visit_count = 5;

        let idx = select_child_idx(&parent, MCTS_C_PRIOR, MCTS_PW_W, MCTS_PW_B, None, 0.0);
        assert_eq!(idx, 1, "unvisited child should be selected first");
    }

    #[test]
    fn test_higher_q_preferred_equal_visits() {
        let mut parent = MctsNode::new_root(Player::Red);
        // Both children visited equally, but child 0 has higher Q for Red
        let mut c0 = MctsNode::new_child(
            Move::new(0, 1, crate::board::PieceType::Pawn),
            Player::Blue,
            0.5,
        );
        c0.visit_count = 10;
        c0.value_sum = [8.0, 5.0, 3.0, 4.0]; // Q[Red]=0.8

        let mut c1 = MctsNode::new_child(
            Move::new(0, 2, crate::board::PieceType::Pawn),
            Player::Blue,
            0.5,
        );
        c1.visit_count = 10;
        c1.value_sum = [3.0, 5.0, 6.0, 6.0]; // Q[Red]=0.3

        parent.children.push(c0);
        parent.children.push(c1);
        parent.visit_count = 20;

        let idx = select_child_idx(&parent, MCTS_C_PRIOR, MCTS_PW_W, MCTS_PW_B, None, 0.0);
        assert_eq!(idx, 0, "child with higher Q for root player should be selected");
    }

    // -- Step 8: Progressive widening --

    #[test]
    fn test_pw_limit_grows_with_visits() {
        let l1 = pw_limit(1, MCTS_PW_W, MCTS_PW_B);
        let l4 = pw_limit(4, MCTS_PW_W, MCTS_PW_B);
        let l16 = pw_limit(16, MCTS_PW_W, MCTS_PW_B);
        let l100 = pw_limit(100, MCTS_PW_W, MCTS_PW_B);
        assert!(l1 >= 2, "pw_limit(1) = {}", l1);
        assert!(l4 >= l1, "pw_limit should grow: {} >= {}", l4, l1);
        assert!(l16 >= l4, "pw_limit should grow: {} >= {}", l16, l4);
        assert!(l100 >= l16, "pw_limit should grow: {} >= {}", l100, l16);
    }

    // -- Score conversion --

    #[test]
    fn test_q_to_centipawns() {
        assert_eq!(q_to_centipawns(0.5), 0);
        assert!(q_to_centipawns(0.7) > 0);
        assert!(q_to_centipawns(0.3) < 0);
        assert_eq!(q_to_centipawns(0.0001), -MCTS_SCORE_CAP);
        assert_eq!(q_to_centipawns(0.9999), MCTS_SCORE_CAP);
    }

    // -- PV extraction --

    #[test]
    fn test_extract_pv_follows_most_visited() {
        let mut root = MctsNode::new_root(Player::Red);
        let mv_a = Move::new(0, 1, crate::board::PieceType::Pawn);
        let mv_b = Move::new(0, 2, crate::board::PieceType::Pawn);

        let mut child_a = MctsNode::new_child(mv_a, Player::Blue, 0.5);
        child_a.visit_count = 10;
        let mut child_b = MctsNode::new_child(mv_b, Player::Blue, 0.5);
        child_b.visit_count = 3;

        root.children.push(child_a);
        root.children.push(child_b);

        let pv = extract_pv(&root, 0); // child_a is at index 0
        assert_eq!(pv.len(), 1);
        assert_eq!(pv[0], mv_a, "PV should follow most-visited child");
    }
}
