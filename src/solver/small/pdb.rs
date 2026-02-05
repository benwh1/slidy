use std::marker::PhantomData;

use crate::{
    algorithm::{
        direction::Direction,
        metric::{Mtm, Stm},
    },
    puzzle::{
        sliding_puzzle::SlidingPuzzle as _,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{small::indexing, statistics::PdbIterationStats},
};

pub struct Pdb<const W: usize, const H: usize, const N: usize, MetricTag> {
    pdb: Box<[u8]>,
    phantom_metric_tag: PhantomData<MetricTag>,
}

impl<const W: usize, const H: usize, const N: usize> Pdb<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    pub(super) fn new_impl(iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let puzzle = Puzzle::<W, H>::new();
        let num_states = puzzle.size().num_states().try_into().unwrap();

        let mut pdb = vec![u8::MAX; num_states];
        let solved_encoded = indexing::encode(puzzle.piece_array());
        pdb[solved_encoded as usize] = 0;

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = iteration_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while new != 0 {
            new = 0;

            for i in 0..num_states {
                if pdb[i] != depth {
                    continue;
                }

                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let piece_array = indexing::decode::<W, N>(i as u64);

                    // SAFETY: `decode` produces valid permutations.
                    let mut puzzle =
                        unsafe { Puzzle::<W, H>::from_piece_array_unchecked(piece_array) };

                    if puzzle.try_move_dir(dir) {
                        let idx = indexing::encode(puzzle.piece_array()) as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            new += 1;
                        }
                    }
                }
            }

            total += new;
            depth += 1;

            if let Some(f) = iteration_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        let pdb = pdb.into_boxed_slice();

        Self {
            pdb,
            phantom_metric_tag: PhantomData,
        }
    }

    pub fn new() -> Self {
        Self::new_impl(None)
    }

    pub fn new_with_iteration_callback(iteration_callback: &dyn Fn(PdbIterationStats)) -> Self {
        Self::new_impl(Some(iteration_callback))
    }
}

impl<const W: usize, const H: usize, const N: usize> Pdb<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    pub(super) fn new_impl(iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let puzzle = Puzzle::<W, H>::new();
        let num_states = puzzle.size().num_states().try_into().unwrap();

        let mut pdb = vec![u8::MAX; num_states];
        let solved_encoded = indexing::encode(puzzle.piece_array());
        pdb[solved_encoded as usize] = 0;

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = iteration_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while new != 0 {
            new = 0;

            for i in 0..num_states {
                if pdb[i] != depth {
                    continue;
                }

                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let piece_array = indexing::decode::<W, N>(i as u64);

                    // SAFETY: `decode` produces valid permutations.
                    let mut puzzle =
                        unsafe { Puzzle::<W, H>::from_piece_array_unchecked(piece_array) };

                    while puzzle.try_move_dir(dir) {
                        let idx = indexing::encode(puzzle.piece_array()) as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            new += 1;
                        }
                    }
                }
            }

            total += new;
            depth += 1;

            if let Some(f) = iteration_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        let pdb = pdb.into_boxed_slice();

        Self {
            pdb,
            phantom_metric_tag: PhantomData,
        }
    }

    pub fn new() -> Self {
        Self::new_impl(None)
    }

    pub fn new_with_iteration_callback(iteration_callback: &dyn Fn(PdbIterationStats)) -> Self {
        Self::new_impl(Some(iteration_callback))
    }
}

impl<const W: usize, const H: usize, const N: usize, MetricTag> Pdb<W, H, N, MetricTag> {
    pub(super) fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    pub(super) unsafe fn get_unchecked(&self, index: usize) -> u8 {
        *self.pdb.get_unchecked(index)
    }
}

impl<const W: usize, const H: usize, const N: usize, MetricTag> AsRef<[u8]>
    for Pdb<W, H, N, MetricTag>
{
    fn as_ref(&self) -> &[u8] {
        &self.pdb
    }
}
