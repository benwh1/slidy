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

/// Builder for a [`Solver`].
///
/// [`Solver`]: crate::solver::projection::solver::Solver
pub struct SolverBuilder<
    'a,
    const W: usize,
    const H: usize,
    const N: usize,
    Target,
    PruneTarget,
    Metric,
> {
    pub(super) target: Option<Target>,
    pub(super) prune_target: Option<PruneTarget>,
    pub(super) pdb_iteration_callback: Option<&'a dyn Fn(PdbIterationStats)>,
    phantom_metric: PhantomData<Metric>,
}

impl<'a, const W: usize, const H: usize, const N: usize, Target, PruneTarget, Metric>
    SolverBuilder<'a, W, H, N, Target, PruneTarget, Metric>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    #[must_use]
    /// Creates a [`SolverBuilder`] with no labels or callback set.
    pub fn new() -> Self {
        Self {
            target: None,
            prune_target: None,
            pdb_iteration_callback: None,
            phantom_metric: PhantomData,
        }
    }

    #[must_use]
    /// Sets the [`SolvedState`] that the solver will solve the puzzle into.
    pub fn target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    #[must_use]
    /// Sets the [`SolvedState`] used for pruning the depth-first search.
    pub fn prune_target(mut self, prune_target: PruneTarget) -> Self {
        self.prune_target = Some(prune_target);
        self
    }

    #[must_use]
    /// Sets the move metric to search in.
    pub fn metric(self, _: Metric) -> Self {
        self
    }

    #[must_use]
    /// Sets a callback that runs after each iteration of the pattern database creation.
    pub fn pdb_iteration_callback(mut self, callback: &'a dyn Fn(PdbIterationStats)) -> Self {
        self.pdb_iteration_callback = Some(callback);
        self
    }
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget, Metric> Default
    for SolverBuilder<'_, W, H, N, Target, PruneTarget, Metric>
{
    fn default() -> Self {
        Self {
            target: None,
            prune_target: None,
            pdb_iteration_callback: None,
            phantom_metric: PhantomData,
        }
    }
}
