use std::ops::ControlFlow;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{axis::Axis, direction::Direction, metric::Mtm},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{
        config::{PdbConfig, SolverConfig},
        small::{indexing, pdb::Pdb, solver::Solver},
        solver::{Solver as SolverT, SolverError},
        statistics::SolverIterationStats,
    },
};

impl<const W: usize, const H: usize, const N: usize> Default for Solver<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<const W: usize, const H: usize, const N: usize> Solver<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    /// Creates a [`Solver`], building a new pattern database.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        Self::with_pdb(Pdb::<_, _, _, Mtm>::new(config))
    }

    fn dfs(
        &mut self,
        depth: u8,
        last_axis: Option<Axis>,
        mut puzzle: Puzzle<W, H>,
    ) -> ControlFlow<()> {
        let coord = indexing::encode(puzzle.piece_array());

        // SAFETY: `encode` produces integers from 0 to k-1 where k is the size of the PDB, so the
        // index is always in bounds.
        let heuristic = unsafe { self.pdb.get_unchecked(coord as usize) };

        if heuristic > depth {
            return ControlFlow::Continue(());
        }

        if depth == 0 {
            self.solutions_found += 1;
            if let Some(f) = &self.config.as_ref().unwrap().solution_callback {
                if f(self.stack.to_alg()).is_break() {
                    return ControlFlow::Break(());
                }
            }

            if self.config.as_ref().unwrap().num_solutions == self.solutions_found {
                return ControlFlow::Break(());
            }

            return ControlFlow::Continue(());
        }

        let original_puzzle = puzzle;

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_axis.is_some_and(|a| a == dir.into()) {
                continue;
            }

            let mut count = 0;

            puzzle = original_puzzle;

            while puzzle.try_move_dir(dir) {
                count += 1;

                self.stack.push(dir);

                if self.dfs(depth - 1, Some(dir.into()), puzzle).is_break() {
                    return ControlFlow::Break(());
                }
            }

            self.stack.remove_n(count);
        }

        ControlFlow::Continue(())
    }

    fn solve_impl<P>(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
    where
        P: SlidingPuzzle,
        P::Piece: AsPrimitive<u8>,
    {
        let mut p = Puzzle::<W, H>::new();
        if p.try_set_state(puzzle) {
            return self.solve_small_puzzle_impl(p, config);
        }

        let mut p = Puzzle::<H, W>::new();
        if p.try_set_state(puzzle) {
            let SolverConfig {
                min,
                max,
                depth_beyond_optimal,
                num_solutions,
                end_of_iter_callback,
                solution_callback,
            } = config;

            let transpose_config = SolverConfig {
                min,
                max,
                depth_beyond_optimal,
                num_solutions,
                end_of_iter_callback,
                solution_callback: Some(Box::new(move |s| {
                    solution_callback
                        .as_ref()
                        .map_or(ControlFlow::Continue(()), |f| f(s.transpose()))
                })),
            };

            return self.solve_small_puzzle_impl(p.conjugate_with_transpose(), transpose_config);
        }

        Err(SolverError::IncompatiblePuzzleSize)
    }

    fn solve_small_puzzle_impl(
        &mut self,
        puzzle: Puzzle<W, H>,
        config: SolverConfig,
    ) -> Result<(), SolverError> {
        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let min = config.min;
        let max = config.max;
        let depth_beyond_optimal = config.depth_beyond_optimal;

        // Reset state
        self.stack.clear();
        self.solutions_found = 0;
        self.config = Some(config);

        let coord = indexing::encode(puzzle.piece_array());
        let hval = self.pdb.get(coord as usize);
        let mut depth = hval.max(min);

        if depth > max {
            return Ok(());
        }

        let mut first_solution_depth = None;

        loop {
            // Run DFS. This checks against `num_solutions` and the return value of the solution
            // callback.
            if self.dfs(depth, None, puzzle).is_break() {
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

impl<P, const W: usize, const H: usize, const N: usize> SolverT<P> for Solver<W, H, N, Mtm>
where
    P: SlidingPuzzle,
    P::Piece: AsPrimitive<u8>,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
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

    use crate::{
        puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle as _},
        solver::{solver::Solver as _, Solver3x3Mtm, Solver4x2Mtm},
    };

    #[test]
    fn test_solver() {
        let mut solver = Solver3x3Mtm::default();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 18);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm(), 18);
    }

    #[test]
    fn test_solver_2() {
        let mut solver = Solver4x2Mtm::default();
        let mut puzzle = Puzzle::from_str("4 6/2 5/0 1/7 3").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        puzzle.apply_alg(&solution);
        assert!(puzzle.is_solved());
    }
}
