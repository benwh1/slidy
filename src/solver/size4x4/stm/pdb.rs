//! The pattern databases: the [`Pdb`] interface the solver searches with, and the table sets
//! that implement it, one per memory budget.
//!
//! | set | partitions | tables | solves/s |
//! |---|---|---|---|
//! | [`Tiny`] | 6-6-3, 6-6-3 | 5.9 MB | ~60 |
//! | [`Medium`] | 7-7-1, 6-6-3 | 75 MB | ~175 |
//! | [`LargeCompact`] | 7-8 | 1.27 GB | ~2,950 |
//! | [`Large`] | 7-8, 9-6 | 3.5 GB | ~3,550 |
//! | [`Huge`] | 9-6, 6-9 | 17.0 GB | ~7,100 |
//!
//! Speeds are single-threaded solves per second of random positions on an i5-9500. Each set is a
//! list of tile groups ([`group`]) whose tables are built ([`build`]), stored ([`region`]) and
//! looked up ([`tables`]) by code they all share. The partitions were found by hill-climbing
//! tile swaps on node counts over 200 random positions.
//!
//! Internal labels throughout: serpentine rows `0 1 2 3 / 7 6 5 4 / 8 9 10 11 / 15 14 13 12`,
//! tile `t`'s home is cell `t`, the blank's cell 0.

pub mod build;
pub mod group;
pub mod huge;
pub mod large;
pub mod large_compact;
pub mod medium;
pub mod region;
pub mod tables;
pub mod tiny;

pub use huge::Huge;
pub use large::Large;
pub use large_compact::LargeCompact;
pub use medium::Medium;
pub use tiny::Tiny;

use crate::solver::size4x4::stm::utils::{Board, Move};

/// A set of pattern databases: everything the search needs to know about its tables. Pairs are
/// `[direct, mirrored]`, the position and its mirror image across the main diagonal.
///
/// The search carries each node's values down the tree, so tables that store only codes can be
/// decoded against the parent's values. Every method is inlined into the search, so keep the
/// methods' data in registers: a value stored at a computed index and read back wider fails store
/// forwarding, and references to locals can keep them in memory.
pub trait Pdb {
    /// A position's table state in both orientations: its indices, and anything else a move
    /// updates incrementally.
    type Indices: Copy;
    /// A node's exact table values (both orientations), carried down the tree for its children
    /// to decode against.
    type Values: Copy;
    /// A node's values set up for decoding its children's entries.
    type Decoder;
    /// A surviving child's decoded estimates, from which [`Self::child`] makes its values.
    type Decoded: Copy;

    /// The root's table state, its exact values and its lower bound, with the blank at `blank`
    /// on the direct board.
    fn root(&self, boards: &[Board; 2], blank: usize) -> (Self::Indices, Self::Values, u64);

    /// The table state after move `m` on `boards`, which moves `tiles` into the blank.
    fn shifted(base: Self::Indices, boards: &[Board; 2], m: &Move, tiles: [u8; 2])
    -> Self::Indices;

    /// Prefetch the table lines [`Self::evaluate`] will read for a child with state `c` and the
    /// blank at `blanks`.
    fn prefetch(&self, c: &Self::Indices, blanks: [u8; 2]);

    /// A node's values set up for [`Self::evaluate`].
    fn decoder(values: Self::Values) -> Self::Decoder;

    /// A child with state `c`, the blank at `blanks` and the moved tile `tiles`: `None` when its
    /// estimate reaches `depth` (the moves left at its parent), else its best-first sort key
    /// (smaller first) and its decoded estimates.
    fn evaluate(
        &self,
        c: &Self::Indices,
        blanks: [u8; 2],
        tiles: [u8; 2],
        decoder: &Self::Decoder,
        depth: u64,
    ) -> Option<(u64, Self::Decoded)>;

    /// A surviving child's values from its parent's `values`, its decoded estimates and the
    /// moved tiles.
    fn child(values: Self::Values, decoded: Self::Decoded, tiles: [u8; 2]) -> Self::Values;
}

/// Gives the set `$set`, a newtype around [`Tables`](tables::Tables), its constructors and its
/// [`Pdb`] implementation, both delegated to the tables.
macro_rules! delegate {
    ($set:ident) => {
        impl $set {
            /// Builds the tables in memory on every available thread.
            #[must_use]
            pub fn build() -> Self {
                let threads = std::thread::available_parallelism().map_or(1, usize::from);
                Self(Tables::build(threads))
            }

            /// Reads the tables from `dir`: the set's combined file (see [`Self::save`]), or else
            /// its separate table files. Only the files' sizes are checked: other tables of the
            /// right sizes give wrong solutions (or a panic), though never undefined behaviour.
            pub fn load(dir: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(Self(Tables::load(dir)?))
            }

            /// Maps the set's combined file in `dir` read-only, so that processes using the same
            /// file share its memory. See [`Self::load`].
            pub fn map(dir: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
                Ok(Self(Tables::map(dir)?))
            }

            /// Saves the tables to `dir` as the set's combined file.
            pub fn save(&self, dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
                self.0.save(dir)
            }
        }

        impl $crate::solver::size4x4::stm::pdb::Pdb for $set {
            type Indices = <Inner as Pdb>::Indices;
            type Values = <Inner as Pdb>::Values;
            type Decoder = <Inner as Pdb>::Decoder;
            type Decoded = <Inner as Pdb>::Decoded;

            #[inline(always)]
            fn root(
                &self,
                boards: &[Board; 2],
                blank: usize,
            ) -> (Self::Indices, Self::Values, u64) {
                self.0.root(boards, blank)
            }

            #[inline(always)]
            fn shifted(
                base: Self::Indices,
                boards: &[Board; 2],
                m: &Move,
                tiles: [u8; 2],
            ) -> Self::Indices {
                Inner::shifted(base, boards, m, tiles)
            }

            #[inline(always)]
            fn prefetch(&self, c: &Self::Indices, blanks: [u8; 2]) {
                self.0.prefetch(c, blanks);
            }

            #[inline(always)]
            fn decoder(values: Self::Values) -> Self::Decoder {
                Inner::decoder(values)
            }

            #[inline(always)]
            fn evaluate(
                &self,
                c: &Self::Indices,
                blanks: [u8; 2],
                tiles: [u8; 2],
                decoder: &Self::Decoder,
                depth: u64,
            ) -> Option<(u64, Self::Decoded)> {
                self.0.evaluate(c, blanks, tiles, decoder, depth)
            }

            #[inline(always)]
            fn child(values: Self::Values, decoded: Self::Decoded, tiles: [u8; 2]) -> Self::Values {
                Inner::child(values, decoded, tiles)
            }
        }
    };
}

use delegate;

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::Pdb;
    use crate::{
        algorithm::algorithm::Algorithm,
        puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle as _},
        solver::{size4x4::stm::solver::Solver, solver::Solver as _},
    };

    /// Random positions and their optimal solution lengths.
    const POSITIONS: [(&str, u64); 12] = [
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

    /// `pdb` solves [`POSITIONS`] optimally.
    pub(super) fn check<P>(pdb: P)
    where
        P: Pdb,
    {
        let mut solver = Solver::new(pdb);
        for (text, length) in POSITIONS {
            let puzzle = Puzzle::from_str(text).unwrap();
            let solution: Algorithm = solver.solve(&puzzle).unwrap();
            let mut solved = puzzle.clone();
            solved.apply_alg(&solution);
            assert!(solved.is_solved(), "{text}: {solution} does not solve it");
            assert_eq!(
                solution.len_stm(),
                length,
                "{text}: {solution} is not optimal"
            );
        }
    }
}
