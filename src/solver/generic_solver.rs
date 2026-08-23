//! Defines the [`GenericSolver`] struct which can optimally solve puzzles with an arbitrary
//! [`SolvedState`] in either the [`Stm`] or [`Mtm`] metric, using an arbitrary [`Heuristic`].

use std::{cell::Cell, marker::PhantomData};

use crate::{
    algorithm::{
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
        stack::Stack,
        statistics::SolverIterationStats,
    },
};

/// An optimal puzzle solver using a [`Heuristic`] `H` to speed up the search.
pub struct GenericSolver<'a, P, S, H, M> {
    stack: Stack<256>,
    heuristic: &'a H,
    solved_state: &'a S,
    solutions_found: Cell<u64>,
    config: Option<SolverConfig>,
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
            solutions_found: Cell::new(0),
            config: None,
            phantom_p: PhantomData,
            phantom_m: PhantomData,
        }
    }
}

impl<P, S, H> Solver<P, u8, S, H, Stm> for GenericSolver<'_, P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Stm>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> Solver<P, u8, S, H, Mtm> for GenericSolver<'_, P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Mtm>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> GenericSolver<'_, P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Stm>,
{
    fn dfs(&self, puzzle: &mut P, depth: u8, last_dir: Option<Direction>) -> bool {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                if let Some(f) = self
                    .config
                    .as_ref()
                    .and_then(|c| c.solution_callback.as_ref())
                {
                    self.solutions_found.update(|n| n + 1);
                    f(self.stack.to_alg())
                }

                return self.config.as_ref().unwrap().num_solutions == self.solutions_found.get();
            } else {
                return false;
            }
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

            self.stack.push(dir);

            if self.dfs(puzzle, depth - 1, Some(dir)) {
                return true;
            }

            self.stack.pop();
            puzzle.try_move_dir(dir.inverse());
        }
        false
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        // Reset state
        self.stack.clear();
        self.solutions_found.set(0);
        self.config = Some(config);

        let config = self.config.as_ref().unwrap();

        let mut puzzle = puzzle.clone();

        let start_heuristic = self.heuristic.bound(&puzzle);
        let min = if start_heuristic % 2 == config.min % 2 {
            config.min
        } else {
            config.min + 1
        };

        let mut depth = start_heuristic.max(min);

        while depth <= config.max {
            if self.dfs(&mut puzzle, depth, None) {
                return Ok(());
            }

            if let Some(f) = &config.end_of_iter_callback {
                f(SolverIterationStats { depth });
            }

            depth = match depth.checked_add(2) {
                Some(d) => d,
                None => break,
            };
        }

        Err(SolverError::NoSolutionFound)
    }
}

impl<P, S, H> GenericSolver<'_, P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Mtm>,
{
    fn dfs(&self, puzzle: &mut P, depth: u8, last_dir: Option<Direction>) -> bool {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                if let Some(f) = self
                    .config
                    .as_ref()
                    .and_then(|c| c.solution_callback.as_ref())
                {
                    self.solutions_found.update(|n| n + 1);
                    f(self.stack.to_alg())
                }

                return self.config.as_ref().unwrap().num_solutions == self.solutions_found.get();
            } else {
                return false;
            }
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

                self.stack.push(dir);

                if self.dfs(puzzle, depth - 1, Some(dir)) {
                    return true;
                }
            }

            if count > 0 {
                puzzle.apply_move(Move::new(dir.inverse(), count));
                self.stack.remove_n(count as usize);
            }
        }

        false
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        // Reset state
        self.stack.clear();
        self.solutions_found.set(0);
        self.config = Some(config);

        let config = self.config.as_ref().unwrap();

        let mut puzzle = puzzle.clone();
        let mut depth = config.min;

        while depth <= config.max {
            if self.dfs(&mut puzzle, depth, None) {
                return Ok(());
            }

            if let Some(f) = &config.end_of_iter_callback {
                f(SolverIterationStats { depth });
            }

            depth = match depth.checked_add(1) {
                Some(d) => d,
                None => break,
            };
        }

        Err(SolverError::NoSolutionFound)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::{
        algorithm::metric::{Mtm, Stm},
        puzzle::{label::label::Rows, puzzle::Puzzle},
    };

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
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Err(SolverError::NoSolutionFound));
    }

    #[test]
    fn test_solve() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = Solver::solve(&mut solver, &puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_solve_with_config() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 31,
            max: 31,
            solution_callback: Some(Box::new(|s| assert_eq!(s.len_stm::<u64>(), 31))),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_config_2() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 0,
            max: 5,
            ..Default::default()
        };
        let result = Solver::solve_with_config(&mut solver, &puzzle, config);
        assert_eq!(result, Err(SolverError::NoSolutionFound));
    }

    #[test]
    fn test_solve_with_config_3() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 20,
            max: 40,
            solution_callback: Some(Box::new(|s| assert_eq!(s.len_stm::<u64>(), 31))),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_config_4() {
        let mut solver: GenericSolver<'_, Puzzle, RowGrids, ManhattanDistance<'_, RowGrids>, Stm> =
            GenericSolver::new(&ManhattanDistance(&RowGrids), &RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 33,
            max: 33,
            solution_callback: Some(Box::new(|s| assert_eq!(s.len_stm::<u64>(), 33))),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_solved_state_mtm() {
        let mut solver: GenericSolver<'_, Puzzle, Rows, ManhattanDistance<'_, Rows>, Mtm> =
            GenericSolver::new(&ManhattanDistance(&Rows), &Rows);
        let puzzle = Puzzle::from_str("2 7 11 1/5 9 3 14/15 10 6 12/4 0 8 13").unwrap();
        let config = SolverConfig {
            min: 0,
            max: u8::MAX,
            solution_callback: Some(Box::new(|s| assert_eq!(s.len_mtm::<u64>(), 21))),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }
}
