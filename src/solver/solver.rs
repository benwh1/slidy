//! Defines the [`Solver`] struct for computing optimal solutions.

use std::marker::PhantomData;

use num_traits::{AsPrimitive, PrimInt, Unsigned};
use thiserror::Error;

use crate::{
    algorithm::{
        algorithm::Algorithm,
        direction::Direction,
        metric::{Mtm, Stm},
        r#move::r#move::Move,
    },
    puzzle::{
        label::label::RowGrids, sliding_puzzle::SlidingPuzzle, solvable::Solvable,
        solved_state::SolvedState,
    },
    solver::{
        heuristic::{manhattan::ManhattanDistance, Heuristic},
        statistics::SolverIterationStats,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Stack {
    stack: [Move; 256],
    idx: usize,
}

impl Stack {
    fn push(&mut self, mv: Move) {
        self.stack[self.idx] = mv;
        self.idx += 1;
    }

    fn pop(&mut self) -> Move {
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
            stack: [Move::new(Direction::Up, 1); 256],
            idx: 0,
        }
    }
}

impl From<&Stack> for Algorithm {
    fn from(stack: &Stack) -> Self {
        Self::with_moves(stack.stack[..stack.idx].to_vec())
    }
}

/// Error type for [`Solver`].
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

/// An optimal puzzle solver using a [`Heuristic`] `H` to speed up the search.
///
/// The type parameter `T` should be chosen such that the maximum length of a potential solution is
/// less than the maximum value of a `T`. In almost all cases, `T = u8` should be used.
#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub struct Solver<'a, P, T, S, H, M> {
    stack: Stack,
    heuristic: &'a H,
    solved_state: &'a S,
    _phantom: PhantomData<fn(P) -> fn(T) -> fn(M)>,
}

impl<'a, P: SlidingPuzzle + Clone> Default
    for Solver<'a, P, u8, RowGrids, ManhattanDistance<'a, RowGrids>, Stm>
{
    fn default() -> Self {
        Self::new(&ManhattanDistance(&RowGrids), &RowGrids)
    }
}

impl<'a, P, T, S, H, M> Solver<'a, P, T, S, H, M> {
    /// Creates a new [`Solver`] using the given heuristic and solved state.
    pub fn new(heuristic: &'a H, solved_state: &'a S) -> Self {
        Self {
            stack: Stack::default(),
            heuristic,
            solved_state,
            _phantom: PhantomData,
        }
    }
}

impl<'a, P, T, S, H> Solver<'a, P, T, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    T: PrimInt + Unsigned + 'static,
    S: SolvedState + Solvable,
    H: Heuristic<P, T, S, Stm>,
    u8: AsPrimitive<T>,
{
    /// Creates a new [`Solver`] using the given heuristic and solved state.
    pub fn new_with_t(heuristic: &'a H, solved_state: &'a S) -> Self {
        Self {
            stack: Stack::default(),
            heuristic,
            solved_state,
            _phantom: PhantomData,
        }
    }

    fn dfs(&mut self, puzzle: &mut P, depth: T, last_dir: Option<Direction>) -> bool {
        if depth == T::zero() {
            return self.solved_state.is_solved(puzzle);
        }

        if self.heuristic.bound(puzzle) > depth {
            return false;
        }

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_dir == Some(dir.inverse()) {
                continue;
            }

            if !puzzle.try_move_dir(dir) {
                continue;
            }

            self.stack.push(Move::new(dir, 1));
            if self.dfs(puzzle, depth - T::one(), Some(dir)) {
                return true;
            }
            self.stack.pop();
            puzzle.try_move_dir(dir.inverse());
        }
        false
    }

    fn solve_impl(
        &mut self,
        puzzle: &P,
        min: T,
        max: T,
        iteration_callback: Option<&dyn Fn(SolverIterationStats)>,
    ) -> Result<Algorithm, SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        self.stack.clear();
        let mut puzzle = puzzle.clone();
        let mut depth = min;
        loop {
            if self.dfs(&mut puzzle, depth, None) {
                let mut solution: Algorithm = (&self.stack).into();
                solution.simplify();
                return Ok(solution);
            }

            if let Some(f) = iteration_callback {
                f(SolverIterationStats {
                    depth: depth.to_u8().unwrap(),
                });
            }

            depth = depth
                .checked_add(&2u8.as_())
                .filter(|&d| d <= max)
                .ok_or(SolverError::NoSolutionFound)?;
        }
    }

    /// Solves `puzzle`.
    pub fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError> {
        let min = self.heuristic.bound(puzzle);
        self.solve_impl(puzzle, min, T::max_value(), None)
    }

    /// Solves `puzzle` with explicit depth bounds.
    pub fn solve_with_bounds(
        &mut self,
        puzzle: &P,
        min: T,
        max: T,
    ) -> Result<Algorithm, SolverError> {
        self.solve_impl(puzzle, min, max, None)
    }

    /// See [`Solver::solve`].
    ///
    /// Runs `callback` after each iteration of the depth-first search.
    pub fn solve_with_callback(
        &mut self,
        puzzle: &P,
        callback: &dyn Fn(SolverIterationStats),
    ) -> Result<Algorithm, SolverError> {
        let min = self.heuristic.bound(puzzle);
        self.solve_impl(puzzle, min, T::max_value(), Some(callback))
    }
}

