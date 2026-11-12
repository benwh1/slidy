//! Defines the [`Pdb`] struct, which is a pattern database used to accelerate [`Solver`].
//!
//! [`Solver`]: crate::solver::projection::solver::Solver

use std::marker::PhantomData;

use crate::{
    algorithm::direction::Direction,
    puzzle::{label::label::Label, size::Size},
    solver::{
        config::PdbConfig,
        indexing,
        projection::{LARGE, SMALL, puzzle::ProjectedPuzzle},
        statistics::PdbIterationStats,
    },
};

/// Pattern database used by [`Solver`].
///
/// [`Solver`]: crate::solver::projection::solver::Solver
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pdb<Metric> {
    pub(super) pdb: Box<[u8]>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

impl<Metric> Pdb<Metric> {
    /// Initializes the [`Pdb`] with `bytes`.
    ///
    /// # Safety
    ///
    /// The caller is responsible for the correctness of the data contained in `bytes`. No
    /// correctness checks are performed.
    ///
    /// If incorrect data is used, then use of the [`Pdb`] in [`Solver`] could lead to incorrect
    /// results or undefined behavior.
    ///
    /// [`Solver`]: crate::solver::projection::solver::Solver
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
    /// `index` must be within bounds.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
        debug_assert!(index < self.pdb.len());

        // SAFETY: caller's responsibility.
        unsafe { *self.pdb.get_unchecked(index) }
    }

    /// Builds a pattern database for the given [`Label`] and puzzle size, by breadth first search
    /// from the solved state. If `MULTI_TILE` is true, a single move can move any number of pieces
    /// in one direction ([`Mtm`]); otherwise it moves one piece ([`Stm`]).
    ///
    /// # Panics
    ///
    /// Panics if some state is 255 or more moves from the solved state, because depths are stored
    /// as a `u8` with `u8::MAX` meaning "not reached yet".
    ///
    /// [`Mtm`]: crate::algorithm::metric::Mtm
    /// [`Stm`]: crate::algorithm::metric::Stm
    pub(super) fn build<const MULTI_TILE: bool, L>(
        label: &L,
        size: Size,
        config: &PdbConfig,
    ) -> Self
    where
        L: Label,
    {
        if size.area() as usize <= SMALL {
            Self::build_n::<SMALL, MULTI_TILE, _>(label, size, config)
        } else {
            Self::build_n::<LARGE, MULTI_TILE, _>(label, size, config)
        }
    }

    fn build_n<const N: usize, const MULTI_TILE: bool, L>(
        label: &L,
        size: Size,
        config: &PdbConfig,
    ) -> Self
    where
        L: Label,
    {
        let solved_state = compute_solved_state(label, size);
        let tally = compute_tally(&solved_state);
        let gap = size.num_pieces() as u8;
        let width = size.width() as u8;
        let solved = ProjectedPuzzle::<N>::new(&solved_state, gap, width);
        let pdb_size = indexing::multinomial(&tally) as usize;

        let mut pdb = vec![u8::MAX; pdb_size];
        let solved_idx = solved.encode(&tally) as usize;
        pdb[solved_idx] = 0;

        let mut current = vec![solved];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while !current.is_empty() {
            let mut next = Vec::with_capacity(current.len() * 4);

            // `u8::MAX` marks states that haven't been reached, so it can't be used as a depth.
            let next_depth = depth.checked_add(1).filter(|&d| d < u8::MAX);

            for state in &current {
                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = *state;
                    while puzzle.do_move(dir) {
                        let idx = puzzle.encode(&tally) as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = next_depth.expect("pattern database depth overflowed `u8`");
                            next.push(puzzle);
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

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        Self {
            pdb: pdb.into_boxed_slice(),
            phantom_metric: PhantomData,
        }
    }
}

impl<Metric> AsRef<[u8]> for Pdb<Metric> {
    fn as_ref(&self) -> &[u8] {
        &self.pdb
    }
}

pub(super) fn compute_solved_state<L>(label: &L, size: Size) -> Vec<u8>
where
    L: Label,
{
    let n = size.area() as usize;
    let mut state = vec![0; n];
    for (i, s) in state.iter_mut().enumerate() {
        let x = (i as u64) % size.width();
        let y = (i as u64) / size.width();
        *s = label.position_label(size, (x, y)) as u8 + 1;
    }
    state[n - 1] = 0;
    state
}

/// The number of entries in a [`Pdb`] built for `label` on a puzzle of the given size.
pub(super) fn expected_len<L>(label: &L, size: Size) -> usize
where
    L: Label,
{
    let tally = compute_tally(&compute_solved_state(label, size));
    indexing::multinomial(&tally) as usize
}

pub(super) fn compute_tally(solved_state: &[u8]) -> Vec<u8> {
    let max_label = *solved_state.iter().max().unwrap() as usize;
    let mut tally = vec![0; max_label + 1];
    for &l in solved_state {
        tally[l as usize] += 1;
    }
    tally
}
