use criterion::{criterion_group, criterion_main, Criterion};
use slidy::puzzle::{
    display::{DisplayGrid, DisplayInline},
    puzzle::Puzzle,
};

fn bench_display_inline(c: &mut Criterion) {
    let p = Puzzle::default();

    c.bench_function("puzzle/display/display_inline", |b| {
        b.iter(|| DisplayInline::new(&p).to_string());
    });
}

fn bench_display_grid(c: &mut Criterion) {
    let p = Puzzle::default();

    c.bench_function("puzzle/display/display_grid", |b| {
        b.iter(|| DisplayGrid::new(&p).to_string());
    });
}

criterion_group!(puzzle_display, bench_display_inline, bench_display_grid);
criterion_main!(puzzle_display);
