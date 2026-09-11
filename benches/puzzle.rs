use std::{hint::black_box, str::FromStr as _};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::SeedableRng as _;
use rand_xoshiro::Xoroshiro128StarStar;
use slidy::{
    algorithm::algorithm::Algorithm,
    puzzle::{
        puzzle::Puzzle,
        scrambler::{RandomState, Scrambler as _},
        size::Size,
        sliding_puzzle::SlidingPuzzle as _,
    },
};

fn bench_reset(c: &mut Criterion) {
    let mut p = Puzzle::default();

    c.bench_function("puzzle/reset", |b| {
        b.iter(|| black_box(p.reset()));
    });
}

fn bench_is_solved(c: &mut Criterion) {
    let p = Puzzle::default();

    c.bench_function("puzzle/is_solved", |b| {
        b.iter(|| black_box(p.is_solved()));
    });
}

fn bench_is_solved_100(c: &mut Criterion) {
    let p = Puzzle::new(Size::new(100, 100).unwrap());

    c.bench_function("puzzle/is_solved_100", |b| {
        b.iter(|| black_box(p.is_solved()));
    });
}

fn bench_solved_pos(c: &mut Criterion) {
    let p = Puzzle::new(Size::new(4, 4).unwrap());

    c.bench_function("puzzle/solved_pos", |b| {
        b.iter(|| black_box(p.solved_pos(10)));
    });
}

fn bench_try_solved_pos(c: &mut Criterion) {
    let p = Puzzle::new(Size::new(4, 4).unwrap());

    c.bench_function("puzzle/try_solved_pos", |b| {
        b.iter(|| black_box(p.try_solved_pos(10).unwrap()));
    });
}

fn bench_solved_pos_xy(c: &mut Criterion) {
    let p = Puzzle::new(Size::new(4, 4).unwrap());

    c.bench_function("puzzle/solved_pos_xy", |b| {
        b.iter(|| black_box(p.solved_pos_xy(10)));
    });
}

fn bench_try_solved_pos_xy(c: &mut Criterion) {
    let p = Puzzle::new(Size::new(4, 4).unwrap());

    c.bench_function("puzzle/try_solved_pos_xy", |b| {
        b.iter(|| black_box(p.try_solved_pos_xy(10).unwrap()));
    });
}

fn bench_can_apply_alg(c: &mut Criterion) {
    let p = Puzzle::default();
    let a = Algorithm::from_str(
        "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2R3D3",
    )
    .unwrap();

    c.bench_function("puzzle/can_apply_alg", |b| {
        b.iter(|| black_box(p.can_apply_alg(&a)));
    });
}

fn bench_apply_alg(c: &mut Criterion) {
    let mut p = Puzzle::default();
    let a = Algorithm::from_str(
        "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2",
    )
    .unwrap();

    c.bench_function("puzzle/apply_alg", |b| {
        b.iter(|| black_box(p.apply_alg(&a)));
    });
}

fn bench_from_str(c: &mut Criterion) {
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    let mut puzzle = Puzzle::new(Size::new(50, 50).unwrap());
    RandomState.scramble_with_rng(&mut puzzle, &mut rng);
    let s = puzzle.to_string();

    c.bench_function("puzzle/from_str", |b| {
        b.iter(|| {
            black_box(Puzzle::from_str(&s).unwrap());
        });
    });
}

criterion_group!(
    puzzle,
    bench_reset,
    bench_is_solved,
    bench_is_solved_100,
    bench_solved_pos,
    bench_try_solved_pos,
    bench_solved_pos_xy,
    bench_try_solved_pos_xy,
    bench_can_apply_alg,
    bench_apply_alg,
    bench_from_str
);
criterion_main!(puzzle);
