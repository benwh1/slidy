use std::marker::PhantomData;

use xxhash_rust::xxh3;

use crate::{
    algorithm::{direction::Direction, metric::Mtm},
    puzzle::{
        sliding_puzzle::SlidingPuzzle as _,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{
        config::PdbConfig,
        small::{indexing, pdb::Pdb},
        statistics::PdbIterationStats,
    },
};

const HASHES: [(usize, usize, u64); 12] = [
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

impl<const W: usize, const H: usize, const N: usize> Default for Pdb<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
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

        let expected_hash = HASHES.iter().find(|(w, h, _)| *w == W && *h == H)?.2;
        let actual_hash = xxh3::xxh3_64(&bytes);

        if actual_hash != expected_hash {
            return None;
        }

        // SAFETY: We checked above that the data is (almost certainly) correct.
        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_pdb_hash<const W: usize, const H: usize, const N: usize>()
    where
        Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
    {
        let pdb = Pdb::<W, H, N, Mtm>::default();
        let actual = xxh3::xxh3_64(pdb.as_ref());
        let expected = HASHES
            .iter()
            .find(|(w, h, _)| *w == W && *h == H)
            .unwrap()
            .2;
        assert_eq!(
            actual, expected,
            "{W}x{H} MTM PDB hash mismatch, got {actual}, expected {expected}",
        );
    }

    #[test]
    fn test_small_hashes() {
        check_pdb_hash::<2, 2, 4>();
        check_pdb_hash::<2, 3, 6>();
        check_pdb_hash::<2, 4, 8>();
        check_pdb_hash::<2, 5, 10>();
        check_pdb_hash::<3, 2, 6>();
        check_pdb_hash::<3, 3, 9>();
        check_pdb_hash::<4, 2, 8>();
        check_pdb_hash::<5, 2, 10>();
    }
}
