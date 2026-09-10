//! Builder for constructing a projection [`Solver`].
//!
//! [`Solver`]: crate::solver::projection::solver::Solver

use std::marker::PhantomData;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    puzzle::{label::label::Label, size::Size, solved_state::SolvedState},
    solver::{config::PdbConfig, statistics::PdbIterationStats},
};

/// Error type for [`SolverBuilder::build`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ProjectionError {
    /// Returned from [`SolverBuilder::build`] when no puzzle [`Size`] was provided.
    #[error("MissingSize: a puzzle size must be provided to the solver builder")]
    MissingSize,

    /// Returned from [`SolverBuilder::build`] when the pruning label is not a projection of the
    /// target label.
    #[error("InvalidProjection: the pruning label is not a projection of the target label")]
    InvalidProjection,
}

/// Result of `SolverBuilder::build_projecting`: the resolved target and pruning labels, the
/// [`PdbConfig`] used to construct the PDB, and the puzzle size.
type BuildProjectionResult<Target, PruneTarget> = (Target, PruneTarget, PdbConfig, Size);

/// Builder for a [`Solver`].
///
/// [`Solver`]: crate::solver::projection::solver::Solver
pub struct SolverBuilder<P, Target, PruneTarget, Metric> {
    size: Option<Size>,
    pub(super) target: Option<Target>,
    pub(super) prune_target: Option<PruneTarget>,
    pub(super) pdb_config: PdbConfig,
    phantom_p: PhantomData<P>,
    phantom_metric: PhantomData<Metric>,
}

impl<P, Target, PruneTarget, Metric> SolverBuilder<P, Target, PruneTarget, Metric> {
    #[must_use]
    /// Creates a [`SolverBuilder`] with no size or labels set.
    pub fn new() -> Self {
        Self {
            size: None,
            target: None,
            prune_target: None,
            pdb_config: PdbConfig::default(),
            phantom_p: PhantomData,
            phantom_metric: PhantomData,
        }
    }

    #[must_use]
    /// Sets the size of puzzle that the solver will solve.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
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
    pub fn pdb_iteration_callback(
        mut self,
        callback: impl Fn(PdbIterationStats) + 'static,
    ) -> Self {
        self.pdb_config.end_of_iter_callback = Some(Box::new(callback));
        self
    }
}

impl<P, Target, PruneTarget, Metric> Default for SolverBuilder<P, Target, PruneTarget, Metric> {
    fn default() -> Self {
        Self {
            size: None,
            target: None,
            prune_target: None,
            pdb_config: PdbConfig::default(),
            phantom_p: PhantomData,
            phantom_metric: PhantomData,
        }
    }
}

impl<P, Target, PruneTarget, Metric> SolverBuilder<P, Target, PruneTarget, Metric>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    pub(super) fn build_projecting(
        self,
    ) -> Result<BuildProjectionResult<Target, PruneTarget>, ProjectionError> {
        let prune_target = self.prune_target.unwrap_or_default();
        let target = self.target.unwrap_or_default();
        let size = self.size.ok_or(ProjectionError::MissingSize)?;

        if !prune_target.is_projection_of(size, &target) {
            return Err(ProjectionError::InvalidProjection);
        }

        Ok((target, prune_target, self.pdb_config, size))
    }
}
