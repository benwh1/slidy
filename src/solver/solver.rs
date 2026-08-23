//! Defines the [`Solver`] trait for a unified solver interface.

use std::{cell::RefCell, rc::Rc};

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
    /// The number of solutions to find.
    pub num_solutions: u64,
    /// A callback that runs after each iteration of the depth-first search.
    pub end_of_iter_callback: Option<Box<dyn Fn(SolverIterationStats)>>,
    /// A callback that runs when a solution is found.
    pub solution_callback: Option<Box<dyn Fn(Algorithm)>>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            min: 0,
            max: u8::MAX,
            num_solutions: 1,
            end_of_iter_callback: None,
            solution_callback: None,
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

    /// Solves `puzzle` using default config.
    fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError> {
        self.solve_many(puzzle, 1).map(|mut v| v.pop().unwrap())
    }

    /// Solves `puzzle` using default config, returning `n` solutions.
    fn solve_many(
        &mut self,
        puzzle: &P,
        num_solutions: u64,
    ) -> Result<Vec<Algorithm>, SolverError> {
        if num_solutions == 0 {
            return Ok(Vec::new());
        }

        let solutions = Rc::new(RefCell::new(Vec::new()));
        let c = solutions.clone();

        let config = SolverConfig {
            num_solutions,
            solution_callback: Some(Box::new(move |s| c.borrow_mut().push(s))),
            ..Default::default()
        };

        self.solve_with_config(puzzle, config)?;

        let solutions = solutions.borrow().clone();
        Ok(solutions)
    }

    /// Solves `puzzle` using the given [`SolverConfig`].
    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>;
}
