//! Times building the small pattern databases for 12-cell puzzles, which take too long for
//! Criterion's repeated sampling. Pass size names (e.g. `4x3_mtm`) to build only those.

use std::{hint::black_box, time::Instant};

use slidy::solver::small::pdb::{Pdb4x3Mtm, Pdb4x3Stm, Pdb6x2Mtm, Pdb6x2Stm};

fn time(name: &str, filter: &[String], build: impl FnOnce() -> usize) {
    if !filter.is_empty() && !filter.iter().any(|f| f == name) {
        return;
    }
    let start = Instant::now();
    let len = build();
    println!(
        "solver/small/pdb/pdb_{name}: {:.2} s ({len} bytes)",
        start.elapsed().as_secs_f64()
    );
}

fn main() {
    let filter: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    time("4x3_stm", &filter, || {
        black_box(Pdb4x3Stm::default()).as_ref().len()
    });
    time("6x2_stm", &filter, || {
        black_box(Pdb6x2Stm::default()).as_ref().len()
    });
    time("4x3_mtm", &filter, || {
        black_box(Pdb4x3Mtm::default()).as_ref().len()
    });
    time("6x2_mtm", &filter, || {
        black_box(Pdb6x2Mtm::default()).as_ref().len()
    });
}
