//! Defines [`Stm`] and [`Mtm`] solvers for small puzzles.
//!
//! [`Stm`]: crate::algorithm::metric::Stm
//! [`Mtm`]: crate::algorithm::metric::Mtm

mod mtm;
mod stm;

use std::{cell::Cell, marker::PhantomData};

use crate::{
    algorithm::{
        algorithm::Algorithm,
        direction::Direction,
        metric::{Mtm, Stm},
    },
    puzzle::{
        label::label::RowGrids,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{
        small::pdb::Pdb,
        solver::{Solver as SolverT, SolverConfig, SolverError},
    },
};

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

impl<const W: usize, const H: usize, const N: usize> SolverT<Puzzle<W, H>, u8, RowGrids, (), Stm>
    for Solver<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(
        &mut self,
        puzzle: &Puzzle<W, H>,
        config: SolverConfig,
    ) -> Result<Algorithm, SolverError> {
        if let Some(callback) = config.callback {
            self.solve_with_callback(puzzle, callback)
        } else {
            self.solve(puzzle)
        }
    }
}

impl<const W: usize, const H: usize, const N: usize> SolverT<Puzzle<W, H>, u8, RowGrids, (), Mtm>
    for Solver<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(
        &mut self,
        puzzle: &Puzzle<W, H>,
        config: SolverConfig,
    ) -> Result<Algorithm, SolverError> {
        if let Some(callback) = config.callback {
            self.solve_with_callback(puzzle, callback)
        } else {
            self.solve(puzzle)
        }
    }
}
