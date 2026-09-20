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
    /// Additional context passed to the solver methods.
    type Context;

    /// Solves `puzzle`, returning an optimal solution.
    fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError>
    where
        Self::Context: Default,
    {
        self.solve_with_context(puzzle, &Self::Context::default())
    }

    /// Solves `puzzle` using the given [`Self::Context`], returning an optimal solution.
    fn solve_with_context(
        &mut self,
        puzzle: &P,
        context: &Self::Context,
    ) -> Result<Algorithm, SolverError> {
        self.solve_many_with_context(puzzle, 1, context)
            .map(|mut v| v.pop().unwrap())
    }

    /// Solves `puzzle`, returning the `n` shortest solutions.
    fn solve_many(&mut self, puzzle: &P, num_solutions: u64) -> Result<Vec<Algorithm>, SolverError>
    where
        Self::Context: Default,
    {
        self.solve_many_with_context(puzzle, num_solutions, &Self::Context::default())
    }

    /// Solves `puzzle` using the given [`Self::Context`], returning the `n` shortest solutions.
    fn solve_many_with_context(
        &mut self,
        puzzle: &P,
        num_solutions: u64,
        context: &Self::Context,
    ) -> Result<Vec<Algorithm>, SolverError> {
        self.solve_collect_with_context(
            puzzle,
            SolverConfig {
                num_solutions,
                ..Default::default()
            },
            context,
        )
    }

    /// Solves `puzzle`, returning all optimal solutions.
    fn solve_all_optimal(&mut self, puzzle: &P) -> Result<Vec<Algorithm>, SolverError>
    where
        Self::Context: Default,
    {
        self.solve_all_optimal_with_context(puzzle, &Self::Context::default())
    }

    /// Solves `puzzle` using the given [`Self::Context`], returning all optimal solutions.
    fn solve_all_optimal_with_context(
        &mut self,
        puzzle: &P,
        context: &Self::Context,
    ) -> Result<Vec<Algorithm>, SolverError> {
        self.solve_collect_with_context(
            puzzle,
            SolverConfig {
                depth_beyond_optimal: 0,
                num_solutions: u64::MAX,
                ..Default::default()
            },
            context,
        )
    }

    /// Solves `puzzle` using the given [`SolverConfig`], collecting the solutions into a [`Vec`].
    fn solve_collect(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<Vec<Algorithm>, SolverError>
    where
        Self::Context: Default,
    {
        self.solve_collect_with_context(puzzle, config, &Self::Context::default())
    }

    /// Solves `puzzle` using the given [`SolverConfig`] and [`Self::Context`], collecting the
    /// solutions into a [`Vec`].
    fn solve_collect_with_context(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
        context: &Self::Context,
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

        self.solve_with_config_and_context(puzzle, config, context)?;

        Ok(receiver.try_iter().collect())
    }

    /// Solves `puzzle` using the given [`SolverConfig`].
    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
    where
        Self::Context: Default,
    {
        self.solve_with_config_and_context(puzzle, config, &Self::Context::default())
    }

    /// Solves `puzzle` using the given [`SolverConfig`] and [`Self::Context`].
    fn solve_with_config_and_context(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
        context: &Self::Context,
    ) -> Result<(), SolverError>;
}
