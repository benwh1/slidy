//! Defines the [`GenericSolver`] struct which can optimally solve puzzles with an arbitrary
//! [`SolvedState`] in either the [`Stm`] or [`Mtm`] metric, using an arbitrary [`Heuristic`].

use std::{marker::PhantomData, ops::ControlFlow};

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
        config::SolverConfig,
        heuristic::{manhattan::ManhattanDistance, Heuristic},
        solver::{Solver, SolverError},
        stack::Stack,
        statistics::SolverIterationStats,
    },
};

/// An optimal puzzle solver using a [`Heuristic`] `H` to speed up the search.
pub struct GenericSolver<P, S, H, M> {
    stack: Stack<256>,
    heuristic: H,
    solved_state: S,
    solutions_found: u64,
    config: Option<SolverConfig>,
    phantom_p: PhantomData<P>,
    phantom_m: PhantomData<M>,
}

impl<P: SlidingPuzzle + Clone> Default
    for GenericSolver<P, RowGrids, ManhattanDistance<RowGrids>, Stm>
{
    fn default() -> Self {
        Self::new(ManhattanDistance(RowGrids), RowGrids)
    }
}

impl<P, S, H, M> GenericSolver<P, S, H, M> {
    /// Creates a new [`GenericSolver`] using the given [`Heuristic`] and [`SolvedState`].
    pub fn new(heuristic: H, solved_state: S) -> Self {
        Self {
            stack: Stack::new(),
            heuristic,
            solved_state,
            solutions_found: 0,
            config: None,
            phantom_p: PhantomData,
            phantom_m: PhantomData,
        }
    }
}

impl<P, S, H> Solver<P> for GenericSolver<P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, S, Stm>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> Solver<P> for GenericSolver<P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, S, Mtm>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> GenericSolver<P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, S, Stm>,
{
    fn dfs(&mut self, puzzle: &mut P, depth: u64, last_dir: Option<Direction>) -> ControlFlow<()> {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                self.solutions_found += 1;
                if let Some(f) = &self.config.as_ref().unwrap().solution_callback {
                    if f(self.stack.to_alg()).is_break() {
                        return ControlFlow::Break(());
                    }
                }

                if self.config.as_ref().unwrap().num_solutions == self.solutions_found {
                    return ControlFlow::Break(());
                }
            }

            return ControlFlow::Continue(());
        }

        if self.heuristic.bound(puzzle) > depth {
            return ControlFlow::Continue(());
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

            if self.dfs(puzzle, depth - 1, Some(dir)).is_break() {
                return ControlFlow::Break(());
            }

            self.stack.pop();
            puzzle.try_move_dir(dir.inverse());
        }
        ControlFlow::Continue(())
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;
        let depth_beyond_optimal = config.depth_beyond_optimal;

        // Reset state
        self.stack.clear();
        self.solutions_found = 0;
        self.config = Some(config);

        let mut puzzle = puzzle.clone();

        let hval = self.heuristic.bound(&puzzle);
        let min = if hval % 2 == min % 2 { min } else { min + 1 };
        let mut depth = hval.max(min);

        if depth > max {
            return Ok(());
        }

        let mut first_solution_depth = None;

        loop {
            // Run DFS. This checks against `num_solutions` and the return value of the solution
            // callback.
            if self.dfs(&mut puzzle, depth, None).is_break() {
                break;
            }

            // Set first solution depth.
            if first_solution_depth.is_none() && self.solutions_found > 0 {
                first_solution_depth = Some(depth);
            }

            // Run end of iteration callback and check return value.
            if let Some(f) = &self.config.as_ref().unwrap().end_of_iter_callback {
                if f(SolverIterationStats { depth }).is_break() {
                    break;
                }
            }

            // Go to next depth.
            depth = match depth.checked_add(2) {
                Some(d) => d,
                None => break,
            };

            // Check against `max`.
            if depth > max {
                break;
            }

            // Check against `depth_beyond_optimal`.
            if first_solution_depth.is_some_and(|first| {
                depth_beyond_optimal.is_some_and(|extra| depth - first > extra)
            }) {
                break;
            }
        }

        Ok(())
    }
}

