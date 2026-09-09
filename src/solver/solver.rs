//! Defines the [`Solver`] trait for a unified solver interface.

use std::{cell::RefCell, ops::ControlFlow, rc::Rc};

use thiserror::Error;

use crate::{
    algorithm::algorithm::Algorithm, puzzle::sliding_puzzle::SlidingPuzzle,
    solver::config::SolverConfig,
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

/// A generic interface for sliding puzzle solvers.
pub trait Solver<P>
where
    P: SlidingPuzzle,
{
    /// Returns whether the solver has been initialised.
    fn is_initialised(&self) -> bool;

    /// Initialises the solver. This may involve precomputing pattern databases or other
    /// expensive operations.
    fn init(&mut self);

    /// Solves `puzzle`, returning an optimal solution.
    fn solve(&self, puzzle: &P) -> Result<Algorithm, SolverError> {
        self.solve_many(puzzle, 1).map(|mut v| v.pop().unwrap())
    }

    /// Solves `puzzle`, returning the `n` shortest solutions.
    fn solve_many(&self, puzzle: &P, num_solutions: u64) -> Result<Vec<Algorithm>, SolverError> {
        if num_solutions == 0 {
            return Ok(Vec::new());
        }

        let solutions = Rc::new(RefCell::new(Vec::new()));
        let c = solutions.clone();

        let config = SolverConfig {
            num_solutions,
            solution_callback: Some(Box::new(move |s| {
                c.borrow_mut().push(s);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };

        self.solve_with_config(puzzle, config)?;

        let solutions = solutions.borrow().clone();
        Ok(solutions)
    }

    /// Solves `puzzle`, returning all optimal solutions.
    fn solve_all_optimal(&self, puzzle: &P) -> Result<Vec<Algorithm>, SolverError> {
        let solutions = Rc::new(RefCell::new(Vec::new()));
        let c = solutions.clone();

        let config = SolverConfig {
            depth_beyond_optimal: Some(0),
            num_solutions: u64::MAX,
            solution_callback: Some(Box::new(move |s| {
                c.borrow_mut().push(s);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };

        self.solve_with_config(puzzle, config)?;

        let solutions = solutions.borrow().clone();
        Ok(solutions)
    }

    /// Solves `puzzle` using the given [`SolverConfig`].
    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>;
}
