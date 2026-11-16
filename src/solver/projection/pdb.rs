//! Defines the [`Pdb`] struct, which is a pattern database used to accelerate [`Solver`].
//!
//! The database stores, for each projected state, enough information to find the state's distance
//! `d` from the projected solved state, given the distance of a neighboring state.
//!
//! In [`Stm`], each state has one bit, `(d >> 1) & 1`. Every move changes `d` by exactly 1, so
//! from a state at distance `h`, the bit tells whether a move goes to `h - 1` or `h + 1`.
//!
//! In [`Mtm`], each state has two bits, `d % 3`, or 3 for states that can't be reached. Every
//! move changes `d` by at most 1, so from a state at distance `h`, the entry tells whether a move
//! goes to `h - 1`, `h`, or `h + 1`.
//!
//! The distance of the first state is found by following moves that get closer until the
//! projected state is solved.
//!
//! [`Mtm`]: crate::algorithm::metric::Mtm
//! [`Solver`]: crate::solver::projection::solver::Solver
//! [`Stm`]: crate::algorithm::metric::Stm

use std::marker::PhantomData;

use crate::{
    puzzle::{label::label::Label, size::Size},
    solver::{
        config::PdbConfig,
        indexing,
        projection::{
            LARGE, SMALL,
            puzzle::{Moves, ProjectedPuzzle},
        },
        statistics::PdbIterationStats,
    },
};

