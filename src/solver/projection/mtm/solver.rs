//! Mtm-specific implementation of the projection [`Solver`].

use crate::{
    algorithm::{axis::Axis, direction::Direction, metric::Mtm},
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::{
        projection::{puzzle::ProjectedPuzzle, solver::Solver},
        solver::{Solver as SolverT, SolverConfig, SolverError},
        statistics::SolverIterationStats,
    },
};

impl<P, Target, PruneTarget> Solver<P, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    fn dfs(
        &self,
        puzzle: &P,
        depth: u8,
        last_axis: Option<Axis>,
        projected: ProjectedPuzzle,
    ) -> bool {
        if projected.is_solved(&self.prune_target_solved_state) && self.check_solution(puzzle) {
            self.solutions_found.update(|n| n + 1);
            if let Some(f) = &self.cfg().solution_callback {
                f(self.stack.to_alg());
            }
            return self.cfg().num_solutions == self.solutions_found.get();
        }

        if depth == 0 {
            return false;
        }

        let idx = self.pdb.encode(&projected);
        let heuristic = self.pdb.get(idx);
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
            if last_axis == Some(dir.into()) {
                continue;
            }

            let mut proj = original.clone();
            let mut count = 0;
            while proj.do_move(dir) {
                count += 1;
                self.stack.push(dir);
                if self.dfs(puzzle, depth - 1, Some(dir.into()), proj.clone()) {
                    return true;
                }
            }
            self.stack.remove_n(count);
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

        let min = config.min;
        let max = config.max;

        self.stack.clear();
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let projected = self.initial_projected(puzzle);
        let start_idx = self.pdb.encode(&projected);
        let mut depth = self.pdb.get(start_idx).max(min);

        while depth <= max {
            if self.dfs(puzzle, depth, None, projected.clone()) {
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

impl<P, Target, PruneTarget> SolverT<P> for Solver<P, Target, PruneTarget, Mtm>
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
    use std::str::FromStr as _;

    use super::*;
    use crate::puzzle::{
        label::label::{Rows, Trivial},
        puzzle::Puzzle,
        size::Size,
    };

    type Solver3x3MtmTrivial = Solver<Puzzle, Trivial, Trivial, Mtm>;
    type Solver3x3MtmRows = Solver<Puzzle, Rows, Rows, Mtm>;

    fn size_3x3() -> Size {
        Size::new(3, 3).unwrap()
    }

    #[test]
    fn test_trivial() {
        let solver = Solver3x3MtmTrivial::builder()
            .size(size_3x3())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 2);
    }

    #[test]
    fn test_rows() {
        let solver = Solver3x3MtmRows::builder()
            .size(size_3x3())
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 13);
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
        let solver = Solver::<Puzzle, Rows, Rows211, Mtm>::builder()
            .size(size)
            .target(Rows)
            .prune_target(Rows211)
            .pdb_iteration_callback(&|s| {
                println!("depth {} new {} total {}", s.depth, s.new, s.total);
            })
            .metric(Mtm)
            .build()
            .unwrap();
        let puzzle = Puzzle::from_str("15 14 4 8/2 7 9 11/1 12 3 10/6 13 0 5").unwrap();
        let solution = solver.solve(&puzzle).unwrap();

        assert_eq!(solution.len_mtm::<u64>(), 22);
    }
}
