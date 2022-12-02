//! Defines the [`Solver`] struct for computing optimal solutions.

use std::marker::PhantomData;

use num_traits::{AsPrimitive, PrimInt, Unsigned};
use thiserror::Error;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction, puzzle_move::Move},
    puzzle::sliding_puzzle::SlidingPuzzle,
};

use super::heuristic::Heuristic;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Stack {
    stack: [Direction; 256],
    idx: usize,
}

impl Stack {
    fn push(&mut self, direction: Direction) {
        self.stack[self.idx] = direction;
        self.idx += 1;
    }

    fn top(&self) -> Option<Direction> {
        if self.idx == 0 {
            None
        } else {
            Some(self.stack[self.idx - 1])
        }
    }

    fn pop(&mut self) -> Direction {
        self.idx -= 1;
        self.stack[self.idx]
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            stack: [Direction::Up; 256],
            idx: 0,
        }
    }
}

impl From<&Stack> for Algorithm {
    fn from(stack: &Stack) -> Self {
        Self::new(
            stack.stack[..stack.idx]
                .iter()
                .map(|d| Move::from(*d))
                .collect(),
        )
    }
}

/// Error type for [`Solver`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SolverError {
    /// Returned when the search finished without finding a solution.
    #[error("NoSolutionFound: no solution was found within the range searched.")]
    NoSolutionFound,
}

/// An optimal puzzle solver using a [`Heuristic`] `H` to speed up the search. The type parameter
/// `T` should be chosen such that the maximum length of a potential solution is less than the
/// maximum value of a `T`. In almost all cases, `T = u8` should be used.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Solver<'a, Piece, Puzzle, T, H>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece> + Clone,
    T: PrimInt + Unsigned + 'static,
    H: Heuristic<Piece, Puzzle, T>,
    u8: AsPrimitive<T>,
{
    puzzle: Puzzle,
    stack: Stack,
    phantom_piece: PhantomData<Piece>,
    heuristic: &'a H,
    phantom_t: PhantomData<T>,
}

impl<'a, Piece, Puzzle, H> Solver<'a, Piece, Puzzle, u8, H>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece> + Clone,
    H: Heuristic<Piece, Puzzle, u8>,
{
    /// Constructs a new [`Solver`] for solving `puzzle`.
    pub fn new(puzzle: &Puzzle, heuristic: &'a H) -> Self {
        Self {
            puzzle: puzzle.clone(),
            stack: Stack::default(),
            phantom_piece: PhantomData,
            heuristic,
            phantom_t: PhantomData,
        }
    }
}

impl<'a, Piece, Puzzle, T, H> Solver<'a, Piece, Puzzle, T, H>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece> + Clone,
    T: PrimInt + Unsigned + 'static,
    H: Heuristic<Piece, Puzzle, T>,
    u8: AsPrimitive<T>,
{
    /// Constructs a new [`Solver`] for solving `puzzle`.
    pub fn new_with_t(puzzle: &Puzzle, heuristic: &'a H) -> Self {
        Self {
            puzzle: puzzle.clone(),
            stack: Stack::default(),
            phantom_piece: PhantomData,
            heuristic,
            phantom_t: PhantomData::<T>,
        }
    }

    fn dfs(&mut self, depth: T) -> bool {
        if depth == T::zero() {
            if self.puzzle.is_solved() {
                return true;
            }
            return false;
        }

        let bound = self.heuristic.bound(&self.puzzle);

        if bound > depth {
            return false;
        }

        use Direction::*;
        for d in [Up, Left, Down, Right] {
            if self.stack.top() == Some(d.inverse()) {
                continue;
            }

            if !self.puzzle.try_move_dir(d) {
                continue;
            }

            self.stack.push(d);

            if self.dfs(depth - T::one()) {
                return true;
            }

            self.stack.pop();
            self.puzzle.try_move_dir(d.inverse());
        }

        false
    }

    /// Solves the puzzle.
    pub fn solve(&mut self) -> Result<Algorithm, SolverError> {
        let mut depth = self.heuristic.bound(&self.puzzle);
        loop {
            if self.dfs(depth) {
                let mut solution: Algorithm = (&self.stack).into();
                solution.simplify();
                return Ok(solution);
            }

            if let Some(d) = depth.checked_add(&2u8.as_()) {
                depth = d;
            } else {
                return Err(SolverError::NoSolutionFound);
            }
        }
    }
}
