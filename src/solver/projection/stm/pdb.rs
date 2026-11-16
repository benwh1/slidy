//! [`Stm`]-specific implementation of the projection [`Pdb`].

use crate::{
    algorithm::metric::Stm,
    puzzle::{label::label::Label, size::Size},
    solver::{
        config::PdbConfig,
        projection::pdb::{self, Pdb},
    },
};

impl Pdb<Stm> {
    /// Creates and builds a new pattern database for the given [`Label`] and puzzle size.
    pub fn new<L>(label: &L, size: Size, config: &PdbConfig) -> Self
    where
        L: Label,
    {
        Self::build::<false, _>(label, size, config)
    }

    /// Returns the entry for the state at `index`: `(d >> 1) & 1`, where `d` is the distance of the
    /// state from the projected solved state.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> u8 {
        self.entry::<1>(index)
    }

    /// Returns the entry for the state at `index`, without bounds checking. See [`Self::get`].
    ///
    /// # Safety
    ///
    /// `index` must be less than 8 times the length of the database in bytes.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
        // SAFETY: caller's responsibility.
        unsafe { self.entry_unchecked::<1>(index) }
    }

    /// The length in bytes of a [`Pdb`] built for `label` on a puzzle of the given size.
    pub(in crate::solver::projection) fn expected_len<L>(label: &L, size: Size) -> usize
    where
        L: Label,
    {
        pdb::num_states(label, size).div_ceil(8)
    }
}
