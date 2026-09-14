//! [`Stm`]-specific implementation of the projection [`Solver`].

use std::ops::ControlFlow;

use crate::{
    algorithm::{direction::Direction, metric::Stm},
    puzzle::{
        label::label::Label, sliding_puzzle::SlidingPuzzle, solvable::Solvable,
        solved_state::SolvedState,
    },
    solver::{
        config::SolverConfig,
        projection::{puzzle::ProjectedPuzzle, solver::Solver, LARGE, SMALL},
        solver::{Solver as SolverT, SolverError},
        statistics::SolverIterationStats,
    },
};

impl<P, Target, PruneTarget> Solver<P, Target, PruneTarget, Stm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Solvable + Default,
    PruneTarget: Label + SolvedState + Default,
{
    fn dfs<const N: usize>(
        &mut self,
        puzzle: &P,
        depth: u64,
        last_dir: Option<Direction>,
        projected: ProjectedPuzzle<N>,
    ) -> ControlFlow<()> {
        let index = projected.encode(&self.tally) as usize;

        if depth == 0 {
            if index == self.prune_target_solved_index && self.check_solution(puzzle) {
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

        // SAFETY: `index` comes from encoding a projected puzzle, so is within bounds.
        let heuristic = unsafe { self.pdb.get_unchecked(index) } as u64;
        if heuristic > depth {
            return ControlFlow::Continue(());
        }

        let original = projected;

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_dir == Some(dir.inverse()) {
                continue;
            }

            let mut proj = original;
            if proj.do_move(dir) {
                self.stack.push(dir);
                if self.dfs(puzzle, depth - 1, Some(dir), proj).is_break() {
                    return ControlFlow::Break(());
                }
                self.stack.pop();
            }
        }

        ControlFlow::Continue(())
    }

    fn solve_impl(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if self.size.area() as usize <= SMALL {
            self.solve_impl_n::<SMALL>(puzzle, config)
        } else {
            self.solve_impl_n::<LARGE>(puzzle, config)
        }
    }

    fn solve_impl_n<const N: usize>(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<(), SolverError> {
        if puzzle.size() != self.size {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !self.target.is_solvable(puzzle) {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;
        let depth_beyond_optimal = config.depth_beyond_optimal;

        self.stack.clear();
        self.solutions_found = 0;
        self.config = Some(config);

        let projected = self.initial_projected::<N>(puzzle);
        let start_index = projected.encode(&self.tally) as usize;
        // SAFETY: `start_index` comes from encoding a projected puzzle, so is within bounds.
        let hval = unsafe { self.pdb.get_unchecked(start_index) } as u64;
        let min = if hval % 2 == min % 2 { min } else { min + 1 };
        let mut depth = hval.max(min);

        if depth > max {
            return Ok(());
        }

        let mut first_solution_depth = None;

        loop {
            // Run DFS. This checks against `num_solutions` and the return value of the solution
            // callback.
            if self.dfs::<N>(puzzle, depth, None, projected).is_break() {
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

impl<P, Target, PruneTarget> SolverT<P> for Solver<P, Target, PruneTarget, Stm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Solvable + Default,
    PruneTarget: Label + SolvedState + Default,
{
    fn is_initialized(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ops::ControlFlow,
        str::FromStr as _,
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc, Mutex,
        },
    };

    use super::*;
    use crate::{
        puzzle::{
            label::{
                label::{Checkerboard, Rows, Trivial},
                scaled::Scaled,
            },
            puzzle::Puzzle,
            scrambler::{RandomState, Scrambler as _},
            size::Size,
        },
        solver::config::PdbConfig,
    };

    type Solver3x3StmTrivial = Solver<Puzzle, Trivial, Trivial, Stm>;
    type Solver3x3StmRows = Solver<Puzzle, Rows, Rows, Stm>;
    type Solver3x3StmDiff = Solver<Puzzle, Rows, Trivial, Stm>;

    #[test]
    fn test_trivial() {
        let mut solver = Solver3x3StmTrivial::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
    }

    #[test]
    fn test_rows() {
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
    }

    #[test]
    fn test_different_targets() {
        let mut solver = Solver3x3StmDiff::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
    }

    #[test]
    fn test_solution_validates() {
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let mut puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        puzzle.apply_alg(&solution);
        assert!(Rows.is_solved(&puzzle));
    }

    #[test]
    fn test_solve_twice() {
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let s1 = solver.solve(&puzzle).unwrap();
        let s2 = solver.solve(&puzzle).unwrap();
        assert_eq!(s1.len_stm(), s2.len_stm());
    }

    #[test]
    fn test_rows_double_rows_4x4() {
        let size = Size::new(4, 4).unwrap();
        let prune = Scaled::new(Rows, (2, 2)).unwrap();
        let mut solver = Solver::<Puzzle, Rows, Scaled<Rows>, Stm>::builder()
            .size(size)
            .prune_target(prune)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("12 7 9 10/5 6 0 14/11 15 2 8/3 1 4 13").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 45);
    }

    #[test]
    fn test_rows_with_pdb_iteration_callback() {
        let iterations = Arc::new(AtomicU64::new(0));
        let iterations_ref = iterations.clone();
        let mut solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .pdb_config(PdbConfig {
                end_of_iter_callback: Some(Box::new(move |stats| {
                    assert!(stats.total > 0);
                    iterations_ref.fetch_add(1, Ordering::Relaxed);
                })),
            })
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm() > 0);
        assert!(iterations.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_solutions_distinct() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
            .size(size)
            .build()
            .unwrap();

        let mut puzzle = Puzzle::new(size);

        for _ in 0..100 {
            RandomState.scramble(&mut puzzle);

            let mut solutions = solver.solve_many(&puzzle, 5).unwrap();
            solutions.sort_by_cached_key(|s| s.to_string());
            solutions.dedup();

            assert_eq!(solutions.len(), 5, "failed on {puzzle}");
        }
    }

    #[test]
    fn test_depth_beyond_optimal_bounds_solutions() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("5 6 2 9/13 15 14 7/8 1 11 12/4 3 10 0").unwrap();
        let optimal = solver.solve(&puzzle).unwrap().len_stm();

        for (depth_beyond_optimal, max_len) in [(Some(0), optimal), (Some(2), optimal + 2)] {
            let solutions = Arc::new(Mutex::new(Vec::new()));
            let sc = solutions.clone();
            let config = SolverConfig {
                depth_beyond_optimal,
                num_solutions: 1000,
                solution_callback: Some(Box::new(move |s| {
                    sc.lock().unwrap().push(s.len_stm());
                    ControlFlow::Continue(())
                })),
                ..Default::default()
            };

            solver.solve_with_config(&puzzle, config).unwrap();

            let lens = solutions.lock().unwrap();
            assert!(
                !lens.is_empty(),
                "no solutions for depth_beyond_optimal={depth_beyond_optimal:?}"
            );
            for len in lens.iter() {
                assert!(
                    *len <= max_len && *len >= optimal,
                    "len {len} outside [{optimal}, {max_len}] for depth_beyond_optimal={depth_beyond_optimal:?}"
                );
            }
        }
    }

    #[test]
    fn test_solution_callback_break_stops() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("5 6 2 9/13 15 14 7/8 1 11 12/4 3 10 0").unwrap();

        let calls = Arc::new(AtomicU64::new(0));
        let cc = calls.clone();
        let config = SolverConfig {
            num_solutions: u64::MAX,
            solution_callback: Some(Box::new(move |_| {
                cc.fetch_add(1, Ordering::Relaxed);
                ControlFlow::Break(())
            })),
            ..Default::default()
        };

        let result = solver.solve_with_config(&puzzle, config);

        assert_eq!(result, Ok(()));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_end_of_iter_callback_break_stops() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Stm)
            .size(size)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("5 6 2 9/13 15 14 7/8 1 11 12/4 3 10 0").unwrap();

        let config = SolverConfig {
            num_solutions: u64::MAX,
            end_of_iter_callback: Some(Box::new(|_| ControlFlow::Break(()))),
            ..Default::default()
        };

        let result = solver.solve_with_config(&puzzle, config);

        assert_eq!(result, Ok(()));
    }
}
