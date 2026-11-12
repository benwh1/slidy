//! [`Stm`]-specific implementation of the projection [`Pdb`].

use crate::{
    algorithm::metric::Stm,
    puzzle::{label::label::Label, size::Size},
    solver::{config::PdbConfig, projection::pdb::Pdb},
};

impl Pdb<Stm> {
    /// Creates and builds a new pattern database for the given [`Label`] and puzzle size.
    ///
    /// # Panics
    ///
    /// Panics if some state is 255 or more moves from the solved state, which is too deep for
    /// the database to store.
    pub fn new<L>(label: &L, size: Size, config: &PdbConfig) -> Self
    where
        L: Label,
    {
        Self::build::<false, _>(label, size, config)
    }
}
