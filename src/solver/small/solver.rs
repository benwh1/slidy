//! Defines [`Stm`] and [`Mtm`] solvers for small puzzles.
//!
//! [`Stm`]: crate::algorithm::metric::Stm
//! [`Mtm`]: crate::algorithm::metric::Mtm

use std::marker::PhantomData;

use crate::solver::{config::SolverConfig, small::pdb::Pdb, stack::Stack};

/// An optimal solver for `WxH` and `HxW` puzzles.
pub struct Solver<const W: usize, const H: usize, const N: usize, Metric> {
    pub(super) pdb: Pdb<W, H, N, Metric>,
    pub(super) stack: Stack<128>,
    pub(super) solutions_found: u64,
    pub(super) config: Option<SolverConfig>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

impl<const W: usize, const H: usize, const N: usize, Metric> Solver<W, H, N, Metric> {
    /// Creates a [`Solver`] using an existing pattern database.
    #[must_use]
    pub fn with_pdb(pdb: Pdb<W, H, N, Metric>) -> Self {
        Self {
            pdb,
            stack: Stack::new(),
            solutions_found: 0,
            config: None,
            phantom_metric: PhantomData,
        }
    }

    /// Consumes `self`, returning the inner [`Pdb`].
    #[must_use]
    pub fn into_inner_pdb(self) -> Pdb<W, H, N, Metric> {
        self.pdb
    }

    /// Returns a reference to the inner [`Pdb`].
    #[must_use]
    pub fn pdb(&self) -> &Pdb<W, H, N, Metric> {
        &self.pdb
    }
}
