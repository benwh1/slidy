//! Defines [`Stm`] and [`Mtm`] solvers for small puzzles.
//!
//! [`Stm`]: crate::algorithm::metric::Stm
//! [`Mtm`]: crate::algorithm::metric::Mtm

use std::{
    cell::{Cell, Ref, RefCell},
    marker::PhantomData,
};

use crate::solver::{config::SolverConfig, small::pdb::Pdb, stack::Stack};

/// An optimal solver for `WxH` and `HxW` puzzles.
pub struct Solver<const W: usize, const H: usize, const N: usize, Metric> {
    pub(super) pdb: Pdb<W, H, N, Metric>,
    pub(super) stack: Stack<128>,
    pub(super) solutions_found: Cell<u64>,
    pub(super) config: RefCell<Option<SolverConfig>>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

impl<const W: usize, const H: usize, const N: usize, Metric> Solver<W, H, N, Metric> {
    pub(super) fn cfg(&self) -> Ref<'_, SolverConfig> {
        let borrow = self.config.borrow();
        Ref::map(borrow, |b| b.as_ref().unwrap())
    }

    /// Consumes `self`, returning the inner [`Pdb`].
    pub fn into_inner_pdb(self) -> Pdb<W, H, N, Metric> {
        self.pdb
    }
}
