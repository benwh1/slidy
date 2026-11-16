use std::{hint::black_box, iter};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rand::{RngExt as _, SeedableRng as _};
use rand_xoshiro::Xoroshiro128StarStar;
use slidy::{
    algorithm::{algorithm::Algorithm, direction::Direction, r#move::r#move::Move},
    puzzle::{
        label::label::FringeGrids,
        puzzle::Puzzle,
        scrambler::{RandomState, Scrambler as _},
        size::Size,
        sliding_puzzle::SlidingPuzzle as _,
        small::{Puzzle3x3, Puzzle4x4},
    },
};

const DIRS: [Direction; 4] = [
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
];

fn solved_4x4() -> Puzzle4x4 {
    Puzzle4x4::try_from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0]).unwrap()
}

/// 64 random 4x4 states.
fn states() -> Vec<Puzzle4x4> {
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    iter::repeat_with(|| {
        let mut p = solved_4x4();
        RandomState.scramble_with_rng(&mut p, &mut rng);
        p
    })
    .take(64)
    .collect()
}

fn bench_try_move_dir(c: &mut Criterion) {
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    let dirs: Vec<_> = iter::repeat_with(|| DIRS[rng.random_range(0..4)])
        .take(1024)
        .collect();

    let mut p = solved_4x4();
    c.bench_function("puzzle/small/try_move_dir_1024", |b| {
        b.iter(|| {
            let mut moved = 0;
            for &dir in &dirs {
                moved += u32::from(p.try_move_dir(black_box(dir)));
            }
            black_box(moved)
        });
    });

    let mut p = Puzzle3x3::try_from([1, 2, 3, 4, 5, 6, 7, 8, 0]).unwrap();
    c.bench_function("puzzle/small/try_move_dir_1024_3x3", |b| {
        b.iter(|| {
            let mut moved = 0;
            for &dir in &dirs {
                moved += u32::from(p.try_move_dir(black_box(dir)));
            }
            black_box(moved)
        });
    });
}

fn bench_apply_alg(c: &mut Criterion) {
    // A random walk of 256 multi-tile moves that stays on the board.
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    let mut p = solved_4x4();
    let mut alg = Algorithm::new();
    while alg.len_mtm() < 256 {
        let mv = Move::new(DIRS[rng.random_range(0..4)], rng.random_range(1..=3));
        if p.try_apply_move(mv) {
            alg.push_combine(mv);
        }
    }

    let mut p = solved_4x4();
    c.bench_function("puzzle/small/apply_alg_256", |b| {
        b.iter(|| {
            p.reset();
            p.apply_alg(black_box(&alg));
            black_box(&p);
        });
    });
}

fn bench_is_solved(c: &mut Criterion) {
    let states = states();
    c.bench_function("puzzle/small/is_solved_64", |b| {
        b.iter(|| black_box(&states).iter().filter(|p| p.is_solved()).count());
    });
}

fn bench_is_solvable(c: &mut Criterion) {
    let states = states();
    c.bench_function("puzzle/small/is_solvable_64", |b| {
        b.iter(|| {
            black_box(&states)
                .iter()
                .filter(|p| p.is_solvable())
                .count()
        });
    });
}

fn bench_piece_position(c: &mut Criterion) {
    let states = states();
    c.bench_function("puzzle/small/piece_position_64x16", |b| {
        b.iter(|| {
            let mut sum = 0;
            for p in black_box(&states) {
                for piece in 0..16 {
                    sum += p.piece_position(piece);
                }
            }
            sum
        });
    });
}

fn bench_set_state(c: &mut Criterion) {
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    let others: Vec<_> = iter::repeat_with(|| {
        let mut p = Puzzle::new(Size::new(4, 4).unwrap());
        RandomState.scramble_with_rng(&mut p, &mut rng);
        p
    })
    .take(64)
    .collect();

    let mut p = solved_4x4();
    c.bench_function("puzzle/small/set_state_64", |b| {
        b.iter(|| {
            for other in black_box(&others) {
                p.set_state(other);
                black_box(&p);
            }
        });
    });
}

fn bench_scramble(c: &mut Criterion) {
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    let mut p = solved_4x4();
    c.bench_function("puzzle/small/random_state", |b| {
        b.iter(|| {
            RandomState.scramble_with_rng(&mut p, &mut rng);
            black_box(&p);
        });
    });
}

fn bench_reset_to_label(c: &mut Criterion) {
    let states = states();
    c.bench_function("puzzle/small/reset_to_label", |b| {
        b.iter_batched_ref(
            || states[0],
            |p| p.reset_to_label(&FringeGrids),
            BatchSize::SmallInput,
        );
    });
}

fn bench_try_invert(c: &mut Criterion) {
    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
    let mut p = solved_4x4();
    RandomState.scramble_with_rng(&mut p, &mut rng);
    p.apply_move(Move::new(Direction::Up, 3));
    p.apply_move(Move::new(Direction::Left, 3));
    assert!(p.invertible());

    c.bench_function("puzzle/small/try_invert", |b| {
        b.iter(|| black_box(p.try_invert()));
    });
}

fn bench_try_from(c: &mut Criterion) {
    let array = [6, 15, 12, 4, 14, 0, 9, 11, 7, 10, 8, 5, 3, 1, 2, 13];
    c.bench_function("puzzle/small/try_from", |b| {
        b.iter(|| black_box(Puzzle4x4::try_from(black_box(array)).unwrap()));
    });
}

criterion_group!(
    benches,
    bench_try_move_dir,
    bench_apply_alg,
    bench_is_solved,
    bench_is_solvable,
    bench_piece_position,
    bench_set_state,
    bench_scramble,
    bench_reset_to_label,
    bench_try_invert,
    bench_try_from,
);
criterion_main!(benches);
