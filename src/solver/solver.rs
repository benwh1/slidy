//! Defines the [`Solver`] trait for a unified solver interface.

use std::{ops::ControlFlow, sync::mpsc};

use thiserror::Error;

use crate::{
    algorithm::algorithm::Algorithm, puzzle::sliding_puzzle::SlidingPuzzle,
    solver::config::SolverConfig,
};

/// Error type for solvers.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SolverError {
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
    fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError> {
        self.solve_many(puzzle, 1).map(|mut v| v.pop().unwrap())
    }

    /// Solves `puzzle`, returning the `n` shortest solutions.
    fn solve_many(
        &mut self,
        puzzle: &P,
        num_solutions: u64,
    ) -> Result<Vec<Algorithm>, SolverError> {
        self.solve_collect(
            puzzle,
            SolverConfig {
                num_solutions,
                ..Default::default()
            },
        )
    }

    /// Solves `puzzle`, returning all optimal solutions.
    fn solve_all_optimal(&mut self, puzzle: &P) -> Result<Vec<Algorithm>, SolverError> {
        self.solve_collect(
            puzzle,
            SolverConfig {
                depth_beyond_optimal: Some(0),
                num_solutions: u64::MAX,
                ..Default::default()
            },
        )
    }

    /// Solves `puzzle` using the given [`SolverConfig`], collecting the solutions into a [`Vec`].
    fn solve_collect(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<Vec<Algorithm>, SolverError> {
        let (sender, receiver) = mpsc::channel();
        let user_callback = config.solution_callback;

        let config = SolverConfig {
            solution_callback: Some(Box::new(move |s| {
                if let Some(f) = &user_callback {
                    sender.send(s.clone()).unwrap();
                    f(s)
                } else {
                    sender.send(s).unwrap();
                    ControlFlow::Continue(())
                }
            })),
            ..config
        };

        self.solve_with_config(puzzle, config)?;

        Ok(receiver.try_iter().collect())
    }

    /// Solves `puzzle` using the given [`SolverConfig`].
    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>;
}
