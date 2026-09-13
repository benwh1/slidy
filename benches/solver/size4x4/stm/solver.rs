use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::solver::size4x4::stm::solver::Solver;

fn bench_solver_default(c: &mut Criterion) {
    c.bench_function("solver/size4x4/stm/solver/default", move |b| {
        b.iter(|| black_box(Solver::default()));
    });
}

criterion_group!(benches, bench_solver_default);
criterion_main!(benches);
