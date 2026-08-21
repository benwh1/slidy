//! Defines the [`Solver`] trait for a unified solver interface.

use thiserror::Error;

use crate::{
    algorithm::algorithm::Algorithm,
    puzzle::{sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::{heuristic::Heuristic, statistics::SolverIterationStats},
};

/// Error type for solvers.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SolverError {
    /// Returned when the search finished without finding a solution.
    #[error("NoSolutionFound: no solution was found within the range searched")]
    NoSolutionFound,

    /// Returned when the solver was given a puzzle of a size that it is not compatible with.
    #[error("IncompatiblePuzzleSize: the puzzle size is incompatible with the solver")]
    IncompatiblePuzzleSize,

    /// Returned when the solver is given an unsolvable puzzle.
    #[error("Unsolvable: the puzzle is unsolvable")]
    Unsolvable,
}

/// Configuration for [`Solver::solve_with_config`].
pub struct SolverConfig {
    /// The minimum depth to begin iterative deepening from.
    pub min: u8,
    /// The maximum depth to search to (inclusive).
    pub max: u8,
    /// An optional callback to be called after each iteration of the depth-first search.
    pub callback: Option<&'static dyn Fn(SolverIterationStats)>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            min: 0,
            max: u8::MAX,
            callback: None,
        }
    }
}

/// A unified interface for optimal puzzle solvers.
///
/// Implementors solve a puzzle and return an optimal solution as an [`Algorithm`].
pub trait Solver<P, T, S, H, M>
where
    P: SlidingPuzzle,
    S: SolvedState,
    H: Heuristic<P, T, S, M>,
{
    /// Returns whether the solver has been initialised.
    fn is_initialised(&self) -> bool;

    /// Initialises the solver. This may involve precomputing pattern databases or other
    /// expensive operations.
    fn init(&mut self);

    /// Solves `puzzle` using default bounds.
    ///
    /// Automatically calls [`Solver::init`] if the solver has not been initialised yet.
    fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError> {
        if !self.is_initialised() {
            self.init();
        }
        self.solve_with_config(puzzle, SolverConfig::default())
    }

    /// Solves `puzzle` using the given [`SolverConfig`].
    ///
    /// Automatically calls [`Solver::init`] if the solver has not been initialised yet.
    fn solve_with_config(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<Algorithm, SolverError>;
}