impl<'a, P, T, S, H> Solver<'a, P, T, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    T: PrimInt + Unsigned + 'static,
    S: SolvedState + Solvable,
    H: Heuristic<P, T, S, Mtm>,
    u8: AsPrimitive<T>,
{
    /// Creates a new [`Solver`] using the given heuristic and solved state.
    pub fn new_with_t(heuristic: &'a H, solved_state: &'a S) -> Self {
        Self {
            stack: Stack::default(),
            heuristic,
            solved_state,
            _phantom: PhantomData,
        }
    }

    fn dfs(&mut self, puzzle: &mut P, depth: T, last_dir: Option<Direction>) -> bool {
        if depth == T::zero() {
            return self.solved_state.is_solved(puzzle);
        }

        if self.heuristic.bound(puzzle) > depth {
            return false;
        }

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_dir.is_some_and(|ld| dir.axis() == ld.axis()) {
                continue;
            }

            let mut count = 0u64;
            while puzzle.can_move_dir(dir) {
                puzzle.move_dir(dir);
                count += 1;
                self.stack.push(Move::new(dir, count));
                if self.dfs(puzzle, depth - T::one(), Some(dir)) {
                    return true;
                }
                self.stack.pop();
            }
            if count > 0 {
                puzzle.apply_move(Move::new(dir.inverse(), count));
            }
        }
        false
    }

    fn solve_impl(
        &mut self,
        puzzle: &P,
        min: T,
        max: T,
        iteration_callback: Option<&dyn Fn(SolverIterationStats)>,
    ) -> Result<Algorithm, SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        self.stack.clear();
        let mut puzzle = puzzle.clone();
        let mut depth = min;
        loop {
            if self.dfs(&mut puzzle, depth, None) {
                let mut solution: Algorithm = (&self.stack).into();
                solution.simplify();
                return Ok(solution);
            }

            if let Some(f) = iteration_callback {
                f(SolverIterationStats {
                    depth: depth.to_u8().unwrap(),
                });
            }

            depth = depth
                .checked_add(&T::one())
                .filter(|&d| d <= max)
                .ok_or(SolverError::NoSolutionFound)?;
        }
    }

    /// Solves `puzzle`.
    pub fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError> {
        let min = self.heuristic.bound(puzzle);
        self.solve_impl(puzzle, min, T::max_value(), None)
    }

    /// Solves `puzzle` with explicit depth bounds.
    pub fn solve_with_bounds(
        &mut self,
        puzzle: &P,
        min: T,
        max: T,
    ) -> Result<Algorithm, SolverError> {
        self.solve_impl(puzzle, min, max, None)
    }

    /// See [`Solver::solve`].
    ///
    /// Runs `callback` after each iteration of the depth-first search.
    pub fn solve_with_callback(
        &mut self,
        puzzle: &P,
        callback: &dyn Fn(SolverIterationStats),
    ) -> Result<Algorithm, SolverError> {
        let min = self.heuristic.bound(puzzle);
        self.solve_impl(puzzle, min, T::max_value(), Some(callback))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use crate::{
        algorithm::metric::{Mtm, Stm},
        puzzle::{label::label::Rows, puzzle::Puzzle},
    };

    use super::*;

    #[test]
    fn test_row_grids_manhattan_stm() {
        let mut solver: Solver<'_, Puzzle, u8, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            Solver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_rows_manhattan_stm() {
        let mut solver: Solver<'_, Puzzle, u8, Rows, ManhattanDistance<'_, Rows>, Stm> =
            Solver::new(&ManhattanDistance(&Rows), &Rows);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 23);
    }

    #[test]
    fn test_row_grids_manhattan_mtm() {
        let mut solver: Solver<'_, Puzzle, u8, RowGrids, ManhattanDistance<'_, RowGrids>, Mtm> =
            Solver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 24);
    }

    #[test]
    fn test_solve_with_bounds_too_low() {
        let mut solver: Solver<'_, Puzzle, u8, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            Solver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let result = solver.solve_with_bounds(&puzzle, 0, 5);
        assert_eq!(result, Err(SolverError::NoSolutionFound));
    }

    #[test]
    fn test_solve_with_bounds_exact() {
        let mut solver: Solver<'_, Puzzle, u8, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            Solver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve_with_bounds(&puzzle, 31, 31).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }
}
