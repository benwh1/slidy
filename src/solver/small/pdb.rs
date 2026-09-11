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
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{config::PdbConfig, small::indexing, statistics::PdbIterationStats},
};

const HASHES_STM: [(usize, usize, u64); 12] = [
    (2, 2, 0x66b33be63e234245),
    (2, 3, 0x0f868220a4a06baf),
    (2, 4, 0x9b7a57ad2c6df83f),
    (2, 5, 0x4feabd468458775d),
    (2, 6, 0x84b6f795340a1b8a),
    (3, 2, 0x8275b13928b93c86),
    (3, 3, 0x8812534cd3f7d59f),
    (3, 4, 0x8bcdde83e8fb98b1),
    (4, 2, 0x0a649d41d893eae3),
    (4, 3, 0x835afc1a5551ae94),
    (5, 2, 0x44333aa439ea04fe),
    (6, 2, 0x05084fa633e32abf),
];

const HASHES_MTM: [(usize, usize, u64); 12] = [
    (2, 2, 0x66b33be63e234245),
    (2, 3, 0xa001670b2c0432ab),
    (2, 4, 0x64e76678662d6c49),
    (2, 5, 0xf7e3bbdb27bc8066),
    (2, 6, 0x1221756582872833),
    (3, 2, 0xfb4f384ea556c974),
    (3, 3, 0x2bc75b60a3361302),
    (3, 4, 0x61152679ea24a66a),
    (4, 2, 0x4e6df36030daed02),
    (4, 3, 0x4e91c8da54abdff8),
    (5, 2, 0xe6f36e4ac7284ada),
    (6, 2, 0xa824375d41fb2487),
];

/// A pattern database for a small `WxH` puzzle.
pub struct Pdb<const W: usize, const H: usize, const N: usize, Metric> {
    pdb: Box<[u8]>,
    phantom_metric: PhantomData<Metric>,
}

/// [`Pdb`] specialized to the 2x2 size and [`Stm`] metric.
pub type Pdb2x2Stm = Pdb<2, 2, 4, Stm>;
/// [`Pdb`] specialized to the 2x2 size and [`Mtm`] metric.
pub type Pdb2x2Mtm = Pdb<2, 2, 4, Mtm>;
/// [`Pdb`] specialized to the 2x3 size and [`Stm`] metric.
pub type Pdb2x3Stm = Pdb<2, 3, 6, Stm>;
/// [`Pdb`] specialized to the 2x3 size and [`Mtm`] metric.
pub type Pdb2x3Mtm = Pdb<2, 3, 6, Mtm>;
/// [`Pdb`] specialized to the 2x4 size and [`Stm`] metric.
pub type Pdb2x4Stm = Pdb<2, 4, 8, Stm>;
/// [`Pdb`] specialized to the 2x4 size and [`Mtm`] metric.
pub type Pdb2x4Mtm = Pdb<2, 4, 8, Mtm>;
/// [`Pdb`] specialized to the 2x5 size and [`Stm`] metric.
pub type Pdb2x5Stm = Pdb<2, 5, 10, Stm>;
/// [`Pdb`] specialized to the 2x5 size and [`Mtm`] metric.
pub type Pdb2x5Mtm = Pdb<2, 5, 10, Mtm>;
/// [`Pdb`] specialized to the 2x6 size and [`Stm`] metric.
pub type Pdb2x6Stm = Pdb<2, 6, 12, Stm>;
/// [`Pdb`] specialized to the 2x6 size and [`Mtm`] metric.
pub type Pdb2x6Mtm = Pdb<2, 6, 12, Mtm>;
/// [`Pdb`] specialized to the 3x2 size and [`Stm`] metric.
pub type Pdb3x2Stm = Pdb<3, 2, 6, Stm>;
/// [`Pdb`] specialized to the 3x2 size and [`Mtm`] metric.
pub type Pdb3x2Mtm = Pdb<3, 2, 6, Mtm>;
/// [`Pdb`] specialized to the 3x3 size and [`Stm`] metric.
pub type Pdb3x3Stm = Pdb<3, 3, 9, Stm>;
/// [`Pdb`] specialized to the 3x3 size and [`Mtm`] metric.
pub type Pdb3x3Mtm = Pdb<3, 3, 9, Mtm>;
/// [`Pdb`] specialized to the 3x4 size and [`Stm`] metric.
pub type Pdb3x4Stm = Pdb<3, 4, 12, Stm>;
/// [`Pdb`] specialized to the 3x4 size and [`Mtm`] metric.
pub type Pdb3x4Mtm = Pdb<3, 4, 12, Mtm>;
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

