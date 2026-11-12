//! Defines the [`Pdb`] struct, which is a pattern database containing the optimal solution length
//! of every state of a small `WxH` puzzle.
//!
//! This is used by [`Solver`] to efficiently find optimal solutions.
//!
//! [`Solver`]: crate::solver::small::solver::Solver

use std::marker::PhantomData;

use xxhash_rust::xxh3;

use crate::{
    algorithm::{
        direction::Direction,
        metric::{Mtm, Stm},
    },
    puzzle::{
        sliding_puzzle::SlidingPuzzle as _,
        small::{Puzzle, sealed::SmallPuzzle},
    },
    solver::{config::PdbConfig, small::indexing, statistics::PdbIterationStats},
};

/// A pattern database for a small `WxH` puzzle.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pdb<const W: usize, const H: usize, const N: usize, Metric> {
    pub(super) pdb: Box<[u8]>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

/// [`Pdb`] specialized to the 2x2 size and [`Stm`] metric.
pub type Pdb2x2Stm = Pdb<2, 2, 4, Stm>;
/// [`Pdb`] specialized to the 2x2 size and [`Mtm`] metric.
pub type Pdb2x2Mtm = Pdb<2, 2, 4, Mtm>;
/// [`Pdb`] specialized to the 3x2 size and [`Stm`] metric.
pub type Pdb3x2Stm = Pdb<3, 2, 6, Stm>;
/// [`Pdb`] specialized to the 3x2 size and [`Mtm`] metric.
pub type Pdb3x2Mtm = Pdb<3, 2, 6, Mtm>;
/// [`Pdb`] specialized to the 3x3 size and [`Stm`] metric.
pub type Pdb3x3Stm = Pdb<3, 3, 9, Stm>;
/// [`Pdb`] specialized to the 3x3 size and [`Mtm`] metric.
pub type Pdb3x3Mtm = Pdb<3, 3, 9, Mtm>;
/// [`Pdb`] specialized to the 4x2 size and [`Stm`] metric.
pub type Pdb4x2Stm = Pdb<4, 2, 8, Stm>;
/// [`Pdb`] specialized to the 4x2 size and [`Mtm`] metric.
pub type Pdb4x2Mtm = Pdb<4, 2, 8, Mtm>;
/// [`Pdb`] specialized to the 4x3 size and [`Stm`] metric.
pub type Pdb4x3Stm = Pdb<4, 3, 12, Stm>;
/// [`Pdb`] specialized to the 4x3 size and [`Mtm`] metric.
pub type Pdb4x3Mtm = Pdb<4, 3, 12, Mtm>;
/// [`Pdb`] specialized to the 5x2 size and [`Stm`] metric.
pub type Pdb5x2Stm = Pdb<5, 2, 10, Stm>;
/// [`Pdb`] specialized to the 5x2 size and [`Mtm`] metric.
pub type Pdb5x2Mtm = Pdb<5, 2, 10, Mtm>;
/// [`Pdb`] specialized to the 6x2 size and [`Stm`] metric.
pub type Pdb6x2Stm = Pdb<6, 2, 12, Stm>;
/// [`Pdb`] specialized to the 6x2 size and [`Mtm`] metric.
pub type Pdb6x2Mtm = Pdb<6, 2, 12, Mtm>;

impl<const W: usize, const H: usize, const N: usize, Metric> Pdb<W, H, N, Metric> {
    /// See [`Self::try_from_bytes`].
    ///
    /// # Safety
    ///
    /// The caller is responsible for the correctness of the data contained in `bytes`. No
    /// correctness checks are performed.
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        Self {
            pdb: bytes,
            phantom_metric: PhantomData,
        }
    }

    /// Returns the entry for the state at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    /// Returns the entry for the state at `index`, without bounds checking.
    ///
    /// # Safety
    ///
    /// `index` must be a valid index into the PDB.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
        // SAFETY: The caller guarantees that `index` is in bounds.
        unsafe { *self.pdb.get_unchecked(index) }
    }
}

impl<const W: usize, const H: usize, const N: usize, Metric> Pdb<W, H, N, Metric>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    /// Builds a pattern database by breadth first search from the solved state. If `MULTI_TILE`
    /// is true, a single move can move any number of pieces in one direction ([`Mtm`]);
    /// otherwise it moves one piece ([`Stm`]).
    pub(super) fn build<const MULTI_TILE: bool>(config: &PdbConfig) -> Self {
        let puzzle = Puzzle::<W, H>::new();
        let num_states = puzzle.size().num_states().unwrap().try_into().unwrap();

        let mut pdb = vec![u8::MAX; num_states];
        let solved_encoded = indexing::encode_pieces::<N>(puzzle.pieces(), puzzle.gap());
        pdb[solved_encoded as usize] = 0;

        let mut current = vec![puzzle];
        let mut current_index = vec![solved_encoded];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while !current.is_empty() {
            let mut next = Vec::with_capacity(current.len() * 2);
            let mut next_index = Vec::with_capacity(current.len() * 2);

            for (state, parent_index) in current.into_iter().zip(current_index) {
                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = state;
                    let mut run_index = parent_index;

                    loop {
                        let prev_gap = puzzle.gap();
                        if !puzzle.try_move_dir(dir) {
                            break;
                        }
                        let next_gap = puzzle.gap();
                        run_index = if next_gap.abs_diff(prev_gap) == 1 {
                            if next_gap > prev_gap {
                                run_index + 1
                            } else {
                                run_index - 1
                            }
                        } else {
                            indexing::encode_pieces::<N>(puzzle.pieces(), next_gap)
                        };
                        let idx = run_index as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            next.push(puzzle);
                            next_index.push(run_index);
                        }

                        if !MULTI_TILE {
                            break;
                        }
                    }
                }
            }

            new = next.len() as u64;
            total += new;
            depth += 1;

            current = next;
            current_index = next_index;

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        let pdb = pdb.into_boxed_slice();

        Self {
            pdb,
            phantom_metric: PhantomData,
        }
    }

    /// Checks that `bytes` has the expected length, and that its hash is the one given for this
    /// size in `hashes`.
    ///
    /// # Safety
    ///
    /// See [`Pdb::try_from_bytes`].
    pub(super) unsafe fn try_from_bytes_with_hashes(
        bytes: Box<[u8]>,
        hashes: &[(usize, usize, u64)],
    ) -> Option<Self> {
        if Some(bytes.len() as u128) != Puzzle::<W, H>::new().size().num_states() {
            return None;
        }

        let expected_hash = hashes.iter().find(|(w, h, _)| *w == W && *h == H)?.2;
        let actual_hash = xxh3::xxh3_64(&bytes);

        if actual_hash != expected_hash {
            return None;
        }

        // SAFETY: We checked above that the data is (almost certainly) correct.
        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }
}

impl<const W: usize, const H: usize, const N: usize, Metric> AsRef<[u8]> for Pdb<W, H, N, Metric> {
    fn as_ref(&self) -> &[u8] {
        &self.pdb
    }
}
