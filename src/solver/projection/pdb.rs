//! Defines the [`Pdb`] struct, which is a pattern database used to accelerate [`Solver`].
//!
//! [`Solver`]: crate::solver::projection::solver::Solver

use std::marker::PhantomData;

use crate::puzzle::{label::label::Label, size::Size};

/// Pattern database used by [`Solver`].
///
/// [`Solver`]: crate::solver::projection::solver::Solver
pub struct Pdb<Metric> {
    pub(super) pdb: Box<[u8]>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

impl<Metric> Pdb<Metric> {
    /// Initialises the [`Pdb`] with `bytes`.
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

pub(super) fn compute_tally(solved_state: &[u8]) -> Vec<u8> {
    let max_label = *solved_state.iter().max().unwrap() as usize;
    let mut tally = vec![0; max_label + 1];
    for &l in solved_state {
        tally[l as usize] += 1;
    }
    tally
}
