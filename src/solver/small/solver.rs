//! Defines [`Stm`] and [`Mtm`] solvers for small puzzles.
//!
//! [`Stm`]: crate::algorithm::metric::Stm
//! [`Mtm`]: crate::algorithm::metric::Mtm

mod mtm;
mod stm;

use std::{cell::Cell, marker::PhantomData};

use crate::{algorithm::direction::Direction, solver::small::pdb::Pdb};

/// An optimal solver for `WxH` puzzles.
pub struct Solver<const W: usize, const H: usize, const N: usize, MetricTag> {
    pdb: Pdb<W, H, N, MetricTag>,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
    phantom_metric_tag: PhantomData<MetricTag>,
}

/// An instance of [`Solver`] that transposes the puzzle, solves it, and transposes the solution.
/// This is so that for non-square `WxH` puzzles, we can re-use the pattern database for solving
/// `HxW` puzzles, instead of generating an essentially equivalent one.
pub struct TransposeSolver<const W: usize, const H: usize, const N: usize, MetricTag>(
    Solver<H, W, N, MetricTag>,
);

impl<const W: usize, const H: usize, const N: usize, MetricTag> Solver<W, H, N, MetricTag> {
    /// Consumes `self`, returning the inner [`Pdb`].
    pub fn into_inner_pdb(self) -> Pdb<W, H, N, MetricTag> {
        self.pdb
    }
}

impl<const W: usize, const H: usize, const N: usize, MetricTag>
    TransposeSolver<W, H, N, MetricTag>
{
    /// Consumes `self`, returning the inner [`Solver`].
    pub fn into_inner(self) -> Solver<H, W, N, MetricTag> {
        self.0
    }
}
