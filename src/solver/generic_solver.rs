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
        puzzle::{
            label::label::{
                Checkerboard, ConcentricRectangles, Diagonals, Fringe, LastTwoRows, Rows,
                SplitFringe, SplitLastTwoRows, SplitSquareFringe, SquareFringe, Trivial,
            },
            puzzle::Puzzle,
            size::Size,
        },
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

    /// Indices of solved states in [`OPTIMAL`].
    const TRIVIAL: usize = 0;
    const ROW_GRIDS: usize = 1;
    const ROWS: usize = 2;
    const FRINGE: usize = 3;
    const SQUARE_FRINGE: usize = 4;
    const SPLIT_FRINGE: usize = 5;
    const SPLIT_SQUARE_FRINGE: usize = 6;
    const DIAGONALS: usize = 7;
    const LAST_TWO_ROWS: usize = 8;
    const SPLIT_LAST_TWO_ROWS: usize = 9;
    const CONCENTRIC_RECTANGLES: usize = 10;
    const CHECKERBOARD: usize = 11;

    /// Optimal solution lengths of random puzzles, in [`Stm`] and then [`Mtm`], for each solved
    /// state in the order of the indices above.
    ///
    /// The puzzles were generated with `slidy generate -s WxH -n N --seed 1` for sizes 4x1, 1x4,
    /// 2x2, 3x2, 2x3, 4x2, 2x4 and 3x3, and the lengths were found by a brute-force breadth-first
    /// search from all solved states.
    #[rustfmt::skip]
    const OPTIMAL: &[(&str, [[u64; 12]; 2])] = &[
        ("0 1 2 3",           [[ 3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
        ("1 0 2 3",           [[ 2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
        ("1 2 3 0",           [[ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0], [ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0]]),
        ("0/1/2/3",           [[ 3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3,  3], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
        ("1/0/2/3",           [[ 2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
        ("1/2/3/0",           [[ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0], [ 0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0]]),
        ("3 0/2 1",           [[ 1,  5,  5,  1,  1,  5,  5,  5,  5,  5,  1,  5], [ 1,  5,  5,  1,  1,  5,  5,  5,  5,  5,  1,  5]]),
        ("3 1/0 2",           [[ 1,  3,  3,  1,  1,  3,  3,  3,  3,  3,  1,  3], [ 1,  3,  3,  1,  1,  3,  3,  3,  3,  3,  1,  3]]),
        ("1 0/3 2",           [[ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1], [ 1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1]]),
        ("5 4 0/2 1 3",       [[ 1, 13,  9,  7, 13,  9, 13, 13, 13, 13,  1,  1], [ 1, 11,  9,  6, 10,  9, 10, 11, 11, 11,  1,  1]]),
        ("3 5 4/2 1 0",       [[ 0, 12, 10,  4, 12, 10, 12, 12, 12, 12,  0,  4], [ 0,  8,  8,  4,  8,  8,  8,  8,  8,  8,  0,  4]]),
        ("5 3 2/4 1 0",       [[ 0, 18,  6,  8,  6, 14,  8, 12,  6,  6,  0,  4], [ 0, 16,  4,  7,  4, 12,  7,  8,  4,  4,  0,  4]]),
        ("1 0 4/3 5 2",       [[ 2, 14,  8,  2,  8,  8,  8, 14, 12, 12,  2, 10], [ 2, 13,  7,  2,  6,  7,  7, 11, 10, 10,  2, 10]]),
        ("2 5 3/4 1 0",       [[ 0, 14,  4,  4,  6,  4, 10,  8,  8,  8,  0,  6], [ 0, 12,  4,  4,  4,  4,  8,  8,  7,  7,  0,  5]]),
        ("0 4 2/1 3 5",       [[ 3, 17,  7,  3,  3,  7,  3,  9,  7,  7,  3,  7], [ 2, 14,  6,  2,  2,  6,  3,  7,  6,  6,  2,  6]]),
        ("5 0/3 4/1 2",       [[ 2, 20, 14,  4, 14, 16, 14, 10, 16,  4,  2, 10], [ 1, 20,  9,  3,  9, 12,  9,  7, 12,  3,  1,  7]]),
        ("3 5/4 2/1 0",       [[ 0, 16, 16,  4, 12, 12, 16, 10, 12,  4,  0,  6], [ 0, 16, 12,  4,  8,  8, 12,  8,  8,  4,  0,  4]]),
        ("5 3/2 0/1 4",       [[ 1, 15, 15,  1, 11, 15, 15, 13, 15,  1,  1,  1], [ 1, 12, 12,  1,  8, 12, 12, 12, 12,  1,  1,  1]]),
        ("0 1/2 4/5 3",       [[ 3, 17,  3,  3,  3,  3,  3,  9,  3,  3,  3,  9], [ 2, 14,  3,  2,  2,  2,  3,  7,  2,  2,  2,  6]]),
        ("2 5/3 0/1 4",       [[ 1, 13, 13,  1,  9, 13, 13, 11, 13,  1,  1,  5], [ 1, 11, 11,  1,  7, 11, 11,  9, 11,  1,  1,  4]]),
        ("4 2/0 5/3 1",       [[ 2, 16,  8,  6,  8, 10,  8, 14, 10,  6,  2,  2], [ 2, 14,  6,  6,  6,  9,  6, 10,  9,  6,  2,  2]]),
        ("0 6 1 3/5 7 4 2",   [[ 4, 24, 10, 12, 16, 12, 16, 24, 16, 16,  4,  8], [ 2, 18,  6,  9, 11,  9, 11, 15, 11, 11,  2,  6]]),
        ("4 2 3 7/6 1 0 5",   [[ 1, 25,  7, 11, 15, 17, 15, 19, 19, 19,  1,  7], [ 1, 16,  5,  8,  9, 12,  9, 16, 13, 13,  1,  5]]),
        ("1 2 0 5/7 4 6 3",   [[ 2, 22,  8,  8, 20, 16, 22, 22, 20, 20,  2,  8], [ 2, 14,  7,  6, 14, 11, 14, 14, 14, 14,  2,  7]]),
        ("6 2 5 4/0 1 7 3",   [[ 3, 23,  7,  9, 11, 15, 13, 13, 11, 11,  3, 13], [ 1, 17,  7,  7,  9, 12,  9, 11,  9,  9,  1,  8]]),
        ("0 5 6 1/4 7 3 2",   [[ 4, 26, 10,  8, 22, 14, 22, 24, 26, 26,  4, 14], [ 2, 18,  9,  6, 14, 11, 15, 17, 18, 18,  2, 10]]),
        ("6 1/0 3/5 2/4 7",   [[ 3, 21, 17, 11, 17, 19, 17, 11, 17, 13,  3,  9], [ 2, 17, 14, 10, 13, 13, 14, 10, 13, 10,  2,  6]]),
        ("4 2/3 7/6 1/5 0",   [[ 0, 22, 16, 10, 12, 16, 16, 20, 16, 10,  0, 10], [ 0, 19, 12,  8,  8, 12, 12, 12, 12,  8,  0,  8]]),
        ("1 2/5 0/7 3/6 4",   [[ 2,  8,  8,  8,  8,  8,  8,  8,  8,  8,  2,  8], [ 1,  5,  5,  5,  5,  5,  5,  5,  5,  5,  1,  5]]),
        ("6 2/5 4/1 7/0 3",   [[ 1, 23, 21, 11, 21, 19, 21, 23, 21,  9,  1,  9], [ 1, 14, 14,  9, 14, 14, 14, 14, 14,  7,  1,  6]]),
        ("5 6/0 1/4 2/3 7",   [[ 3, 23, 19, 11, 19, 17, 19, 23, 21, 11,  3,  7], [ 2, 15, 15,  9, 14, 14, 15, 15, 15,  9,  2,  6]]),
        ("7 0 1/2 5 3/6 8 4", [[ 3, 21, 13, 11, 11, 21, 21, 19, 19, 15,  3, 11], [ 2, 18, 10,  6,  6, 15, 15, 14, 14, 10,  2,  9]]),
        ("1 2 8/5 4 7/3 6 0", [[ 0, 24, 16, 10, 10, 18, 18, 16, 14, 12,  6, 10], [ 0, 16, 12,  8,  8, 12, 12, 12, 12,  8,  4,  8]]),
        ("5 1 8/7 6 3/4 0 2", [[ 1, 25, 17, 15, 15, 19, 19, 17, 17, 15,  9,  7], [ 1, 19, 12, 11, 11, 13, 13, 13, 13, 11,  7,  6]]),
        ("2 6 4/8 0 7/5 1 3", [[ 2, 22, 18, 14, 14, 22, 22, 18, 22, 16,  6, 16], [ 2, 19, 14, 10, 10, 16, 16, 16, 16, 13,  6, 12]]),
        ("0 5 6/1 4 8/2 7 3", [[ 4, 24, 18, 16, 16, 18, 18, 10, 18, 16,  6, 10], [ 2, 18, 13, 10, 10, 13, 13,  6, 13, 10,  5,  6]]),
        ("4 5 2/8 7 0/6 1 3", [[ 1, 23, 17, 15, 15, 17, 17, 19, 17, 15,  5, 15], [ 1, 16, 11, 11, 11, 11, 11, 14, 11, 11,  4,  9]]),
    ];

    /// A metric to test the solver in.
    trait TestMetric {
        /// The index of the metric's lengths in [`OPTIMAL`].
        const INDEX: usize;

        /// Returns the length of `alg` in the metric.
        fn len(alg: &Algorithm) -> u64;
    }

    impl TestMetric for Stm {
        const INDEX: usize = 0;

        fn len(alg: &Algorithm) -> u64 {
            alg.len_stm()
        }
    }

    impl TestMetric for Mtm {
        const INDEX: usize = 1;

        fn len(alg: &Algorithm) -> u64 {
            alg.len_mtm()
        }
    }

    /// Checks that a solver using `heuristic` finds solutions of the lengths in [`OPTIMAL`] for
    /// `solved_state`, which has index `label`.
    fn check_optimal<S, H, M>(solved_state: S, heuristic: H, label: usize)
    where
        S: SolvedState,
        M: TestMetric,
        GenericSolver<Puzzle, S, H, M>: Solver<Puzzle, Context = ()>,
    {
        let mut solver = GenericSolver::<Puzzle, S, H, M>::new(heuristic, solved_state);
        for (state, lengths) in OPTIMAL {
            let expected = lengths[M::INDEX][label];
            let mut puzzle = Puzzle::from_str(state).unwrap();
            let solution = solver.solve(&puzzle).unwrap();
            assert_eq!(M::len(&solution), expected, "state {state}");
            puzzle.apply_alg(&solution);
            assert!(solver.solved_state.is_solved(&puzzle), "state {state}");
        }
    }

    /// Checks [`check_optimal`] for `solved_state` with no heuristic in both metrics.
    fn check_optimal_no_heuristic<S>(solved_state: S, label: usize)
    where
        S: SolvedState + Solvable + Clone,
    {
        check_optimal::<_, _, Stm>(solved_state.clone(), (), label);
        check_optimal::<_, _, Mtm>(solved_state, (), label);
    }

    /// Checks [`check_optimal`] for `solved_state` with its Manhattan distance and with no
    /// heuristic, in both metrics.
    fn check_optimal_manhattan<S>(solved_state: S, label: usize)
    where
        S: SolvedState + Solvable + Clone,
        ManhattanDistance<S>: Heuristic<Puzzle, S, Stm>,
    {
        let md = ManhattanDistance(solved_state.clone());
        check_optimal::<_, _, Stm>(solved_state.clone(), md.clone(), label);
        check_optimal::<_, _, Mtm>(solved_state.clone(), MtmHeuristic(md), label);
        check_optimal_no_heuristic(solved_state, label);
    }

    #[test]
    fn test_optimal_trivial() {
        check_optimal_manhattan(Trivial, TRIVIAL);
    }

    #[test]
    fn test_optimal_row_grids() {
        check_optimal_manhattan(RowGrids, ROW_GRIDS);

        let max = Max((ManhattanDistance(RowGrids), InversionCount));
        check_optimal::<_, _, Stm>(RowGrids, InversionCount, ROW_GRIDS);
        check_optimal::<_, _, Stm>(RowGrids, max.clone(), ROW_GRIDS);
        check_optimal::<_, _, Mtm>(RowGrids, MtmHeuristic(InversionCount), ROW_GRIDS);
        check_optimal::<_, _, Mtm>(RowGrids, MtmHeuristic(max), ROW_GRIDS);
    }

    #[test]
    fn test_optimal_rows() {
        check_optimal_manhattan(Rows, ROWS);
    }

    #[test]
    fn test_optimal_fringe() {
        check_optimal_manhattan(Fringe, FRINGE);
    }

    #[test]
    fn test_optimal_square_fringe() {
        check_optimal_manhattan(SquareFringe, SQUARE_FRINGE);
    }

    #[test]
    fn test_optimal_split_fringe() {
        check_optimal_manhattan(SplitFringe, SPLIT_FRINGE);
    }

    #[test]
    fn test_optimal_split_square_fringe() {
        check_optimal_manhattan(SplitSquareFringe, SPLIT_SQUARE_FRINGE);
    }

    #[test]
    fn test_optimal_diagonals() {
        check_optimal_manhattan(Diagonals, DIAGONALS);
    }

    #[test]
    fn test_optimal_last_two_rows() {
        check_optimal_no_heuristic(LastTwoRows, LAST_TWO_ROWS);
    }

    #[test]
    fn test_optimal_split_last_two_rows() {
        check_optimal_no_heuristic(SplitLastTwoRows, SPLIT_LAST_TWO_ROWS);
    }

    #[test]
    fn test_optimal_concentric_rectangles() {
        check_optimal_no_heuristic(ConcentricRectangles, CONCENTRIC_RECTANGLES);
    }

    #[test]
    fn test_optimal_checkerboard() {
        check_optimal_manhattan(Checkerboard, CHECKERBOARD);
    }
}
