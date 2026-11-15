//! Defines the [`GenericSolver`] struct which can optimally solve puzzles with an arbitrary
//! [`SolvedState`] in either the [`Stm`] or [`Mtm`] metric, using an arbitrary [`Heuristic`].

use std::{marker::PhantomData, ops::ControlFlow};

use num_traits::Zero as _;

use crate::{
    algorithm::{
        direction::Direction,
        metric::{Mtm, Stm},
    },
    puzzle::{
        label::label::RowGrids, sliding_puzzle::SlidingPuzzle, solvable::Solvable,
        solved_state::SolvedState,
    },
    solver::{
        config::SolverConfig,
        heuristic::{Heuristic, manhattan::ManhattanDistance},
        search::{Search, iterative_deepening},
        solver::{Solver, SolverError},
    },
};

/// The directions to search, in order.
const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
];

/// Returns the position of the gap after moving `count` pieces in direction `dir`, where the gap
/// is at `gap`, or `None` if the move isn't possible.
fn gap_after_move<P>(
    puzzle: &P,
    (gx, gy): (u64, u64),
    dir: Direction,
    count: u64,
) -> Option<(u64, u64)>
where
    P: SlidingPuzzle,
{
    let (w, h) = puzzle.size().into();
    // The gap moves in the opposite direction to the pieces.
    match dir {
        Direction::Up => Some((gx, gy + count)).filter(|&(_, y)| y < h),
        Direction::Left => Some((gx + count, gy)).filter(|&(x, _)| x < w),
        Direction::Down => Some((gx, gy.checked_sub(count)?)),
        Direction::Right => Some((gx.checked_sub(count)?, gy)),
    }
}

/// An optimal puzzle solver using a [`Heuristic`] `H` to speed up the search.
pub struct GenericSolver<P, S, H, M> {
    search: Search<256>,
    heuristic: H,
    solved_state: S,
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
            search: Search::new(),
            heuristic,
            solved_state,
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
    type Context = ();

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
        _context: &Self::Context,
    ) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> Solver<P> for GenericSolver<P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, S, Mtm>,
{
    type Context = ();

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
        _context: &Self::Context,
    ) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

impl<P, S, H> GenericSolver<P, S, H, Stm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, S, Stm>,
{
    /// Finds solutions of length `depth` of `puzzle`, which has the gap at `gap` and heuristic
    /// state `state`, and was reached by moving in direction `last_dir`.
    fn dfs(
        &mut self,
        puzzle: &mut P,
        state: &mut H::State,
        depth: u64,
        gap: (u64, u64),
        last_dir: Option<Direction>,
    ) -> ControlFlow<()> {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                return self.search.record_solution();
            }

            return ControlFlow::Continue(());
        }

        for dir in DIRECTIONS {
            if last_dir == Some(dir.inverse()) {
                continue;
            }

            let Some(next_gap) = gap_after_move(puzzle, gap, dir, 1) else {
                continue;
            };

            puzzle.move_dir(dir);

            let next_bound = self
                .heuristic
                .bound_after_move(state, puzzle, next_gap, gap);
            // Children at depth 0 are only checked for being solved, not against the bound.
            if depth == 1 || next_bound < depth {
                self.search.stack.push(dir);

                if self
                    .dfs(puzzle, state, depth - 1, next_gap, Some(dir))
                    .is_break()
                {
                    return ControlFlow::Break(());
                }

                self.search.stack.pop();
            }

            puzzle.move_dir(dir.inverse());
            self.heuristic.undo_move(state, puzzle, next_gap, gap);
        }

        ControlFlow::Continue(())
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let mut puzzle = puzzle.clone();
        let hval = self.heuristic.bound(&puzzle);

        // The search only tries depths with the same parity as the lower bound. Every move moves
        // the gap by one step, so every solution has the parity of the gap's distance from its
        // solved position, but the heuristic's bound might not. Round it up to that parity.
        let (gx, gy) = puzzle.gap_position_xy();
        let (sx, sy) = puzzle.solved_pos_xy(P::Piece::zero());
        let parity = (gx.abs_diff(sx) + gy.abs_diff(sy)) % 2;
        let lower_bound = if hval % 2 == parity { hval } else { hval + 1 };

        let mut state = self.heuristic.init(&puzzle);
        iterative_deepening::<Stm, _, _>(
            self,
            |s| &mut s.search,
            config,
            lower_bound,
            |s, depth| s.dfs(&mut puzzle, &mut state, depth, (gx, gy), None),
        );

        Ok(())
    }
}

