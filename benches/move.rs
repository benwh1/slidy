use criterion::{criterion_group, criterion_main, Criterion};
use slidy::algorithm::{
    direction::Direction,
    display::r#move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort, MoveDisplay as _},
    r#move::r#move::Move,
};

fn bench_display_long_spaced(c: &mut Criterion) {
    let m = Move::new(Direction::Up, 10);
    c.bench_function("move/display_long_spaced", |b| {
        b.iter(|| DisplayLongSpaced::new(m).to_string());
    });
}

fn bench_display_long_unspaced(c: &mut Criterion) {
    let m = Move::new(Direction::Up, 10);
    c.bench_function("move/display_long_unspaced", |b| {
        b.iter(|| DisplayLongUnspaced::new(m).to_string());
    });
}

fn bench_display_short(c: &mut Criterion) {
    let m = Move::new(Direction::Up, 10);
    c.bench_function("move/display_short", |b| {
        b.iter(|| DisplayShort::new(m).to_string());
    });
}

criterion_group!(
    move_benches,
    bench_display_long_spaced,
    bench_display_long_unspaced,
    bench_display_short
);
criterion_main!(move_benches);
