use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::solver::small::pdb::{Pdb5x2Mtm, Pdb5x2Stm};

fn bench_pdb_5x2_stm(c: &mut Criterion) {
    c.bench_function("solver/small/pdb/pdb_5x2_stm", |b| {
        b.iter(|| black_box(Pdb5x2Stm::default()));
    });
}

fn bench_pdb_5x2_mtm(c: &mut Criterion) {
    c.bench_function("solver/small/pdb/pdb_5x2_mtm", |b| {
        b.iter(|| black_box(Pdb5x2Mtm::default()));
    });
}

criterion_group!(benches, bench_pdb_5x2_stm, bench_pdb_5x2_mtm);
criterion_main!(benches);
