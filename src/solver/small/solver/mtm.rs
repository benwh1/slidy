use std::{cell::Cell, marker::PhantomData};

use num_traits::AsPrimitive;

use crate::{
    algorithm::{algorithm::Algorithm, axis::Axis, direction::Direction, metric::Mtm},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{
        small::{indexing, pdb::Pdb, solver::Solver},
        solver::SolverError,
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
            solution: [const { Cell::new(Direction::Up) }; 128],
            solution_ptr: Cell::new(0),
            phantom_metric_tag: PhantomData,
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
            return true;
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

            let mut amount = 0;

            puzzle = original_puzzle;

            while puzzle.try_move_dir(dir) {
                amount += 1;

                self.solution[self.solution_ptr.get()].set(dir);
                self.solution_ptr.set(self.solution_ptr.get() + 1);

                if self.dfs(depth - 1, Some(dir.into()), puzzle) {
                    return true;
                }
            }

            self.solution_ptr
                .set(self.solution_ptr.get() - amount as usize);
        }

        false
    }

    fn solve_impl<P: SlidingPuzzle>(
        &self,
        puzzle: &P,
        callback: Option<&dyn Fn(SolverIterationStats)>,
    ) -> Result<Algorithm, SolverError>
    where
        P::Piece: AsPrimitive<u8>,
    {
        let mut p = Puzzle::<W, H>::new();
        if p.try_set_state(puzzle) {
            return self.solve_small_puzzle_impl(p, callback);
        }

        let mut p = Puzzle::<H, W>::new();
        if p.try_set_state(puzzle) {
            return self
                .solve_small_puzzle_impl(p.conjugate_with_transpose(), callback)
                .map(|a| a.transpose());
        }

        Err(SolverError::IncompatiblePuzzleSize)
    }

    fn solve_small_puzzle_impl(
        &self,
        puzzle: Puzzle<W, H>,
        callback: Option<&dyn Fn(SolverIterationStats)>,
    ) -> Result<Algorithm, SolverError> {
        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        // Reset state
        self.solution_ptr.set(0);

        let coord = indexing::encode(puzzle.piece_array());
        let mut depth = self.pdb.get(coord as usize);

        loop {
            if self.dfs(depth, None, puzzle) {
                let mut solution = Algorithm::new();

                for dir in self.solution[..self.solution_ptr.get()]
                    .iter()
                    .map(|c| c.get())
                {
                    solution.push_combine(dir.into());
                }

                return Ok(solution);
            }

            if let Some(f) = callback {
                f(SolverIterationStats { depth });
            }

            depth += 1;
        }
    }

    /// Solves `puzzle`, returning an optimal [`Mtm`] solution.
    pub fn solve<P: SlidingPuzzle>(&self, puzzle: &P) -> Result<Algorithm, SolverError>
    where
        P::Piece: AsPrimitive<u8>,
    {
        self.solve_impl(puzzle, None)
    }

    /// See [`Solver::solve`].
    ///
    /// Runs `callback` after each iteration of the depth-first search.
    pub fn solve_with_callback<P: SlidingPuzzle>(
        &self,
        puzzle: &P,
        callback: &dyn Fn(SolverIterationStats),
    ) -> Result<Algorithm, SolverError>
    where
        P::Piece: AsPrimitive<u8>,
    {
        self.solve_impl(puzzle, Some(callback))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use crate::{
        puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle as _},
        solver::{Solver3x3Mtm, Solver4x2Mtm},
    };

    #[test]
    fn test_solver() {
        let solver = Solver3x3Mtm::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
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
