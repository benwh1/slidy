use std::{hint::black_box, str::FromStr, time::Duration};

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{
        label::label::{SplitSquareFringe, SquareFringe},
        puzzle::Puzzle,
    },
    solver::{
        projection::solver::Solver,
        solver::Solver as _,
    },
};

type SolverStm = Solver<4, 4, 16, SplitSquareFringe, SquareFringe, Stm>;
type SolverMtm = Solver<4, 4, 16, SplitSquareFringe, SquareFringe, Mtm>;

const SCRAMBLE: &str = "1 6 0 15/2 7 3 11/13 8 5 14/12 10 9 4";

fn bench_pdb(c: &mut Criterion) {
    let mut group = c.benchmark_group("projection/pdb");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("stm", |b| {
        b.iter(|| black_box(SolverStm::builder().build()));
    });
    group.bench_function("mtm", |b| {
        b.iter(|| black_box(SolverMtm::builder().build()));
    });
    group.finish();
}

fn bench_solve(c: &mut Criterion) {
    let mut group = c.benchmark_group("projection/solve");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));

    let puzzle = Puzzle::from_str(SCRAMBLE).unwrap();

    let solver = SolverStm::builder().build();
    let p = puzzle.clone();
    group.bench_function("stm", move |b| {
        b.iter(|| {
            let solution = solver.solve(black_box(&p)).unwrap();
            black_box(solution.len_stm::<u64>());
        });
    });

    let solver = SolverMtm::builder().build();
    let p = puzzle.clone();
    group.bench_function("mtm", move |b| {
        b.iter(|| {
            let solution = solver.solve(black_box(&p)).unwrap();
            black_box(solution.len_mtm::<u64>());
        });
    });
    group.finish();
}

criterion_group!(benches, bench_pdb, bench_solve);
criterion_main!(benches);