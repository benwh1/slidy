//! Stm-specific implementation of the projection [`Solver`].

use crate::{
    algorithm::{direction::Direction, metric::Stm},
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::{
        projection::{encoding, puzzle::ProjectedPuzzle, solver::Solver},
        solver::{Solver as SolverT, SolverConfig, SolverError},
        statistics::SolverIterationStats,
    },
};

impl<P, Target, PruneTarget> Solver<P, Target, PruneTarget, Stm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    fn dfs<const N: usize>(
        &self,
        puzzle: &P,
        depth: u8,
        last_dir: Option<Direction>,
        projected: ProjectedPuzzle<N>,
    ) -> bool {
        let index = self.pdb.encode(&projected);
        if index == self.prune_target_solved_index && self.check_solution(puzzle) {
            self.solutions_found.update(|n| n + 1);
            if let Some(f) = &self.cfg().solution_callback {
                f(self.stack.to_alg());
            }
            return self.cfg().num_solutions == self.solutions_found.get();
        }

        if depth == 0 {
            return false;
        }

        // SAFETY: `index` comes from encoding a projected puzzle, so is within bounds.
        let heuristic = unsafe { self.pdb.get_unchecked(index) };
        if heuristic > depth {
            return false;
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
                if self.dfs(puzzle, depth - 1, Some(dir), proj) {
                    return true;
                }
                self.stack.pop();
            }
        }

        false
    }

    fn solve_impl(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        if puzzle.size() != self.size {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        if self.size.area() as usize <= encoding::SMALL {
            self.solve_with_size::<{ encoding::SMALL }>(puzzle, config)
        } else {
            self.solve_with_size::<{ encoding::MAX_PIECES }>(puzzle, config)
        }
    }

    fn solve_with_size<const N: usize>(
        &self,
        puzzle: &P,
        config: SolverConfig,
    ) -> Result<(), SolverError> {
        let min = config.min;
        let max = config.max;

        self.stack.clear();
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let projected = self.initial_projected::<N>(puzzle);
        let start_index = self.pdb.encode(&projected);
        // SAFETY: `start_index` comes from encoding a projected puzzle, so is within bounds.
        let pdb_val = unsafe { self.pdb.get_unchecked(start_index) };
        let mut depth = pdb_val.max(min);

        while depth <= max {
            if self.dfs::<N>(puzzle, depth, None, projected) {
                return Ok(());
            }

            if let Some(f) = &self.cfg().end_of_iter_callback {
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

impl<P, Target, PruneTarget> SolverT<P> for Solver<P, Target, PruneTarget, Stm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, str::FromStr as _};

    use super::*;
    use crate::puzzle::{
        label::{
            label::{Rows, Trivial},
            scaled::Scaled,
        },
        puzzle::Puzzle,
        size::Size,
    };

    type Solver3x3StmTrivial = Solver<Puzzle, Trivial, Trivial, Stm>;
    type Solver3x3StmRows = Solver<Puzzle, Rows, Rows, Stm>;
    type Solver3x3StmDiff = Solver<Puzzle, Rows, Trivial, Stm>;

    #[test]
    fn test_stm_trivial() {
        let solver = Solver3x3StmTrivial::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm::<u64>() > 0);
    }

    #[test]
    fn test_stm_rows() {
        let solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm::<u64>() > 0);
    }

    #[test]
    fn test_stm_different_targets() {
        let solver = Solver3x3StmDiff::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm::<u64>() > 0);
    }

    #[test]
    fn test_solution_validates() {
        let solver = Solver3x3StmRows::builder()
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
        let solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let s1 = solver.solve(&puzzle).unwrap();
        let s2 = solver.solve(&puzzle).unwrap();
        assert_eq!(s1.len_stm::<u64>(), s2.len_stm::<u64>());
    }

    #[test]
    fn test_stm_rows_double_rows_4x4() {
        let size = Size::new(4, 4).unwrap();
        let prune = Scaled::new(Rows, (2, 2)).unwrap();
        let solver = Solver::<Puzzle, Rows, Scaled<Rows>, Stm>::builder()
            .size(size)
            .prune_target(prune)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("12 7 9 10/5 6 0 14/11 15 2 8/3 1 4 13").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 45);
    }

    #[test]
    fn test_stm_rows_with_pdb_iteration_callback() {
        let iterations = Cell::new(0u64);
        let solver = Solver3x3StmRows::builder()
            .size(Size::new(3, 3).unwrap())
            .pdb_iteration_callback(&|stats| {
                assert!(stats.total > 0);
                iterations.set(iterations.get() + 1);
            })
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert!(solution.len_stm::<u64>() > 0);
        assert!(iterations.get() > 0);
    }

    #[test]
    fn test_stm_all_builder_options() {
        let size = Size::new(4, 4).unwrap();
        let prune = Scaled::new(Rows, (2, 2)).unwrap();
        let solver = Solver::<Puzzle, Rows, Scaled<Rows>, Stm>::builder()
            .size(size)
            .target(Rows)
            .prune_target(prune)
            .pdb_iteration_callback(&|_| {})
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("12 7 9 10/5 6 0 14/11 15 2 8/3 1 4 13").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 45);
    }

    #[test]
    fn test_stm_missing_size_error() {
        let err = Solver3x3StmRows::builder().build();
        assert!(matches!(
            err,
            Err(crate::solver::projection::builder::ProjectionError::MissingSize)
        ));
    }
}
