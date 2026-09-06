//! Builder for constructing a projection [`Solver`].
//!
//! [`Solver`]: crate::solver::projection::solver::Solver

use std::marker::PhantomData;

use crate::{
    puzzle::{
        label::label::Label,
        small::{sealed::SmallPuzzle, Puzzle},
        solved_state::SolvedState,
    },
    solver::statistics::PdbIterationStats,
};

pub struct SolverBuilder<
    'a,
    const W: usize,
    const H: usize,
    const N: usize,
    Target,
    PruneTarget,
    MetricTag,
> {
    pub(super) target: Option<Target>,
    pub(super) prune_target: Option<PruneTarget>,
    pub(super) pdb_iteration_callback: Option<&'a dyn Fn(PdbIterationStats)>,
    _metric: PhantomData<MetricTag>,
}

impl<'a, const W: usize, const H: usize, const N: usize, Target, PruneTarget, MetricTag>
    SolverBuilder<'a, W, H, N, Target, PruneTarget, MetricTag>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    #[must_use]
    pub(super) fn new() -> Self {
        Self {
            target: None,
            prune_target: None,
            pdb_iteration_callback: None,
            _metric: PhantomData,
        }
    }

    #[must_use]
    pub fn with_target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    #[must_use]
    pub fn with_prune_target(mut self, prune_target: PruneTarget) -> Self {
        self.prune_target = Some(prune_target);
        self
    }

    #[must_use]
    pub fn with_pdb_iteration_callback(mut self, callback: &'a dyn Fn(PdbIterationStats)) -> Self {
        self.pdb_iteration_callback = Some(callback);
        self
    }
}
