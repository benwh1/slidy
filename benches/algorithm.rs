use std::{hint::black_box, str::FromStr as _};

use criterion::{criterion_group, criterion_main, Criterion};
use slidy::algorithm::algorithm::Algorithm;

fn bench_from_str(c: &mut Criterion) {
    c.bench_function("algorithm/from_str", |b| {
        b.iter(|| {
            black_box(Algorithm::from_str(
                "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2R3D3",
            ))
        });
    });
}

criterion_group!(algorithm, bench_from_str);
criterion_main!(algorithm);
