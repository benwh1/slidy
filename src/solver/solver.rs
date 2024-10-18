//! Defines the [`Solver`] struct for computing optimal solutions.

use std::marker::PhantomData;

use num_traits::{AsPrimitive, PrimInt, Unsigned};
use thiserror::Error;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction, r#move::r#move::Move},
    puzzle::{label::label::RowGrids, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::heuristic::{manhattan::ManhattanDistance, Heuristic},
};

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

    fn clear(&mut self) {
        self.idx = 0;
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
        Self::with_moves(
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

/// An optimal puzzle solver using a [`Heuristic`] `H` to speed up the search.
///
/// The type parameter `T` should be chosen such that the maximum length of a potential solution is
/// less than the maximum value of a `T`. In almost all cases, `T = u8` should be used.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Solver<'a, Puzzle, T, S, H>
where
    Puzzle: SlidingPuzzle + Clone,
    T: PrimInt + Unsigned + 'static,
    S: SolvedState,
    H: Heuristic<Puzzle, T>,
    u8: AsPrimitive<T>,
{
    stack: Stack,
    phantom_puzzle: PhantomData<Puzzle>,
    heuristic: &'a H,
    solved_state: &'a S,
    phantom_t: PhantomData<T>,
}

impl<Puzzle> Default for Solver<'static, Puzzle, u8, RowGrids, ManhattanDistance<'static, RowGrids>>
where
    Puzzle: SlidingPuzzle + Clone,
{
    fn default() -> Self {
        Self::new_with_t(&ManhattanDistance(&RowGrids), &RowGrids)
    }
}

impl<'a, Puzzle, S, H> Solver<'a, Puzzle, u8, S, H>
where
    Puzzle: SlidingPuzzle + Clone,
    S: SolvedState,
    H: Heuristic<Puzzle, u8>,
{
    /// Creates a new [`Solver`] using the given heuristic.
    pub fn new(heuristic: &'a H, solved_state: &'a S) -> Self {
        Self {
            stack: Stack::default(),
            phantom_puzzle: PhantomData,
            heuristic,
            solved_state,
            phantom_t: PhantomData,
        }
    }
}

impl<'a, Puzzle, T, S, H> Solver<'a, Puzzle, T, S, H>
where
    Puzzle: SlidingPuzzle + Clone,
    T: PrimInt + Unsigned + 'static,
    S: SolvedState,
    H: Heuristic<Puzzle, T>,
    u8: AsPrimitive<T>,
{
    /// Constructs a new [`Solver`] for solving `puzzle`.
    pub fn new_with_t(heuristic: &'a H, solved_state: &'a S) -> Self {
        Self {
            stack: Stack::default(),
            phantom_puzzle: PhantomData,
            heuristic,
            solved_state,
            phantom_t: PhantomData::<T>,
        }
    }

    fn dfs(&mut self, puzzle: &mut Puzzle, depth: T) -> bool {
        if depth == T::zero() {
            return self.solved_state.is_solved(puzzle);
        }

        let bound = self.heuristic.bound(puzzle);

        if bound > depth {
            return false;
        }

        for d in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            if self.stack.top() == Some(d.inverse()) {
                continue;
            }

            if !puzzle.try_move_dir(d) {
                continue;
            }

            self.stack.push(d);

            if self.dfs(puzzle, depth - T::one()) {
                return true;
            }

            self.stack.pop();
            puzzle.try_move_dir(d.inverse());
        }

        false
    }

    /// Solves `puzzle`.
    pub fn solve(&mut self, puzzle: &Puzzle) -> Result<Algorithm, SolverError> {
        self.stack.clear();
        let mut puzzle = puzzle.clone();
        let mut depth = self.heuristic.bound(&puzzle);
        loop {
            if self.dfs(&mut puzzle, depth) {
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::puzzle::{label::label::Rows, puzzle::Puzzle};

    use super::*;

    #[test]
    fn test_row_grids_manhattan() {
        let mut solver = Solver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_rows_manhattan() {
        let mut solver = Solver::new(&ManhattanDistance(&Rows), &Rows);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 23);
    }
}
