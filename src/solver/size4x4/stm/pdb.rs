//! The pattern databases, one per memory budget, and the [`Pdb`] interface the solver searches
//! with.
//!
//! | database | partitions | size | solves/s |
//! |---|---|---|---|
//! | [`Tiny`] | 6-6-3, 6-6-3 | 5.9 MB | ~130 |
//! | [`Medium`] | 7-7-1, 6-6-3 | 75 MB | ~450 |
//! | [`Large`] | 7-8 | 1.27 GB | ~2,900 |
//! | [`Huge`] | 9-6, 6-9 | 17.0 GB | ~6,000 |
//!
//! Speeds are single-threaded solves per second of random positions on an i5-9500. Each
//! database partitions the tiles into groups, once or twice, and its heuristic is the maximum,
//! over the partitions and both orientations (the position and its mirror image across the main
//! diagonal), of the sum of the groups' distances. A group's distance counts only its own tiles'
//! moves, to its tiles home with the blank free to reach its own home: a group whose tiles wall
//! in the blank's home must make more moves before the puzzle is solved. Some groups' tables also
//! hold the distance counting every move, which is a term of the maximum too.
//!
//! Most tables store only a code of each distance, which the search decodes against the
//! parent's distances (a move changes them by a known amount), so the search carries each node's
//! distances down the tree. The root's come from walking downhill in each coded table.

// Internal labels throughout: serpentine rows `0 1 2 3 / 7 6 5 4 / 8 9 10 11 / 15 14 13 12`,
// tile `t`'s home is cell `t`, the blank's cell 0.

// Huge's tables need more than a 32-bit address space.
#[cfg(target_pointer_width = "64")]
mod huge;
mod large;
mod medium;
mod sealed;
mod tiny;

#[cfg(target_pointer_width = "64")]
pub use huge::Huge;
pub use large::Large;
pub use medium::Medium;
use sealed::Search;
pub use tiny::Tiny;

use crate::solver::config::PdbConfig;

/// A pattern database for the [`Solver`]: building it, and loading it from the bytes it gives
/// through [`AsRef`]. Implemented only by the databases in this module.
///
/// [`Solver`]: super::solver::Solver
pub trait Pdb: AsRef<[u8]> + Sized + Search {
    /// Builds the pattern database.
    fn new(config: &PdbConfig) -> Self;

    /// Initializes a pattern database from a boxed byte slice containing the pre-computed data
    /// (see [`AsRef`]).
    ///
    /// The length of the data is checked, and the [`xxh3`] hash is computed and checked against
    /// a known value to verify integrity.
    ///
    /// # Safety
    ///
    /// Despite the correctness checks described above, this function is unsafe because it is
    /// still technically possible for `bytes` to contain incorrect data in the event of a hash
    /// collision.
    ///
    /// If the data is incorrect, then using the resulting pattern database in the [`Solver`]
    /// can cause undefined behavior.
    ///
    /// [`xxh3`]: xxhash_rust::xxh3
    /// [`Solver`]: super::solver::Solver
    unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self>;

    /// Initializes the pattern database with `bytes`.
    ///
    /// # Safety
    ///
    /// The caller is responsible for the correctness of the data contained in `bytes`. No
    /// correctness checks are performed.
    ///
    /// If incorrect data is used, then use of the pattern database in the [`Solver`] could lead
    /// to incorrect results or undefined behavior.
    ///
    /// [`Solver`]: super::solver::Solver
    unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self;
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use xxhash_rust::xxh3;

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

    /// `pdb` solves [`POSITIONS`] optimally, has hash `hash`, and (unless it is over a gigabyte,
    /// to spare the copy) loads from its bytes.
    pub(super) fn check<P>(pdb: P, hash: u64)
    where
        P: Pdb,
    {
        let mut solver = Solver::with_pdb(pdb);
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
        let bytes = solver.pdb().as_ref();
        let actual = xxh3::xxh3_64(bytes);
        assert_eq!(actual, hash, "the hash is {actual:#x}");
        if bytes.len() < 1 << 30 {
            // SAFETY: the bytes come from a database built above.
            assert!(unsafe { P::try_from_bytes(bytes.into()) }.is_some());
        }
        // SAFETY: wrong lengths are rejected before anything is read.
        assert!(unsafe { P::try_from_bytes(vec![0; 10].into()) }.is_none());
    }
}