/// Pattern database used by [`Solver`].
///
/// See the [module documentation](self) for the format.
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
    /// results, panics, or failing to terminate.
    ///
    /// [`Solver`]: crate::solver::projection::solver::Solver
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        Self {
            pdb: bytes,
            phantom_metric: PhantomData,
        }
    }

    /// Returns the entry of `BITS` bits for the state at `index`.
    #[inline(always)]
    pub(super) fn entry<const BITS: usize>(&self, index: usize) -> u8 {
        let per_byte = 8 / BITS;
        let shift = BITS * (index % per_byte);
        (self.pdb[index / per_byte] >> shift) & ((1 << BITS) - 1)
    }

    /// Returns the entry of `BITS` bits for the state at `index`, without bounds checking.
    ///
    /// # Safety
    ///
    /// `index` must be less than `8 / BITS` times the length of the database in bytes.
    #[inline(always)]
    pub(super) unsafe fn entry_unchecked<const BITS: usize>(&self, index: usize) -> u8 {
        let per_byte = 8 / BITS;
        let shift = BITS * (index % per_byte);
        debug_assert!(index / per_byte < self.pdb.len());

        // SAFETY: caller's responsibility.
        (unsafe { *self.pdb.get_unchecked(index / per_byte) } >> shift) & ((1 << BITS) - 1)
    }

    /// Builds a pattern database for the given [`Label`] and puzzle size, by breadth first search
    /// from the solved state. If `MULTI_TILE` is true, a single move can move any number of pieces
    /// in one direction ([`Mtm`]), and the database has two bits per state; otherwise a move moves
    /// one piece ([`Stm`]), and the database has one bit per state.
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
        let solved = ProjectedPuzzle::<N>::new(&solved_state, gap);
        let moves = Moves::new(size, &tally);
        let num_states = indexing::multinomial(&tally) as usize;

        let mut table = Table::<MULTI_TILE>::new(num_states);
        table.claim(solved.encode(&tally) as usize, 0);

        let mut current = vec![solved];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while !current.is_empty() {
            let mut next = Vec::with_capacity(current.len() * 4);

            for state in &current {
                let index = state.encode(&tally);
                let mut rest = moves.possible(state.gap());
                while rest != 0 {
                    let d = rest.trailing_zeros();
                    rest &= rest - 1;

                    let (mut puzzle, mut idx) = (*state, index);
                    loop {
                        (puzzle, idx) = moves.apply(&puzzle, idx, d);
                        if table.is_new(idx as usize) {
                            table.claim(idx as usize, depth + 1);
                            next.push(puzzle);
                        }

                        if !MULTI_TILE || moves.possible(puzzle.gap()) & (1 << d) == 0 {
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
            pdb: table.finish(),
            phantom_metric: PhantomData,
        }
    }
}

/// The table being filled in by [`Pdb::build`], with `MULTI_TILE` as there.
struct Table<const MULTI_TILE: bool> {
    /// The entries of the [`Pdb`]. In [`Mtm`], entries of states that haven't been reached are 3.
    ///
    /// [`Mtm`]: crate::algorithm::metric::Mtm
    entries: Vec<u8>,
    /// In [`Stm`], one bit per state: whether the state has been reached. Empty in [`Mtm`].
    ///
    /// [`Mtm`]: crate::algorithm::metric::Mtm
    /// [`Stm`]: crate::algorithm::metric::Stm
    reached: Vec<u64>,
}

impl<const MULTI_TILE: bool> Table<MULTI_TILE> {
    fn new(num_states: usize) -> Self {
        if MULTI_TILE {
            Self {
                entries: vec![u8::MAX; num_states.div_ceil(4)],
                reached: Vec::new(),
            }
        } else {
            Self {
                entries: vec![0; num_states.div_ceil(8)],
                reached: vec![0; num_states.div_ceil(64)],
            }
        }
    }

    /// Returns whether the state at `index` hasn't been reached.
    #[inline(always)]
    fn is_new(&self, index: usize) -> bool {
        if MULTI_TILE {
            (self.entries[index / 4] >> (2 * (index % 4))) & 3 == 3
        } else {
            (self.reached[index / 64] >> (index % 64)) & 1 == 0
        }
    }

    /// Marks the state at `index`, which hasn't been reached, as reached at depth `depth`.
    #[inline(always)]
    fn claim(&mut self, index: usize, depth: u64) {
        if MULTI_TILE {
            // The entry is 3, so this sets it to `depth % 3`.
            self.entries[index / 4] ^= (3 ^ (depth % 3) as u8) << (2 * (index % 4));
        } else {
            self.reached[index / 64] |= 1 << (index % 64);
            self.entries[index / 8] |= (((depth >> 1) & 1) as u8) << (index % 8);
        }
    }

    fn finish(self) -> Box<[u8]> {
        self.entries.into_boxed_slice()
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

/// The number of states of the projection onto `label` of a puzzle of the given size, which is
/// the number of entries in a [`Pdb`] built for them.
pub(super) fn num_states<L>(label: &L, size: Size) -> usize
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        algorithm::metric::{Mtm, Stm},
        puzzle::label::label::{Checkerboard, Rows, Trivial},
        solver::directions::DIRECTIONS,
    };

    /// The distance of each state from the solved state, or `None` for states that can't be
    /// reached, by a plain breadth first search.
    fn distances<L>(label: &L, size: Size, multi_tile: bool) -> Vec<Option<u32>>
    where
        L: Label,
    {
        let solved_state = compute_solved_state(label, size);
        let tally = compute_tally(&solved_state);
        let solved = ProjectedPuzzle::<LARGE>::new(&solved_state, size.num_pieces() as u8);
        let moves = Moves::new(size, &tally);
        let mut distances = vec![None; num_states(label, size)];
        distances[solved.encode(&tally) as usize] = Some(0);

        let mut current = vec![solved];
        let mut depth = 0;
        while !current.is_empty() {
            depth += 1;
            let mut next = Vec::new();
            for state in current {
                for (d, &dir) in DIRECTIONS.iter().enumerate() {
                    let mut child = state;
                    while moves.possible(child.gap()) & (1 << d) != 0 {
                        child = moves.apply_dir(&child, dir);
                        let entry = &mut distances[child.encode(&tally) as usize];
                        if entry.is_none() {
                            *entry = Some(depth);
                            next.push(child);
                        }
                        if !multi_tile {
                            break;
                        }
                    }
                }
            }
            current = next;
        }

        distances
    }

    fn check<L>(label: &L, size: Size)
    where
        L: Label,
    {
        let config = PdbConfig::default();

        let stm = Pdb::<Stm>::new(label, size, &config);
        let distances = distances(label, size, false);
        assert_eq!(stm.pdb.len(), Pdb::<Stm>::expected_len(label, size));
        for (index, d) in distances.iter().enumerate() {
            if let Some(d) = d {
                assert_eq!(
                    u32::from(stm.get(index)),
                    (d >> 1) & 1,
                    "{size} index {index}"
                );
            }
        }

        let mtm = Pdb::<Mtm>::new(label, size, &config);
        let distances = self::distances(label, size, true);
        assert_eq!(mtm.pdb.len(), Pdb::<Mtm>::expected_len(label, size));
        for (index, d) in distances.iter().enumerate() {
            let expected = d.map_or(3, |d| d % 3);
            assert_eq!(u32::from(mtm.get(index)), expected, "{size} index {index}");
        }
    }

    #[test]
    fn test_entries() {
        check(&Trivial, Size::new(3, 3).unwrap());
        check(&Trivial, Size::new(2, 4).unwrap());
        check(&Rows, Size::new(4, 3).unwrap());
        check(&Rows, Size::new(2, 5).unwrap());
        check(&Checkerboard, Size::new(4, 4).unwrap());
        check(&Checkerboard, Size::new(5, 3).unwrap());
    }
}
