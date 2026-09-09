use std::str::FromStr as _;

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::algorithm::{
    algorithm::Algorithm,
    display::{
        algorithm::{AlgorithmDisplay as _, DisplaySpaced, DisplayUnspaced},
        r#move::DisplayShort,
    },
};

const ALG: &str = "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2R3D3";

fn bench_display_spaced_display_short(c: &mut Criterion) {
    let a = Algorithm::from_str(ALG).unwrap();
    c.bench_function("algorithm/display_spaced/display_short", |b| {
        b.iter(|| DisplaySpaced::<DisplayShort>::new(&a).to_string());
    });
}

fn bench_display_unspaced_display_short(c: &mut Criterion) {
    let a = Algorithm::from_str(ALG).unwrap();
    c.bench_function("algorithm/display_unspaced/display_short", |b| {
        b.iter(|| DisplayUnspaced::<DisplayShort>::new(&a).to_string());
    });
}

criterion_group!(
    algorithm_display,
    bench_display_spaced_display_short,
    bench_display_unspaced_display_short
);
criterion_main!(algorithm_display);
