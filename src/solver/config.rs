//! Defines the [`SolverConfig`] struct which is used by solvers implementing the [`Solver`] trait.
//!
//! [`Solver`]: crate::solver::solver::Solver

use std::ops::ControlFlow;

use crate::{algorithm::algorithm::Algorithm, solver::statistics::SolverIterationStats};

/// Configuration for
/// [`Solver::solve_with_config`](crate::solver::solver::Solver::solve_with_config).
pub struct SolverConfig {
    /// The minimum depth to begin iterative deepening from.
    pub min: u8,

    /// The maximum depth to search to (inclusive).
    pub max: u8,

    /// When set, the search stops once it has deepened at most this far past the depth of the
    /// first solution found. For example `Some(0)` finds only optimal solutions and `Some(2)`
    /// all solutions within two moves of optimal.
    pub depth_beyond_optimal: Option<u8>,

    /// The number of solutions to find.
    pub num_solutions: u64,

    /// A callback that runs after each iteration of the depth-first search.
    ///
    /// Returning [`ControlFlow::Break`] stops the search.
    pub end_of_iter_callback: Option<Box<dyn Fn(SolverIterationStats) -> ControlFlow<()>>>,

    /// A callback that runs when a solution is found.
    ///
    /// Returning [`ControlFlow::Break`] stops the search.
    pub solution_callback: Option<Box<dyn Fn(Algorithm) -> ControlFlow<()>>>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            min: 0,
            max: u8::MAX,
            depth_beyond_optimal: None,
            num_solutions: 1,
            end_of_iter_callback: None,
            solution_callback: None,
        }
    }
}
