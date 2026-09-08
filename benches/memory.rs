//! Peak memory usage of building the pattern databases used by the `projection` perf benches.
//!
//! Builds the same PDBs as `projection/pdb/stm/build` and `projection/pdb/mtm/build` (a 4x4
//! Checkerboard) and reports both the peak live heap as measured by glibc `mallinfo2`
//! (`uordblks + hblkhd`, sampled continuously during the build) and the process peak resident
//! set as measured by the kernel. Peak RSS is too coarse to see these small PDBs under glibc's
//! arena reuse, so the live-heap figure is the headline number.
//!
//! Pass `stm` or `mtm` as the single argument to build just one metric.

#![cfg(target_os = "linux")]

use std::{
    env,
    hint::black_box,
    os::raw::c_int,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
};

use slidy::{
    algorithm::metric::{Mtm, Stm},
    puzzle::{label::label::Checkerboard, puzzle::Puzzle, size::Size},
    solver::projection::solver::Solver,
};

type PdbSolverStm = Solver<Puzzle, Checkerboard, Checkerboard, Stm>;
type PdbSolverMtm = Solver<Puzzle, Checkerboard, Checkerboard, Mtm>;

#[repr(C)]
#[derive(Default)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

/// Field-for-field mirror of libc's `struct rusage` on `x86_64` `Linux`, sized so that `getrusage`
/// never writes past the buffer.
#[repr(C)]
#[derive(Default)]
struct Rusage {
    ru_utime: Timeval,
    ru_stime: Timeval,
    ru_maxrss: i64,
    ru_ixrss: i64,
    ru_idrss: i64,
    ru_isrss: i64,
    ru_minflt: i64,
    ru_majflt: i64,
    ru_nswap: i64,
    ru_inblock: i64,
    ru_oublock: i64,
    ru_msgsnd: i64,
    ru_msgrcv: i64,
    ru_nsignals: i64,
    ru_nvcsw: i64,
    ru_nivcsw: i64,
}

/// glibc `struct mallinfo2`.
#[repr(C)]
#[derive(Default)]
struct Mallinfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

extern "C" {
    fn getrusage(who: c_int, usage: *mut Rusage) -> c_int;
    fn mallinfo2() -> Mallinfo2;
}

fn peak_rss_kb() -> i64 {
    let mut usage = Rusage::default();
    // SAFETY: the buffer mirrors the full libc `struct rusage`, and the call is made with
    // `RUSAGE_SELF`. Failing the call is ignored; the snapshot stays all-zero and reports 0.
    let _ = unsafe { getrusage(0, &raw mut usage) };
    usage.ru_maxrss
}

/// Peak of `uordblks + hblkhd` (live heap bytes) while the closure runs. The sampler thread
/// never allocates, so `mallinfo2`'s main-arena accounting stays aligned with the caller.
fn peak_live_heap_during(build: impl FnOnce()) -> usize {
    let peak = AtomicUsize::new(0);
    let done = AtomicBool::new(false);

    thread::scope(|scope| {
        let sampler = thread::Builder::new()
            .name("mallinfo-sampler".to_owned())
            .spawn_scoped(scope, || {
                while !done.load(Ordering::Acquire) {
                    // SAFETY: `mallinfo2` takes no arguments and returns a plain libc struct.
                    let m = unsafe { mallinfo2() };
                    peak.fetch_max(m.uordblks + m.hblkhd, Ordering::SeqCst);
                }
            })
            .unwrap();

        build();
        done.store(true, Ordering::Release);
        sampler.join().unwrap();
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
    let metric = env::args().nth(1);

    let live = match metric.as_deref() {
        Some("stm") => peak_live_heap_during(build_stm),
        Some("mtm") => peak_live_heap_during(build_mtm),
        Some(other) => {
            eprintln!("unknown metric {other}; expected `stm` or `mtm`");
            std::process::exit(2);
        }
        None => peak_live_heap_during(|| {
            build_stm();
            build_mtm();
        }),
    };

    println!("peak_live_heap_kb={}", live / 1024);
    println!("peak_rss_kb={}", peak_rss_kb());
}
