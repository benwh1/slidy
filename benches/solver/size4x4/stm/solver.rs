use std::{hint::black_box, str::FromStr as _};

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::{
    puzzle::puzzle::Puzzle,
    solver::{size4x4::stm::solver::Solver, solver::Solver as _},
};

fn bench_solver_default(c: &mut Criterion) {
    c.bench_function("solver/size4x4/stm/solver/default", move |b| {
        b.iter(|| black_box(Solver::default()));
    });
}

fn bench_solve(c: &mut Criterion) {
    let mut solver = Solver::default();
    let puzzle = Puzzle::from_str("6 12 0 3/11 1 10 4/9 7 8 13/15 14 2 5").unwrap();

    c.bench_function("solver/size4x4/stm/solver/solve", move |b| {
        b.iter(|| black_box(solver.solve(&puzzle)));
    });
}

criterion_group!(benches, bench_solver_default, bench_solve);
criterion_main!(benches);
