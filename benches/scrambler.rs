use criterion::{criterion_group, criterion_main, Criterion};
use rand::SeedableRng as _;
use rand_xoshiro::Xoroshiro128StarStar;
use slidy::puzzle::{
    puzzle::Puzzle,
    scrambler::{RandomState, Scrambler as _},
    size::Size,
};

fn bench_random_state(c: &mut Criterion) {
    let mut p = Puzzle::new(Size::new(100, 100).unwrap());
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);

    c.bench_function("scrambler/random_state", |b| {
        b.iter(|| RandomState.scramble_with_rng(&mut p, &mut rng));
    });
}

criterion_group!(scrambler, bench_random_state);
criterion_main!(scrambler);
