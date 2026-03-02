// Stage 19 — Performance benchmarks for profiling and optimization tracking.
//
// Benchmarks: NNUE forward pass, incremental accumulator push, BRS search,
// MCTS search, move generation.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use odin_engine::board::{Board, Player};
use odin_engine::eval::nnue::accumulator::AccumulatorStack;
use odin_engine::eval::nnue::weights::NnueWeights;
use odin_engine::eval::nnue::forward_pass;
use odin_engine::eval::{BootstrapEvaluator, EvalProfile};
use odin_engine::gamestate::GameState;
use odin_engine::movegen::{generate_legal, generate_pseudo_legal, make_move, unmake_move};
use odin_engine::search::brs::BrsSearcher;
use odin_engine::search::mcts::MctsSearcher;
use odin_engine::search::{SearchBudget, Searcher};

fn bench_nnue_forward_pass(c: &mut Criterion) {
    let weights = NnueWeights::random(42);
    let board = Board::starting_position();
    let mut stack = AccumulatorStack::new();
    stack.init_from_board(&board, &weights);
    let acc = stack.current();

    c.bench_function("nnue_forward_pass", |b| {
        b.iter(|| {
            black_box(forward_pass(black_box(acc), black_box(&weights), Player::Red));
        });
    });
}

fn bench_nnue_incremental_push(c: &mut Criterion) {
    let weights = NnueWeights::random(42);
    let mut board = Board::starting_position();
    let moves = generate_legal(&mut board);

    // Use the first legal move for benchmarking push
    let mv = moves[0];

    c.bench_function("nnue_incremental_push", |b| {
        b.iter_batched(
            || {
                let mut stack = AccumulatorStack::new();
                stack.init_from_board(&board, &weights);
                stack
            },
            |mut stack| {
                stack.push(black_box(mv), black_box(&board), black_box(&weights));
                black_box(&stack);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_nnue_full_init(c: &mut Criterion) {
    let weights = NnueWeights::random(42);
    let board = Board::starting_position();
    let mut stack = AccumulatorStack::new();

    c.bench_function("nnue_full_init", |b| {
        b.iter(|| {
            stack.init_from_board(black_box(&board), black_box(&weights));
            black_box(stack.current());
        });
    });
}

fn bench_generate_legal(c: &mut Criterion) {
    let mut board = Board::starting_position();

    c.bench_function("generate_legal_startpos", |b| {
        b.iter(|| {
            let moves = generate_legal(black_box(&mut board));
            black_box(moves);
        });
    });
}

fn bench_generate_pseudo_legal(c: &mut Criterion) {
    let board = Board::starting_position();

    c.bench_function("generate_pseudo_legal_startpos", |b| {
        b.iter(|| {
            let moves = generate_pseudo_legal(black_box(&board));
            black_box(moves);
        });
    });
}

fn bench_brs_depth_4(c: &mut Criterion) {
    let gs = GameState::new_standard_ffa();

    c.bench_function("brs_depth_4_startpos", |b| {
        b.iter_batched(
            || BrsSearcher::new(
                Box::new(BootstrapEvaluator::new(EvalProfile::Standard)),
                None,
            ),
            |mut searcher| {
                let result = searcher.search(
                    black_box(&gs),
                    SearchBudget {
                        max_depth: Some(4),
                        max_nodes: None,
                        max_time_ms: None,
                    },
                );
                black_box(result);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_brs_depth_6(c: &mut Criterion) {
    let gs = GameState::new_standard_ffa();

    let mut group = c.benchmark_group("brs_depth_6");
    group.sample_size(10);
    group.bench_function("startpos", |b| {
        b.iter_batched(
            || BrsSearcher::new(
                Box::new(BootstrapEvaluator::new(EvalProfile::Standard)),
                None,
            ),
            |mut searcher| {
                let result = searcher.search(
                    black_box(&gs),
                    SearchBudget {
                        max_depth: Some(6),
                        max_nodes: None,
                        max_time_ms: None,
                    },
                );
                black_box(result);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_mcts_1000_sims(c: &mut Criterion) {
    let gs = GameState::new_standard_ffa();

    let mut group = c.benchmark_group("mcts_1000_sims");
    group.sample_size(10);
    group.bench_function("startpos", |b| {
        b.iter_batched(
            || MctsSearcher::with_seed(
                Box::new(BootstrapEvaluator::new(EvalProfile::Standard)),
                None,
                42,
            ),
            |mut searcher| {
                let result = searcher.search(
                    black_box(&gs),
                    SearchBudget {
                        max_depth: None,
                        max_nodes: Some(1000),
                        max_time_ms: None,
                    },
                );
                black_box(result);
            },
            criterion::BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_make_unmake(c: &mut Criterion) {
    let mut board = Board::starting_position();
    let moves = generate_legal(&mut board);
    let mv = moves[0];

    c.bench_function("make_unmake_move", |b| {
        b.iter(|| {
            let undo = make_move(black_box(&mut board), black_box(mv));
            unmake_move(&mut board, mv, undo);
            black_box(&board);
        });
    });
}

criterion_group!(
    benches,
    bench_nnue_forward_pass,
    bench_nnue_incremental_push,
    bench_nnue_full_init,
    bench_generate_legal,
    bench_generate_pseudo_legal,
    bench_brs_depth_4,
    bench_brs_depth_6,
    bench_mcts_1000_sims,
    bench_make_unmake,
);
criterion_main!(benches);
