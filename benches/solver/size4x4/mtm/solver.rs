use std::{hint::black_box, str::FromStr as _};

use criterion::{Criterion, criterion_group, criterion_main};
use slidy::{
    puzzle::puzzle::Puzzle,
    solver::{size4x4::mtm::solver::Solver, solver::Solver as _},
};

fn bench_solver_default(c: &mut Criterion) {
    c.bench_function("solver/size4x4/mtm/solver/default", move |b| {
        b.iter(|| black_box(Solver::default()));
    });
}

fn bench_solve_all_optimal(c: &mut Criterion) {
    let mut solver = Solver::default();
    let puzzle = Puzzle::from_str("5 8 7 9/0 1 13 14/15 6 2 4/11 12 10 3").unwrap();

    c.bench_function("solver/size4x4/mtm/solver/solve_all_optimal", move |b| {
        b.iter(|| black_box(solver.solve_all_optimal(&puzzle)));
    });
}

criterion_group! {
    name = benches;
    // Building the pattern database takes about 4 s, and the solve about 0.26 s.
    config = Criterion::default().sample_size(10);
    targets = bench_solver_default, bench_solve_all_optimal
}
criterion_main!(benches);
