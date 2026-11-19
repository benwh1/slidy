//! [`Mtm`]-specific implementation of the projection [`Pdb`].

use crate::{
    algorithm::metric::Mtm,
    puzzle::{label::label::Label, size::Size},
    solver::{
        config::PdbConfig,
        projection::pdb::{self, Pdb},
    },
};

impl Pdb<Mtm> {
    /// Creates and builds a new pattern database for the given [`Label`] and puzzle size.
    pub fn new<L>(label: &L, size: Size, config: &PdbConfig) -> Self
    where
        L: Label,
    {
        Self::build::<true, _>(label, size, config)
    }

    /// Returns the entry for the state at `index`: `d % 3`, where `d` is the distance of the state
    /// from the projected solved state, or 3 if the state can't be reached.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> u8 {
        self.entry::<2>(index)
    }

    /// Returns the entry for the state at `index`, without bounds checking. See [`Self::get`].
    ///
    /// # Safety
    ///
    /// `index` must be less than 4 times the length of the database in bytes.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
        // SAFETY: caller's responsibility.
        unsafe { self.entry_unchecked::<2>(index) }
    }

    /// The length in bytes of a [`Pdb`] built for `label` on a puzzle of the given size.
    pub(crate) fn expected_len<L>(label: &L, size: Size) -> usize
    where
        L: Label,
    {
        pdb::num_states(label, size).div_ceil(4)
    }
}