impl<P, S, H> GenericSolver<P, S, H, Mtm>
where
    P: SlidingPuzzle + Clone,
    S: SolvedState + Solvable,
    H: Heuristic<P, S, Mtm>,
{
    /// Finds solutions of length `depth` of `puzzle`, which has the gap at `gap` and heuristic
    /// state `state`, and was reached by moving in direction `last_dir`.
    fn dfs(
        &mut self,
        puzzle: &mut P,
        state: &mut H::State,
        depth: u64,
        gap: (u64, u64),
        last_dir: Option<Direction>,
    ) -> ControlFlow<()> {
        if depth == 0 {
            if self.solved_state.is_solved(puzzle) {
                return self.search.record_solution();
            }

            return ControlFlow::Continue(());
        }

        for dir in DIRECTIONS {
            if last_dir.is_some_and(|ld| dir.axis() == ld.axis()) {
                continue;
            }

            // Move one more piece at a time, so that each move of `count` pieces is reached from
            // the move of `count - 1`.
            let mut count = 0;
            let mut prev_gap = gap;
            while let Some(next_gap) = gap_after_move(puzzle, gap, dir, count + 1) {
                puzzle.move_dir(dir);
                count += 1;
                self.search.stack.push(dir);

                let next_bound = self
                    .heuristic
                    .bound_after_move(state, puzzle, next_gap, prev_gap);
                // Children at depth 0 are only checked for being solved, not against the bound.
                if (depth == 1 || next_bound < depth)
                    && self
                        .dfs(puzzle, state, depth - 1, next_gap, Some(dir))
                        .is_break()
                {
                    return ControlFlow::Break(());
                }

                prev_gap = next_gap;
            }

            // Undo the single moves in reverse order.
            for i in (1..=count).rev() {
                puzzle.move_dir(dir.inverse());
                let from = gap_after_move(puzzle, gap, dir, i).unwrap();
                let to = gap_after_move(puzzle, gap, dir, i - 1).unwrap();
                self.heuristic.undo_move(state, puzzle, from, to);
            }
            self.search.stack.remove_n(count as usize);
        }

        ControlFlow::Continue(())
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if !self.solved_state.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let mut puzzle = puzzle.clone();
        let hval = self.heuristic.bound(&puzzle);
        let gap = puzzle.gap_position_xy();

        let mut state = self.heuristic.init(&puzzle);
        iterative_deepening::<Mtm, _, _>(
            self,
            |s| &mut s.search,
            config,
            hval,
            |s, depth| s.dfs(&mut puzzle, &mut state, depth, gap, None),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{ops::ControlFlow, str::FromStr as _};

    use super::*;
    use crate::{
        algorithm::{
            algorithm::Algorithm,
            metric::{Mtm, Stm},
        },
        puzzle::{label::label::Rows, puzzle::Puzzle, size::Size},
        solver::{
            config::SolverConfig,
            heuristic::{inversion::InversionCount, max::Max, mtm::MtmHeuristic},
        },
    };

    #[test]
    fn test_trivial_heuristic_stm_odd_solution() {
        // The `()` heuristic always gives an even bound, but this puzzle needs an odd number of
        // moves.
        let mut solver: GenericSolver<_, _, _, Stm> = GenericSolver::new((), RowGrids);
        let puzzle = Puzzle::from_str("1 2 3/4 5 6/7 0 8").unwrap();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution, Algorithm::from_str("L").unwrap());

        let mut puzzle = Puzzle::new(Size::new(3, 3).unwrap());
        puzzle.apply_alg(&Algorithm::from_str("DRD").unwrap());
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 3);
        assert!(solution.is_solution_of(puzzle));
    }

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
    fn test_row_grids_inversion_count() {
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();

        let mut solver: GenericSolver<_, _, _, Stm> = GenericSolver::new(InversionCount, RowGrids);
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 31);
        assert!(solution.is_solution_of(puzzle.clone()));

        let mut solver: GenericSolver<_, _, _, Mtm> =
            GenericSolver::new(MtmHeuristic(InversionCount), RowGrids);
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 20);
    }

    #[test]
    fn test_row_grids_max_heuristic() {
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();

        let mut solver: GenericSolver<_, _, _, Stm> =
            GenericSolver::new(Max((ManhattanDistance(RowGrids), InversionCount)), RowGrids);
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 31);
        assert!(solution.is_solution_of(puzzle.clone()));

        let mut solver: GenericSolver<_, _, _, Mtm> = GenericSolver::new(
            MtmHeuristic(Max((ManhattanDistance(RowGrids), InversionCount))),
            RowGrids,
        );
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 20);
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

    #[test]
    fn test_min_deeper_than_initial_stack_capacity() {
        let mut solver = GenericSolver::default();
        let mut puzzle = Puzzle::from_str("1 2/0 3").unwrap();
        let config = SolverConfig {
            min: 300,
            ..Default::default()
        };

        let solutions = solver.solve_collect(&puzzle, config).unwrap();
        assert_eq!(solutions.len(), 1);

        // Solutions have length `1 + 12k` or `11 + 12k`, see the 2x2 small solver test.
        let solution = &solutions[0];
        assert_eq!(solution.len_stm(), 301);
        puzzle.apply_alg(solution);
        assert!(puzzle.is_solved());
    }

    #[test]
    fn test_zero_solutions() {
        let mut solver = GenericSolver::default();
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(solver.solve_collect(&puzzle, config), Ok(Vec::new()));

        let unsolvable = Puzzle::from_str("2 1 3/4 5 6/7 8 0").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(
            solver.solve_collect(&unsolvable, config),
            Err(SolverError::Unsolvable)
        );
    }

    #[test]
    fn test_zero_solutions_mtm() {
        let mut solver =
            GenericSolver::<_, _, _, Mtm>::new(MtmHeuristic(ManhattanDistance(RowGrids)), RowGrids);
        let puzzle = Puzzle::from_str("8 6 7/2 5 4/3 0 1").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(solver.solve_collect(&puzzle, config), Ok(Vec::new()));

        let unsolvable = Puzzle::from_str("2 1 3/4 5 6/7 8 0").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(
            solver.solve_collect(&unsolvable, config),
            Err(SolverError::Unsolvable)
        );
    }
}
