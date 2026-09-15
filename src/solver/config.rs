//! Defines the [`SolverConfig`] and [`PdbConfig`] structs used by solvers and pattern databases.
//!
//! [`Solver`]: crate::solver::solver::Solver

use std::ops::ControlFlow;

use crate::{
    algorithm::algorithm::Algorithm,
    solver::statistics::{PdbIterationStats, SolverIterationStats},
};

/// Configuration for
/// [`Solver::solve_with_config`](crate::solver::solver::Solver::solve_with_config).
pub struct SolverConfig {
    /// The minimum depth to begin iterative deepening from.
    pub min: u64,

    /// The maximum depth to search to (inclusive).
    pub max: u64,

    /// When set, the search stops once it has deepened at most this far past the depth of the
    /// first solution found. For example, 0 finds only optimal solutions and 2 finds all solutions
    /// within two moves of optimal.
    pub depth_beyond_optimal: u64,

    /// The number of solutions to find.
    pub num_solutions: u64,

    /// A callback that runs after each iteration of the depth-first search.
    ///
    /// Returning [`ControlFlow::Break`] stops the search.
    pub end_of_iter_callback:
        Option<Box<dyn Fn(SolverIterationStats) -> ControlFlow<()> + Send + Sync>>,

    /// A callback that runs when a solution is found.
    ///
    /// Returning [`ControlFlow::Break`] stops the search.
    pub solution_callback: Option<Box<dyn Fn(Algorithm) -> ControlFlow<()> + Send + Sync>>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            min: 0,
            max: u64::MAX,
            depth_beyond_optimal: u64::MAX,
            num_solutions: 1,
            end_of_iter_callback: None,
            solution_callback: None,
        }
    }
}

/// Configuration for constructing pattern databases.
#[derive(Default)]
pub struct PdbConfig {
    /// A callback that runs after each breadth-first-search iteration of the pattern database
    /// creation.
    pub end_of_iter_callback: Option<Box<dyn Fn(PdbIterationStats) + Send + Sync>>,
}