impl<P, S, H> GenericSolver<P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, S, Mtm>,
{
    fn dfs(&mut self, puzzle: &mut P, depth: u64, last_dir: Option<Direction>) -> ControlFlow<()> {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                self.solutions_found += 1;
                if let Some(f) = &self.config.as_ref().unwrap().solution_callback {
                    if f(self.stack.to_alg()).is_break() {
                        return ControlFlow::Break(());
                    }
                }

                if self.config.as_ref().unwrap().num_solutions == self.solutions_found {
                    return ControlFlow::Break(());
                }
            }

            return ControlFlow::Continue(());
        }

        if self.heuristic.bound(puzzle) > depth {
            return ControlFlow::Continue(());
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

            let mut count = 0;

            while puzzle.can_move_dir(dir) {
                puzzle.move_dir(dir);
                count += 1;

                self.stack.push(dir);

                if self.dfs(puzzle, depth - 1, Some(dir)).is_break() {
                    return ControlFlow::Break(());
                }
            }

            if count > 0 {
                puzzle.apply_move(Move::new(dir.inverse(), count));
                self.stack.remove_n(count as usize);
            }
        }

        ControlFlow::Continue(())
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;
        let depth_beyond_optimal = config.depth_beyond_optimal;

        // Reset state
        self.stack.clear();
        self.solutions_found = 0;
        self.config = Some(config);

        let mut puzzle = puzzle.clone();

        let hval = self.heuristic.bound(&puzzle);
        let mut depth = hval.max(min);

        if depth > max {
            return Ok(());
        }

        let mut first_solution_depth = None;

        loop {
            // Run DFS. This checks against `num_solutions` and the return value of the solution
            // callback.
            if self.dfs(&mut puzzle, depth, None).is_break() {
                break;
            }

            // Set first solution depth.
            if first_solution_depth.is_none() && self.solutions_found > 0 {
                first_solution_depth = Some(depth);
            }

            // Run end of iteration callback and check return value.
            if let Some(f) = &self.config.as_ref().unwrap().end_of_iter_callback {
                if f(SolverIterationStats { depth }).is_break() {
                    break;
                }
            }

            // Go to next depth.
            depth = match depth.checked_add(1) {
                Some(d) => d,
                None => break,
            };

            // Check against `max`.
            if depth > max {
                break;
            }

            // Check against `depth_beyond_optimal`.
            if first_solution_depth.is_some_and(|first| {
                depth_beyond_optimal.is_some_and(|extra| depth - first > extra)
            }) {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{ops::ControlFlow, str::FromStr as _};

    use super::*;
    use crate::{
        algorithm::metric::{Mtm, Stm},
        puzzle::{label::label::Rows, puzzle::Puzzle},
        solver::heuristic::mtm::MtmHeuristic,
    };

    #[test]
    fn test_row_grids_manhattan_stm() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 31);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 31);
    }

    #[test]
    fn test_rows_manhattan_stm() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(Rows), Rows);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 23);
    }

    #[test]
    fn test_row_grids_manhattan_mtm() {
        let mut solver: GenericSolver<_, _, _, Mtm> =
            GenericSolver::new(MtmHeuristic(ManhattanDistance(RowGrids)), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 20);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 20);
    }

    #[test]
    fn test_solve_with_bounds_too_low() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 0,
            max: 5,
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 31);
    }

    #[test]
    fn test_solve_with_config() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 31,
            max: 31,
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_stm(), 31);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_config_2() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 0,
            max: 5,
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_config_3() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 20,
            max: 40,
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_stm(), 31);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_config_4() {
        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 33,
            max: 33,
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_stm(), 33);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_solved_state_mtm() {
        let mut solver: GenericSolver<_, _, _, Mtm> =
            GenericSolver::new(MtmHeuristic(ManhattanDistance(Rows)), Rows);
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let config = SolverConfig {
            min: 0,
            max: u64::MAX,
            // The true optimum (13) is verified against the complete projection solver in
            // `projection::mtm::solver::tests`.
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_mtm(), 13);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }
}
