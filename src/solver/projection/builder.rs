//! Builder for constructing a projection [`Solver`].
//!
//! [`Solver`]: crate::solver::projection::solver::Solver

use std::marker::PhantomData;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    puzzle::size::Size,
    solver::{config::PdbConfig, projection::pdb::Pdb},
};

/// Error type for [`SolverBuilder::build`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SolverBuilderError {
    /// Returned from [`SolverBuilder::build`] when no puzzle [`Size`] was provided.
    #[error("MissingSize: a puzzle size must be provided")]
    MissingSize,

    /// Returned from [`SolverBuilder::build`] when the pruning label is not a projection of the
    /// target label.
    #[error("InvalidProjection: the pruning label is not a projection of the target label")]
    InvalidProjection,
}

pub(super) enum PdbAction<Metric> {
    Build { config: PdbConfig },
    UseExisting { pdb: Pdb<Metric> },
}

impl<Metric> Default for PdbAction<Metric> {
    fn default() -> Self {
        Self::Build {
            config: PdbConfig::default(),
        }
    }
}

/// Builder for a [`Solver`].
///
/// [`Solver`]: crate::solver::projection::solver::Solver
#[derive(Default)]
pub struct SolverBuilder<P, Target, PruneTarget, Metric> {
    pub(super) size: Option<Size>,
    pub(super) target: Option<Target>,
    pub(super) prune_target: Option<PruneTarget>,
    pub(super) pdb_action: PdbAction<Metric>,
    phantom_p: PhantomData<P>,
}

impl<P, Target, PruneTarget, Metric> SolverBuilder<P, Target, PruneTarget, Metric> {
    #[must_use]
    /// Creates a [`SolverBuilder`] with default values.
    pub fn new() -> Self {
        Self {
            size: None,
            target: None,
            prune_target: None,
            pdb_action: PdbAction::Build {
                config: PdbConfig::default(),
            },
            phantom_p: PhantomData,
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
    ///
    /// [`SolvedState`]: crate::puzzle::solved_state::SolvedState
    pub fn target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    #[must_use]
    /// Sets the [`SolvedState`] used for pruning the depth-first search.
    ///
    /// [`SolvedState`]: crate::puzzle::solved_state::SolvedState
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
    /// Sets the [`PdbConfig`] used for creating the [`Pdb`].
    ///
    /// Use [`Self::pdb`] to build the [`Solver`] with an existing [`Pdb`].
    ///
    /// [`Solver`]: crate::solver::projection::solver::Solver
    pub fn pdb_config(mut self, config: PdbConfig) -> Self {
        self.pdb_action = PdbAction::Build { config };
        self
    }

    #[must_use]
    /// Sets an existing [`Pdb`] to be used in the [`Solver`].
    ///
    /// Use [`Self::pdb_config`] to build the [`Pdb`] from scratch with a custom configuration.
    ///
    /// [`Solver`]: crate::solver::projection::solver::Solver
    pub fn pdb(mut self, pdb: Pdb<Metric>) -> Self {
        self.pdb_action = PdbAction::UseExisting { pdb };
        self
    }
}
