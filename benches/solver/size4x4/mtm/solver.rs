use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use slidy::solver::size4x4::mtm::solver::Solver;

fn bench_solver_default(c: &mut Criterion) {
    c.bench_function("solver/size4x4/mtm/solver/default", move |b| {
        b.iter(|| black_box(Solver::default()));
    });
}

criterion_group! {
    name = benches;
    // Building the pattern database takes about 2 s.
    config = Criterion::default().sample_size(10);
    targets = bench_solver_default
}
criterion_main!(benches);
