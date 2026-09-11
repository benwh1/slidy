use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{label::label::Checkerboard, puzzle::Puzzle, size::Size},
    solver::projection::solver::Solver,
};

fn bench_pdb_stm(c: &mut Criterion) {
    type SolverStm = Solver<Puzzle, Checkerboard, Checkerboard, Stm>;

    c.bench_function("solver/projection/pdb/pdb_stm", |b| {
        b.iter(|| {
            black_box(
                SolverStm::builder()
                    .size(Size::new(4, 4).unwrap())
                    .build()
                    .unwrap(),
            )
        });
    });
}

fn bench_pdb_mtm(c: &mut Criterion) {
    type SolverMtm = Solver<Puzzle, Checkerboard, Checkerboard, Mtm>;

    c.bench_function("solver/projection/pdb/pdb_mtm", |b| {
        b.iter(|| {
            black_box(
                SolverMtm::builder()
                    .size(Size::new(4, 4).unwrap())
                    .build()
                    .unwrap(),
            )
        });
    });
}

criterion_group!(benches, bench_pdb_stm, bench_pdb_mtm);
criterion_main!(benches);
