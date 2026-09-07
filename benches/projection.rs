use std::{hint::black_box, str::FromStr, time::Duration};

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{
        label::label::{Checkerboard, SplitSquareFringe, SquareFringe},
        puzzle::Puzzle,
    },
    solver::{projection::solver::Solver, solver::Solver as _},
};

const SCRAMBLE: &str = "1 6 0 15/2 7 3 11/13 8 5 14/12 10 9 4";

fn bench_pdb_stm(c: &mut Criterion) {
    type PdbSolver = Solver<4, 4, 16, Checkerboard, Checkerboard, Stm>;

    let mut group = c.benchmark_group("projection/pdb/stm");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("build", |b| {
        b.iter(|| black_box(PdbSolver::builder().build().unwrap()));
    });
    group.finish();
}

fn bench_pdb_mtm(c: &mut Criterion) {
    type PdbSolver = Solver<4, 4, 16, Checkerboard, Checkerboard, Mtm>;

    let mut group = c.benchmark_group("projection/pdb/mtm");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("build", |b| {
        b.iter(|| black_box(PdbSolver::builder().build().unwrap()));
    });
    group.finish();
}

fn bench_solve_stm(c: &mut Criterion) {
    type SolveSolver = Solver<4, 4, 16, SplitSquareFringe, SquareFringe, Stm>;

    let mut group = c.benchmark_group("projection/solve/stm");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    let puzzle = Puzzle::from_str(SCRAMBLE).unwrap();
    let solver = SolveSolver::builder().build().unwrap();
    let p = puzzle.clone();
    group.bench_function("solve", move |b| {
        b.iter(|| {
            let solution = solver.solve(black_box(&p)).unwrap();
            black_box(solution.len_stm::<u64>());
        });
    });
    group.finish();
}

fn bench_solve_mtm(c: &mut Criterion) {
    type SolveSolver = Solver<4, 4, 16, SplitSquareFringe, SquareFringe, Mtm>;

    let mut group = c.benchmark_group("projection/solve/mtm");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    let puzzle = Puzzle::from_str(SCRAMBLE).unwrap();
    let solver = SolveSolver::builder().build().unwrap();
    let p = puzzle.clone();
    group.bench_function("solve", move |b| {
        b.iter(|| {
            let solution = solver.solve(black_box(&p)).unwrap();
            black_box(solution.len_mtm::<u64>());
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_pdb_stm,
    bench_pdb_mtm,
    bench_solve_stm,
    bench_solve_mtm
);
criterion_main!(benches);
