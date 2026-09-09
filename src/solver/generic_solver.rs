//! Defines the [`GenericSolver`] struct which can optimally solve puzzles with an arbitrary
//! [`SolvedState`] in either the [`Stm`] or [`Mtm`] metric, using an arbitrary [`Heuristic`].

use std::{
    cell::{Cell, Ref, RefCell},
    marker::PhantomData,
};

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
pub struct GenericSolver<P, S, H, M> {
    stack: Stack<256>,
    heuristic: H,
    solved_state: S,
    solutions_found: Cell<u64>,
    config: RefCell<Option<SolverConfig>>,
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
            stack: Stack::default(),
            heuristic,
            solved_state,
            solutions_found: Cell::new(0),
            config: RefCell::new(None),
            phantom_p: PhantomData,
            phantom_m: PhantomData,
        }
    }

    fn cfg(&self) -> Ref<'_, SolverConfig> {
        let borrow = self.config.borrow();
        Ref::map(borrow, |b| b.as_ref().unwrap())
    }
}

impl<P, S, H> Solver<P> for GenericSolver<P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Stm>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> Solver<P> for GenericSolver<P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Mtm>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> GenericSolver<P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Stm>,
{
    fn dfs(&self, puzzle: &mut P, depth: u8, last_dir: Option<Direction>) -> bool {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                self.solutions_found.update(|n| n + 1);
                if let Some(f) = &self.cfg().solution_callback {
                    if f(self.stack.to_alg()).is_break() {
                        return true;
                    }
                }

                return self.cfg().num_solutions == self.solutions_found.get();
            }

            return false;
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

    fn solve_impl(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;
        let depth_beyond_optimal = config.depth_beyond_optimal;

        // Reset state
        self.stack.clear();
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let mut puzzle = puzzle.clone();

        let start_heuristic = self.heuristic.bound(&puzzle);
        let min = if start_heuristic % 2 == min % 2 {
            min
        } else {
            min + 1
        };

        let mut depth = start_heuristic.max(min);
        let mut first_solution_depth: Option<u8> = None;

        while depth <= max {
            if first_solution_depth.is_some_and(|fd| {
                depth_beyond_optimal.is_some_and(|e| depth > fd.saturating_add(e))
            }) {
                break;
            }

            let found_before = self.solutions_found.get();
            if self.dfs(&mut puzzle, depth, None) {
                return Ok(());
            }
            if first_solution_depth.is_none() && self.solutions_found.get() > found_before {
                first_solution_depth = Some(depth);
            }

            if let Some(f) = &self.cfg().end_of_iter_callback {
                if f(SolverIterationStats { depth }).is_break() {
                    return Ok(());
                }
            }

            depth = match depth.checked_add(2) {
                Some(d) => d,
                None => break,
            };
        }

        if self.solutions_found.get() > 0 {
            Ok(())
        } else {
            Err(SolverError::NoSolutionFound)
        }
    }
}

impl<P, S, H> GenericSolver<P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, u8, S, Mtm>,
{
    fn dfs(&self, puzzle: &mut P, depth: u8, last_dir: Option<Direction>) -> bool {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                self.solutions_found.update(|n| n + 1);
                if let Some(f) = &self.cfg().solution_callback {
                    if f(self.stack.to_alg()).is_break() {
                        return true;
                    }
                }

                return self.cfg().num_solutions == self.solutions_found.get();
            }

            return false;
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

            let mut count = 0;

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

    fn solve_impl(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;
        let depth_beyond_optimal = config.depth_beyond_optimal;

        // Reset state
        self.stack.clear();
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let mut puzzle = puzzle.clone();
        let mut depth = min;
        let mut first_solution_depth: Option<u8> = None;

        while depth <= max {
            if first_solution_depth.is_some_and(|fd| {
                depth_beyond_optimal.is_some_and(|e| depth > fd.saturating_add(e))
            }) {
                break;
            }

            let found_before = self.solutions_found.get();
            if self.dfs(&mut puzzle, depth, None) {
                return Ok(());
            }
            if first_solution_depth.is_none() && self.solutions_found.get() > found_before {
                first_solution_depth = Some(depth);
            }

            if let Some(f) = &self.cfg().end_of_iter_callback {
                if f(SolverIterationStats { depth }).is_break() {
                    return Ok(());
                }
            }

            depth = match depth.checked_add(1) {
                Some(d) => d,
                None => break,
            };
        }

        if self.solutions_found.get() > 0 {
            Ok(())
        } else {
            Err(SolverError::NoSolutionFound)
        }
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
        let solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_rows_manhattan_stm() {
        let solver: GenericSolver<_, _, _, Stm> = GenericSolver::new(ManhattanDistance(Rows), Rows);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 23);
    }

    #[test]
    fn test_row_grids_manhattan_mtm() {
        let solver: GenericSolver<_, _, _, Mtm> =
            GenericSolver::new(MtmHeuristic(ManhattanDistance(RowGrids)), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 20);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 20);
    }

    #[test]
    fn test_solve_with_bounds_too_low() {
        let solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
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
        let solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 31);
    }

    #[test]
    fn test_solve_with_config() {
        let solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 31,
            max: 31,
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_stm::<u64>(), 31);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_config_2() {
        let solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
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
    fn test_solve_with_config_3() {
        let solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 20,
            max: 40,
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_stm::<u64>(), 31);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_config_4() {
        let solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(ManhattanDistance(RowGrids), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            min: 33,
            max: 33,
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_stm::<u64>(), 33);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_solve_with_solved_state_mtm() {
        let solver: GenericSolver<_, _, _, Mtm> =
            GenericSolver::new(MtmHeuristic(ManhattanDistance(Rows)), Rows);
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let config = SolverConfig {
            min: 0,
            max: u8::MAX,
            // The true optimum (13) is verified against the complete projection solver in
            // `projection::mtm::solver::tests`.
            solution_callback: Some(Box::new(|s| {
                assert_eq!(s.len_mtm::<u64>(), 13);
                ControlFlow::Continue(())
            })),
            ..Default::default()
        };
        let result = solver.solve_with_config(&puzzle, config);
        assert_eq!(result, Ok(()));
    }
}