impl<const W: usize, const H: usize, const N: usize> Default for Pdb<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<const W: usize, const H: usize, const N: usize> Default for Pdb<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<const W: usize, const H: usize, const N: usize> Pdb<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    /// Creates and builds a new pattern database for a `WxH` puzzle in the [`Stm`] metric.
    ///
    /// Depending on the size of the puzzle, this may take several minutes to run.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        let puzzle = Puzzle::<W, H>::new();
        let num_states = puzzle.size().num_states().try_into().unwrap();

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
                let parent_gap = state.gap();
                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = state;

                    if puzzle.try_move_dir(dir) {
                        let new_gap = puzzle.gap();
                        let child_index = if new_gap.abs_diff(parent_gap) == 1 {
                            if new_gap > parent_gap {
                                parent_index + 1
                            } else {
                                parent_index - 1
                            }
                        } else {
                            indexing::encode_pieces::<N>(puzzle.pieces(), new_gap)
                        };

                        let idx = child_index as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            next.push(puzzle);
                            next_index.push(child_index);
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

    /// Initializes a [`Pdb`] from a boxed byte slice containing the pre-computed data.
    ///
    /// The length of the data is checked, and the [`xxh3`] hash is computed and checked against a
    /// known value to verify integrity.
    ///
    /// # Safety
    ///
    /// Despite the correctness checks described above, this function is unsafe because it is
    /// still technically possible for `bytes` to contain incorrect data in the event of a hash
    /// collision.
    ///
    /// If the data is incorrect, then using the resulting [`Pdb`] in [`Solver`] can cause undefined
    /// behavior.
    ///
    /// [`Solver`]: crate::solver::small::solver::Solver
    #[must_use]
    pub unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.len() as u128 != Puzzle::<W, H>::new().size().num_states() {
            return None;
        }

        let expected_hash = HASHES_STM.iter().find(|(w, h, _)| *w == W && *h == H)?.2;
        let actual_hash = xxh3::xxh3_64(&bytes);

        if actual_hash != expected_hash {
            return None;
        }

        // SAFETY: We checked above that the data is (almost certainly) correct.
        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }
}

impl<const W: usize, const H: usize, const N: usize> Pdb<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    /// Creates and builds a new pattern database for a `WxH` puzzle in the [`Mtm`] metric.
    ///
    /// Depending on the size of the puzzle, this may take several minutes to run.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        let puzzle = Puzzle::<W, H>::new();
        let num_states = puzzle.size().num_states().try_into().unwrap();

        let mut pdb = vec![u8::MAX; num_states];
        let solved_encoded = indexing::encode(puzzle.piece_array());
        pdb[solved_encoded as usize] = 0;

        let mut current = vec![puzzle];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while !current.is_empty() {
            let mut next = Vec::with_capacity(current.len() * 2);

            for state in current {
                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = state;

                    while puzzle.try_move_dir(dir) {
                        let idx = indexing::encode(puzzle.piece_array()) as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            next.push(puzzle);
                        }
                    }
                }
            }

            new = next.len() as u64;
            total += new;
            depth += 1;

            current = next;

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

    /// Initializes a [`Pdb`] from a boxed byte slice containing the pre-computed data.
    ///
    /// The length of the data is checked, and the [`xxh3`] hash is computed and checked against a
    /// known value to verify integrity.
    ///
    /// # Safety
    ///
    /// Despite the correctness checks described above, this function is unsafe because it is
    /// still technically possible for `bytes` to contain incorrect data in the event of a hash
    /// collision.
    ///
    /// If the data is incorrect, then using the resulting [`Pdb`] in [`Solver`] can cause undefined
    /// behavior.
    ///
    /// [`Solver`]: crate::solver::small::solver::Solver
    #[must_use]
    pub unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.len() as u128 != Puzzle::<W, H>::new().size().num_states() {
            return None;
        }

        let expected_hash = HASHES_MTM.iter().find(|(w, h, _)| *w == W && *h == H)?.2;
        let actual_hash = xxh3::xxh3_64(&bytes);

        if actual_hash != expected_hash {
            return None;
        }

        // SAFETY: We checked above that the data is (almost certainly) correct.
        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }
}

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
        *self.pdb.get_unchecked(index)
    }
}

impl<const W: usize, const H: usize, const N: usize, Metric> AsRef<[u8]> for Pdb<W, H, N, Metric> {
    fn as_ref(&self) -> &[u8] {
        &self.pdb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_5x2_stm_tally() {
        let pdb = Pdb5x2Stm::default();
        let bytes = pdb.as_ref();

        let mut tally = [0u32; 64];
        for &b in bytes {
            tally[b as usize] += 1;
        }

        let expected = [
            1, 2, 3, 6, 11, 19, 30, 44, 68, 112, 176, 271, 411, 602, 851, 1232, 1783, 2530, 3567,
            4996, 6838, 9279, 12463, 16597, 21848, 28227, 35682, 44464, 54597, 65966, 78433, 91725,
            104896, 116966, 126335, 131998, 133107, 128720, 119332, 106335, 91545, 75742, 60119,
            45840, 33422, 23223, 15140, 9094, 5073, 2605, 1224, 528, 225, 75, 20, 2, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        assert_eq!(tally, expected);
    }
}
