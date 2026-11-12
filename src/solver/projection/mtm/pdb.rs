//! [`Mtm`]-specific implementation of the projection [`Pdb`].

use crate::{
    algorithm::metric::Mtm,
    puzzle::{label::label::Label, size::Size},
    solver::{config::PdbConfig, projection::pdb::Pdb},
};

impl Pdb<Mtm> {
    /// Creates and builds a new pattern database for the given [`Label`] and puzzle size.
    pub fn new<L>(label: &L, size: Size, config: &PdbConfig) -> Self
    where
        L: Label,
    {
        Self::build::<true, _>(label, size, config)
    }
}
