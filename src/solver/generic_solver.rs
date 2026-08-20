//! Defines the [`GenericSolver`] struct which can optimally solve puzzles with an arbitrary
//! [`SolvedState`] in either the [`Stm`] or [`Mtm`] metric, using an arbitrary [`Heuristic`].

use std::marker::PhantomData;

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
        solver::{Solver, SolverConfig, SolverError},
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

/// An optimal puzzle solver using a [`Heuristic`] `H` to speed up the search.
#[derive(Clone, Debug)]
pub struct GenericSolver<'a, P, S, H, M> {
    stack: Stack,
    heuristic: &'a H,
    solved_state: &'a S,
    initialized: bool,
    phantom_p: PhantomData<P>,
    phantom_m: PhantomData<M>,
}

impl<'a, P: SlidingPuzzle + Clone> Default
    for GenericSolver<'a, P, RowGrids, ManhattanDistance<'a, RowGrids>, Stm>
{
    fn default() -> Self {
        Self::new(&ManhattanDistance(&RowGrids), &RowGrids)
    }
}

impl<'a, P, S, H, M> GenericSolver<'a, P, S, H, M> {
    /// Creates a new [`GenericSolver`] using the given [`Heuristic`] and [`SolvedState`].
    pub fn new(heuristic: &'a H, solved_state: &'a S) -> Self {
        Self {
            stack: Stack::default(),
            heuristic,
            solved_state,
            initialized: false,
            phantom_p: PhantomData,
            phantom_m: PhantomData,
        }
    }
}

impl<'a, P, S, H> Solver<Stm> for GenericSolver<'a, P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Stm>,
{
    type Puzzle = P;
    type Heuristic = H;
    type SolvedState = S;

    fn is_initialised(&self) -> bool {
        self.initialized
    }

    fn init(&mut self) {
        self.initialized = true;
    }

    fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError> {
        let min = self.heuristic.bound(puzzle);
        let config = SolverConfig {
            min,
            ..Default::default()
        };
        self.solve_with_config(puzzle, config)
    }

    fn solve_with_config(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<Algorithm, SolverError> {
        if !self.initialized {
            self.init();
        }
        self.solve_impl(puzzle, config.min, config.max, config.callback)
    }
}

impl<'a, P, S, H> Solver<Mtm> for GenericSolver<'a, P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Mtm>,
{
    type Puzzle = P;
    type Heuristic = H;
    type SolvedState = S;

    fn is_initialised(&self) -> bool {
        self.initialized
    }

    fn init(&mut self) {
        self.initialized = true;
    }

    fn solve(&mut self, puzzle: &P) -> Result<Algorithm, SolverError> {
        let min = self.heuristic.bound(puzzle);
        let config = SolverConfig {
            min,
            ..Default::default()
        };
        self.solve_with_config(puzzle, config)
    }

    fn solve_with_config(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<Algorithm, SolverError> {
        if !self.initialized {
            self.init();
        }
        self.solve_impl(puzzle, config.min, config.max, config.callback)
    }
}

impl<'a, P, S, H> GenericSolver<'a, P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Stm>,
{
    fn dfs(&mut self, puzzle: &mut P, depth: u8, last_dir: Option<Direction>) -> bool {
        if depth == 0 {
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
            if self.dfs(puzzle, depth - 1, Some(dir)) {
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
        min: u8,
        max: u8,
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
                f(SolverIterationStats { depth });
            }

            depth = depth
                .checked_add(2)
                .filter(|&d| d <= max)
                .ok_or(SolverError::NoSolutionFound)?;
        }
    }
}

impl<'a, P, S, H> GenericSolver<'a, P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Mtm>,
{
    fn dfs(&mut self, puzzle: &mut P, depth: u8, last_dir: Option<Direction>) -> bool {
        if depth == 0 {
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
                if self.dfs(puzzle, depth - 1, Some(dir)) {
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
        min: u8,
        max: u8,
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
                f(SolverIterationStats { depth });
            }

            depth = depth
                .checked_add(1)
                .filter(|&d| d <= max)
                .ok_or(SolverError::NoSolutionFound)?;
        }
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
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_rows_manhattan_stm() {
        let mut solver: GenericSolver<'_, Puzzle, Rows, ManhattanDistance<'_, Rows>, Stm> =
            GenericSolver::new(&ManhattanDistance(&Rows), &Rows);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 23);
    }

    #[test]
    fn test_row_grids_manhattan_mtm() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Mtm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 24);
    }

    #[test]
    fn test_solve_with_bounds_too_low() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 0,
            max: 5,
            callback: None,
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Err(SolverError::NoSolutionFound));
    }

    #[test]
    fn test_solve_with_bounds_exact() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 31,
            max: 31,
            callback: None,
        };
        let result = solver.solve_with_config(&puzzle, config);
        let solution = result.unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_solve_trait_auto_init() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        assert!(!solver.is_initialised());
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = Solver::solve(&mut solver, &puzzle).unwrap();
        assert!(solver.is_initialised());
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_solve_trait_with_config() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 31,
            max: 31,
            callback: None,
        };
        let solution = Solver::solve_with_config(&mut solver, &puzzle, config).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_solve_trait_bounds_too_low() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 0,
            max: 5,
            callback: None,
        };
        let result = Solver::solve_with_config(&mut solver, &puzzle, config);
        assert_eq!(result, Err(SolverError::NoSolutionFound));
    }
}
