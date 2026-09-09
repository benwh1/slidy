//! Peak memory usage of building the pattern databases used by the `projection` perf benches.
//!
//! Builds the same PDBs as `projection/pdb/stm/build` and `projection/pdb/mtm/build` (a 4x4
//! Checkerboard) and reports the peak resident set of the process as reported by
//! [`sysinfo`](https://crates.io/crates/sysinfo), sampled continuously during the build.

use std::{
    hint::black_box,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
};

use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{label::label::Checkerboard, puzzle::Puzzle, size::Size},
    solver::projection::solver::Solver,
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

type PdbSolverStm = Solver<Puzzle, Checkerboard, Checkerboard, Stm>;
type PdbSolverMtm = Solver<Puzzle, Checkerboard, Checkerboard, Mtm>;

/// Peak resident set (in bytes) while the closure runs. The sampler thread polls this process's
/// memory usage in a tight loop and records the running maximum. The [`System`] is brought up on
/// the calling thread before the sampler starts so the very first sample is already available
/// when `build` begins (a `ProcessesToUpdate::Some` refresh only updates already-known pids).
fn peak_memory_bytes_during(build: impl FnOnce()) -> u64 {
    let peak = AtomicU64::new(0);
    let done = AtomicBool::new(false);
    let mut sys = System::new_all();
    let pid = sysinfo::get_current_pid().unwrap();

    thread::scope(|scope| {
        thread::Builder::new()
            .name("memory-sampler".to_owned())
            .spawn_scoped(scope, || {
                while !done.load(Ordering::Acquire) {
                    sys.refresh_processes_specifics(
                        ProcessesToUpdate::Some(&[pid]),
                        true,
                        ProcessRefreshKind::nothing().with_memory(),
                    );
                    if let Some(process) = sys.process(pid) {
                        peak.fetch_max(process.memory(), Ordering::SeqCst);
                    }
                }
            })
            .unwrap();

        build();
        done.store(true, Ordering::Release);
    });

    peak.load(Ordering::SeqCst)
}

fn build_stm() {
    let solver = PdbSolverStm::builder()
        .size(Size::new(4, 4).unwrap())
        .build()
        .unwrap();
    black_box(solver);
}

fn build_mtm() {
    let solver = PdbSolverMtm::builder()
        .size(Size::new(4, 4).unwrap())
        .build()
        .unwrap();
    black_box(solver);
}

fn main() {
    let peak_kb = peak_memory_bytes_during(build_stm) / 1024;
    println!("peak_memory_kb[stm]={peak_kb}");

    let peak_kb = peak_memory_bytes_during(build_mtm) / 1024;
    println!("peak_memory_kb[mtm]={peak_kb}");
}
