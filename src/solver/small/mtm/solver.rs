use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    ops::ControlFlow,
};

use num_traits::AsPrimitive;

use crate::{
    algorithm::{axis::Axis, direction::Direction, metric::Mtm},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{
        config::SolverConfig,
        small::{indexing, pdb::Pdb, solver::Solver},
        solver::{Solver as SolverT, SolverError},
        stack::Stack,
        statistics::SolverIterationStats,
    },
};

impl<const W: usize, const H: usize, const N: usize> Default for Solver<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize, const N: usize> Solver<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<W, H>>,
{
    /// Creates a [`Solver`], building a new pattern database.
    #[must_use]
    pub fn new() -> Self {
        Self::with_pdb(Pdb::<W, H, N, Mtm>::new())
    }

    /// Creates a [`Solver`] using an existing pattern database.
    #[must_use]
    pub fn with_pdb(pdb: Pdb<W, H, N, Mtm>) -> Self {
        Self {
            pdb,
            stack: Stack::default(),
            solutions_found: Cell::new(0),
            config: RefCell::new(None),
            phantom_metric: PhantomData,
        }
    }

    fn dfs(&self, depth: u8, last_axis: Option<Axis>, mut puzzle: Puzzle<W, H>) -> bool {
        let coord = indexing::encode(puzzle.piece_array());

        // SAFETY: `encode` produces integers from 0 to k-1 where k is the size of the PDB, so the
        // index is always in bounds.
        let heuristic = unsafe { self.pdb.get_unchecked(coord as usize) };

        if heuristic > depth {
            return false;
        }

        if depth == 0 {
            self.solutions_found.update(|n| n + 1);
            if let Some(f) = &self.cfg().solution_callback {
                if f(self.stack.to_alg()).is_break() {
                    return true;
                }
            }

            return self.cfg().num_solutions == self.solutions_found.get();
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

                if self.dfs(depth - 1, Some(dir.into()), puzzle) {
                    return true;
                }
            }

            self.stack.remove_n(count);
        }

        false
    }

    fn solve_impl<P>(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
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
        &self,
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
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let coord = indexing::encode(puzzle.piece_array());
        let mut depth = self.pdb.get(coord as usize).max(min);
        let mut first_solution_depth: Option<u8> = None;

        while depth <= max {
            if first_solution_depth.is_some_and(|fd| {
                depth_beyond_optimal.is_some_and(|e| depth > fd.saturating_add(e))
            }) {
                break;
            }

            let found_before = self.solutions_found.get();
            if self.dfs(depth, None, puzzle) {
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

    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
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
        let solver = Solver3x3Mtm::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 18);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_mtm::<u64>(), 18);
    }

    #[test]
    fn test_solver_2() {
        let solver = Solver4x2Mtm::new();
        let mut puzzle = Puzzle::from_str("4 6/2 5/0 1/7 3").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        puzzle.apply_alg(&solution);
        assert!(puzzle.is_solved());
    }
}
