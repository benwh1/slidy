//! [`Mtm`]-specific implementation of the projection [`Solver`].

use std::ops::ControlFlow;

use crate::{
    algorithm::{axis::Axis, direction::Direction, metric::Mtm},
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

impl<P, Target, PruneTarget> Solver<P, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Solvable + Default,
    PruneTarget: Label + SolvedState + Default,
{
    fn dfs<const N: usize>(
        &mut self,
        puzzle: &P,
        depth: u8,
        last_axis: Option<Axis>,
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
        let heuristic = unsafe { self.pdb.get_unchecked(index) };
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
            if last_axis == Some(dir.into()) {
                continue;
            }

            let mut proj = original;
            let mut count = 0;
            while proj.do_move(dir) {
                count += 1;
                self.stack.push(dir);
                if self
                    .dfs(puzzle, depth - 1, Some(dir.into()), proj)
                    .is_break()
                {
                    return ControlFlow::Break(());
                }
            }
            self.stack.remove_n(count);
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
        let hval = unsafe { self.pdb.get_unchecked(start_index) };
        let mut depth = hval.max(min);

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

        if self.solutions_found > 0 {
            Ok(())
        } else {
            Err(SolverError::NoSolutionFound)
        }
    }
}

impl<P, Target, PruneTarget> SolverT<P> for Solver<P, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Solvable + Default,
    PruneTarget: Label + SolvedState + Default,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::{
        puzzle::{
            label::label::{Checkerboard, Rows, Trivial},
            puzzle::Puzzle,
            scrambler::{RandomState, Scrambler as _},
            size::Size,
        },
        solver::config::PdbConfig,
    };

    type Solver3x3MtmTrivial = Solver<Puzzle, Trivial, Trivial, Mtm>;
    type Solver3x3MtmRows = Solver<Puzzle, Rows, Rows, Mtm>;

    #[test]
    fn test_trivial() {
        let mut solver = Solver3x3MtmTrivial::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 2);
    }

    #[test]
    fn test_rows() {
        let mut solver = Solver3x3MtmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 13);
    }

    #[test]
    fn test_4x4_rows() {
        #[derive(Default)]
        struct Rows211;

        impl Label for Rows211 {
            fn position_label(&self, _: Size, (_, y): (u64, u64)) -> u64 {
                [0, 0, 1, 2][y as usize]
            }

            fn num_labels(&self, _: Size) -> u64 {
                3
            }
        }

        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, Rows, Rows211, Mtm>::builder()
            .size(size)
            .target(Rows)
            .prune_target(Rows211)
            .pdb_config(PdbConfig {
                end_of_iter_callback: Some(Box::new(|s| {
                    println!("depth {} new {} total {}", s.depth, s.new, s.total);
                })),
            })
            .metric(Mtm)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("15 14 4 8/2 7 9 11/1 12 3 10/6 13 0 5").unwrap();
        let solution = solver.solve(&puzzle).unwrap();

        assert_eq!(solution.len_mtm(), 22);
    }

    #[test]
    fn test_solutions_distinct() {
        let size = Size::new(4, 4).unwrap();
        let mut solver = Solver::<Puzzle, _, _, _>::builder()
            .target(Checkerboard)
            .prune_target(Checkerboard)
            .metric(Mtm)
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
}
