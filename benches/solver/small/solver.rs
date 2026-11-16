use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rand::SeedableRng as _;
use rand_xoshiro::Xoshiro256StarStar;
use slidy::{
    puzzle::{
        puzzle::Puzzle,
        scrambler::{RandomState, Scrambler as _},
        size::Size,
    },
    solver::{Solver3x3Mtm, Solver5x2Mtm, config::SolverConfig, solver::Solver as _},
};

/// `count` random puzzles of size `w x h`.
fn puzzles(w: u64, h: u64, count: usize) -> Vec<Puzzle> {
    let mut rng = Xoshiro256StarStar::seed_from_u64(0);
    let mut puzzle = Puzzle::new(Size::new(w, h).unwrap());
    std::iter::repeat_with(|| {
        RandomState.scramble_with_rng(&mut puzzle, &mut rng);
        puzzle.clone()
    })
    .take(count)
    .collect()
}

fn bench_solve_3x3_mtm(c: &mut Criterion) {
    let mut solver = Solver3x3Mtm::default();
    let puzzles = puzzles(3, 3, 1000);

    c.bench_function("solver/small/solve_3x3_mtm", |b| {
        b.iter(|| {
            for puzzle in &puzzles {
                black_box(solver.solve(black_box(puzzle)).unwrap());
            }
        });
    });
}

fn bench_solve_5x2_mtm(c: &mut Criterion) {
    let mut solver = Solver5x2Mtm::default();
    let puzzles = puzzles(5, 2, 1000);

    c.bench_function("solver/small/solve_5x2_mtm", |b| {
        b.iter(|| {
            for puzzle in &puzzles {
                black_box(solver.solve(black_box(puzzle)).unwrap());
            }
        });
    });
}

/// Finds every optimal solution, which searches much more of the tree than finding one.
fn bench_solve_all_5x2_mtm(c: &mut Criterion) {
    let mut solver = Solver5x2Mtm::default();
    let puzzles = puzzles(5, 2, 100);

    c.bench_function("solver/small/solve_all_5x2_mtm", |b| {
        b.iter(|| {
            for puzzle in &puzzles {
                let config = SolverConfig {
                    num_solutions: u64::MAX,
                    ..Default::default()
                };
                black_box(solver.solve_collect(black_box(puzzle), config).unwrap());
            }
        });
    });
}

criterion_group!(
    benches,
    bench_solve_3x3_mtm,
    bench_solve_5x2_mtm,
    bench_solve_all_5x2_mtm
);
criterion_main!(benches);
