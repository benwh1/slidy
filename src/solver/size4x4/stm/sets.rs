//! The generic table sets chosen for each memory budget (see [`generic`](super::generic)).
//!
//! Internal labels throughout (serpentine rows `0 1 2 3 / 7 6 5 4 / 8 9 10 11 / 15 14 13 12`,
//! tile `t`'s home is cell `t`, the blank's cell 0). Speeds are single-threaded solves per second
//! of random positions on an i5-9500 (one core, 1000-2000 positions; [`x9669`](super::x9669),
//! 17 GB, does ~7,300 on the same positions).
//!
//! | set | partitions | tables | solves/s |
//! |---|---|---|---|
//! | [`Tiny`] | 6-6-3, 6-6-3 | 5.9 MB | ~60 |
//! | [`Medium`] | 7-7-1, 6-6-3 | 75 MB | ~170-190 |
//! | [`LargeCompact`] | 7-8 | 1.27 GB | ~2,600 |
//! | [`Large`] | 7-8, 9-6 | 3.5 GB | ~3,100 |

use Format::{Bytes8, Bytes16, Min2, Pair, Pair8};

use crate::solver::size4x4::stm::{
    generic::{Format, Gen, Group, Spec, parts},
    pdb::Layout,
};

// The partitions were found by hill-climbing tile swaps on lab node counts (200 random
// positions); the 6-6-3 and 7-7-1 below are local optima, and the tiny set's second partition
// the best of 120 evaluations against the first.

const fn g(tiles: &[u8], partition: u8, format: Format) -> Group {
    Group::new(tiles, partition, format)
}

/// Defines a [`Spec`] and its [`Gen`] alias from a list of groups.
macro_rules! set {
    ($(#[$doc:meta])* $spec:ident, $set:ident, $file:literal, [$($g:expr),* $(,)?]) => {
        #[doc = concat!("The spec of [`", stringify!($set), "`].")]
        pub struct $spec;

        impl $spec {
            const G: [Group; [$(stringify!($g)),*].len()] = [$($g),*];
        }

        impl Spec for $spec {
            const GROUPS: &'static [Group] = &Self::G;
            const LAYOUT: Layout = Layout {
                file: $file,
                parts: &parts(&Self::G),
            };
        }

        $(#[$doc])*
        pub type $set = Gen<$spec, { $spec::G.len() }>;
    };
}

/// Top row but the blank's home.
const A3: &[u8] = &[1, 2, 3];
/// Left half of the bottom three rows.
const B6: &[u8] = &[6, 7, 8, 9, 14, 15];
/// Right half of the bottom three rows.
const C6: &[u8] = &[4, 5, 10, 11, 12, 13];
/// Top two rows but the blank's home.
const T7: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
/// Bottom two rows but the far corner.
const B7: &[u8] = &[8, 9, 10, 11, 13, 14, 15];
/// Bottom two rows.
const B8: &[u8] = &[8, 9, 10, 11, 12, 13, 14, 15];
/// Top two rows and the next row's first two cells, but the blank's home.
const T9: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
/// The rest.
const R6: &[u8] = &[10, 11, 12, 13, 14, 15];

set!(
    /// max of two 6-6-3 partitions, the 6-tile groups as 2-bit minimum codes: 5.9 MB.
    TinySpec,
    Tiny,
    "combined-tiny",
    [
        g(A3, 0, Bytes8),
        g(B6, 0, Min2),
        g(C6, 0, Min2),
        g(&[3, 4, 11], 1, Bytes8),
        g(&[1, 2, 5, 6, 7, 10], 1, Min2),
        g(&[8, 9, 12, 13, 14, 15], 1, Min2),
    ]
);

set!(
    /// max(7-7-1 as 2-bit minimum codes, 6-6-3 exact with the 6-tile groups' full values): 75 MB.
    MediumSpec,
    Medium,
    "combined-medium",
    [
        g(T7, 0, Min2),
        g(B7, 0, Min2),
        g(&[12], 0, Bytes8),
        g(A3, 1, Bytes8),
        g(B6, 1, Pair).with_full(),
        g(C6, 1, Pair).with_full(),
    ]
);

set!(
    /// 7-8 exact with both groups' full values, the 8-tile table packed by the blank's rank:
    /// 1.27 GB.
    LargeCompactSpec,
    LargeCompact,
    "combined-large-compact",
    [g(T7, 0, Pair).with_full(), g(B8, 0, Pair8).with_full()]
);

set!(
    /// max(7-8 exact with full values, 9-6 with a 2-bit minimum 1-9 table and an exact 10-15 one
    /// with its full value): 3.5 GB.
    LargeSpec,
    Large,
    "combined-large",
    [
        g(T7, 0, Pair).with_full(),
        g(B8, 0, Pair).with_full(),
        g(T9, 1, Min2),
        g(R6, 1, Bytes16).with_full(),
    ]
);

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::{
        algorithm::algorithm::Algorithm,
        puzzle::{sliding_puzzle::SlidingPuzzle as _, small::Puzzle4x4},
        solver::size4x4::stm::{build, large, pdb::Region, tables},
    };

    /// Random positions and their optimal solution lengths.
    const POSITIONS: [(&str, usize); 12] = [
        ("3 12 11 15/4 13 14 8/10 6 2 9/1 0 5 7", 56),
        ("3 15 4 9/11 2 14 8/1 0 7 5/13 12 10 6", 51),
        ("5 2 4 1/7 3 14 8/0 13 10 9/11 12 6 15", 44),
        ("5 8 11 9/12 7 14 0/13 1 3 6/15 10 4 2", 58),
        ("11 12 7 13/5 0 6 8/14 15 2 3/9 4 1 10", 58),
        ("10 3 12 5/7 4 9 11/6 1 14 0/2 13 8 15", 49),
        ("5 8 1 7/10 14 3 6/9 15 11 12/4 2 13 0", 44),
        ("11 0 2 4/13 15 10 3/7 14 8 9/6 1 5 12", 49),
        ("9 14 6 15/12 5 4 3/11 8 10 13/7 0 1 2", 60),
        ("9 4 1 2/12 10 13 6/7 11 8 15/3 14 0 5", 53),
        ("5 12 9 7/15 13 8 2/1 6 11 4/14 3 0 10", 51),
        ("7 0 3 10/13 9 1 15/5 12 14 4/6 2 11 8", 45),
    ];

    /// The set's tables, built in memory.
    fn region<S: Spec>() -> Region {
        Region::build(&S::LAYOUT, |part, out| {
            let group = S::GROUPS[part];
            build::encode(group, &build::values(group, 4), out, 4);
        })
        .unwrap()
    }

    fn check<S: Spec, const N: usize>() {
        let region = region::<S>();
        for (text, length) in POSITIONS {
            let board = tables::parse_scramble(text).unwrap();
            let solution = large::solve::<Gen<S, N>>(&board, region.as_dbs()).unwrap();
            let mut puzzle = Puzzle4x4::try_from(board).unwrap();
            puzzle.apply_alg(&Algorithm::from_str(&solution).unwrap());
            assert!(puzzle.is_solved(), "{text}: {solution} does not solve it");
            assert_eq!(solution.len(), length, "{text}: {solution} is not optimal");
        }
    }

    #[test]
    fn tiny_solves_optimally() {
        check::<TinySpec, { TinySpec::G.len() }>();
    }

    #[test]
    #[ignore = "builds 7-tile tables: over a minute"]
    fn medium_solves_optimally() {
        check::<MediumSpec, { MediumSpec::G.len() }>();
    }
}
