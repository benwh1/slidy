use std::{hint::black_box, str::FromStr as _};

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{
        label::label::{SplitSquareFringe, SquareFringe},
        puzzle::Puzzle,
        size::Size,
    },
    solver::{projection::solver::Solver, solver::Solver as _},
};

const SCRAMBLE: &str = "1 6 0 15/2 7 3 11/13 8 5 14/12 10 9 4";

fn bench_solve_stm(c: &mut Criterion) {
    type SolverStm = Solver<Puzzle, SplitSquareFringe, SquareFringe, Stm>;

    let puzzle = Puzzle::from_str(SCRAMBLE).unwrap();
    let solver = SolverStm::builder()
        .size(Size::new(4, 4).unwrap())
        .build()
        .unwrap();

    c.bench_function("solve", move |b| {
        b.iter(|| {
            let solution = solver.solve(black_box(&puzzle)).unwrap();
            black_box(solution.len_stm());
        });
    });
}

fn bench_solve_mtm(c: &mut Criterion) {
    type SolverMtm = Solver<Puzzle, SplitSquareFringe, SquareFringe, Mtm>;

    let puzzle = Puzzle::from_str(SCRAMBLE).unwrap();
    let solver = SolverMtm::builder()
        .size(Size::new(4, 4).unwrap())
        .build()
        .unwrap();

    c.bench_function("solve", move |b| {
        b.iter(|| {
            let solution = solver.solve(black_box(&puzzle)).unwrap();
            black_box(solution.len_mtm());
        });
    });
}

criterion_group!(benches, bench_solve_stm, bench_solve_mtm);
criterion_main!(benches);
