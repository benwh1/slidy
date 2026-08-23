//! Defines [`Stm`] and [`Mtm`] solvers for small puzzles.
//!
//! [`Stm`]: crate::algorithm::metric::Stm
//! [`Mtm`]: crate::algorithm::metric::Mtm

mod mtm;
mod stm;

use std::{
    cell::{Cell, Ref, RefCell},
    marker::PhantomData,
};

use crate::solver::{small::pdb::Pdb, solver::SolverConfig, stack::Stack};

/// An optimal solver for `WxH` and `HxW` puzzles.
pub struct Solver<const W: usize, const H: usize, const N: usize, MetricTag> {
    pdb: Pdb<W, H, N, MetricTag>,
    stack: Stack<128>,
    solutions_found: Cell<u64>,
    config: RefCell<Option<SolverConfig>>,
    phantom_metric_tag: PhantomData<MetricTag>,
}

impl<const W: usize, const H: usize, const N: usize, MetricTag> Solver<W, H, N, MetricTag> {
    fn cfg(&self) -> Ref<'_, SolverConfig> {
        let borrow = self.config.borrow();
        Ref::map(borrow, |b| b.as_ref().unwrap())
    }

    /// Consumes `self`, returning the inner [`Pdb`].
    pub fn into_inner_pdb(self) -> Pdb<W, H, N, MetricTag> {
        self.pdb
    }
}
