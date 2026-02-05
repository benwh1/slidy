//! Defines [`Stm`] and [`Mtm`] solvers for small puzzles.
//!
//! [`Stm`]: crate::algorithm::metric::Stm
//! [`Mtm`]: crate::algorithm::metric::Mtm

mod mtm;
mod stm;

use std::{cell::Cell, marker::PhantomData};

use crate::{
    algorithm::{
        direction::Direction,
        metric::{Mtm, Stm},
    },
    solver::small::pdb::Pdb,
};

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

/// [`Solver`] specialized to the 2x2 size and [`Stm`] metric.
pub type Solver2x2Stm = Solver<2, 2, 4, Stm>;
/// [`Solver`] specialized to the 2x2 size and [`Mtm`] metric.
pub type Solver2x2Mtm = Solver<2, 2, 4, Mtm>;
/// [`TransposeSolver`] specialized to the 2x3 size and [`Stm`] metric.
pub type Solver2x3Stm = TransposeSolver<2, 3, 6, Stm>;
/// [`TransposeSolver`] specialized to the 2x3 size and [`Mtm`] metric.
pub type Solver2x3Mtm = TransposeSolver<2, 3, 6, Mtm>;
/// [`TransposeSolver`] specialized to the 2x4 size and [`Stm`] metric.
pub type Solver2x4Stm = TransposeSolver<2, 4, 8, Stm>;
/// [`TransposeSolver`] specialized to the 2x4 size and [`Mtm`] metric.
pub type Solver2x4Mtm = TransposeSolver<2, 4, 8, Mtm>;
/// [`TransposeSolver`] specialized to the 2x5 size and [`Stm`] metric.
pub type Solver2x5Stm = TransposeSolver<2, 5, 10, Stm>;
/// [`TransposeSolver`] specialized to the 2x5 size and [`Mtm`] metric.
pub type Solver2x5Mtm = TransposeSolver<2, 5, 10, Mtm>;
/// [`TransposeSolver`] specialized to the 2x6 size and [`Stm`] metric.
pub type Solver2x6Stm = TransposeSolver<2, 6, 12, Stm>;
/// [`TransposeSolver`] specialized to the 2x6 size and [`Mtm`] metric.
pub type Solver2x6Mtm = TransposeSolver<2, 6, 12, Mtm>;
/// [`Solver`] specialized to the 3x2 size and [`Stm`] metric.
pub type Solver3x2Stm = Solver<3, 2, 6, Stm>;
/// [`Solver`] specialized to the 3x2 size and [`Mtm`] metric.
pub type Solver3x2Mtm = Solver<3, 2, 6, Mtm>;
/// [`Solver`] specialized to the 3x3 size and [`Stm`] metric.
pub type Solver3x3Stm = Solver<3, 3, 9, Stm>;
/// [`Solver`] specialized to the 3x3 size and [`Mtm`] metric.
pub type Solver3x3Mtm = Solver<3, 3, 9, Mtm>;
/// [`TransposeSolver`] specialized to the 3x4 size and [`Stm`] metric.
pub type Solver3x4Stm = TransposeSolver<3, 4, 12, Stm>;
/// [`TransposeSolver`] specialized to the 3x4 size and [`Mtm`] metric.
pub type Solver3x4Mtm = TransposeSolver<3, 4, 12, Mtm>;
/// [`Solver`] specialized to the 4x2 size and [`Stm`] metric.
pub type Solver4x2Stm = Solver<4, 2, 8, Stm>;
/// [`Solver`] specialized to the 4x2 size and [`Mtm`] metric.
pub type Solver4x2Mtm = Solver<4, 2, 8, Mtm>;
/// [`Solver`] specialized to the 4x3 size and [`Stm`] metric.
pub type Solver4x3Stm = Solver<4, 3, 12, Stm>;
/// [`Solver`] specialized to the 4x3 size and [`Mtm`] metric.
pub type Solver4x3Mtm = Solver<4, 3, 12, Mtm>;
/// [`Solver`] specialized to the 5x2 size and [`Stm`] metric.
pub type Solver5x2Stm = Solver<5, 2, 10, Stm>;
/// [`Solver`] specialized to the 5x2 size and [`Mtm`] metric.
pub type Solver5x2Mtm = Solver<5, 2, 10, Mtm>;
/// [`Solver`] specialized to the 6x2 size and [`Stm`] metric.
pub type Solver6x2Stm = Solver<6, 2, 12, Stm>;
/// [`Solver`] specialized to the 6x2 size and [`Mtm`] metric.
pub type Solver6x2Mtm = Solver<6, 2, 12, Mtm>;
