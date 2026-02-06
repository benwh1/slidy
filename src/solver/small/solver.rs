//! Defines [`Stm`] and [`Mtm`] solvers for small puzzles.
//!
//! [`Stm`]: crate::algorithm::metric::Stm
//! [`Mtm`]: crate::algorithm::metric::Mtm

mod mtm;
mod stm;

use std::{cell::Cell, marker::PhantomData};

use crate::{algorithm::direction::Direction, solver::small::pdb::Pdb};

/// An optimal solver for `WxH` and `HxW` puzzles.
pub struct Solver<const W: usize, const H: usize, const N: usize, MetricTag> {
    pdb: Pdb<W, H, N, MetricTag>,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
    phantom_metric_tag: PhantomData<MetricTag>,
}

impl<const W: usize, const H: usize, const N: usize, MetricTag> Solver<W, H, N, MetricTag> {
    /// Consumes `self`, returning the inner [`Pdb`].
    pub fn into_inner_pdb(self) -> Pdb<W, H, N, MetricTag> {
        self.pdb
    }
}
