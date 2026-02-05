//! Defines the [`Solver`] struct for optimally solving small puzzles in [`Stm`] using a complete
//! pattern database.

use std::{cell::Cell, ops::Deref};

use num_traits::AsPrimitive;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction, metric::Stm},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::{
        small::{indexing, pdb::Pdb},
        solver::SolverError,
        statistics::{PdbIterationStats, SolverIterationStats},
    },
};

/// An optimal solver for `WxH` puzzles in [`Stm`].
pub struct Solver<const W: usize, const H: usize, const N: usize> {
    pdb: Pdb<W, H, N, Stm>,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
}

/// An instance of [`Solver`] that transposes the puzzle, solves it, and transposes the solution.
/// This is so that for non-square `WxH` puzzles, we can re-use the pattern database for solving
/// `HxW` puzzles, instead of generating an essentially equivalent one.
pub struct TransposeSolver<const W: usize, const H: usize, const N: usize>(Solver<H, W, N>);

/// [`Solver`] specialized to the 2x2 size.
pub type Solver2x2 = Solver<2, 2, 4>;
/// [`TransposeSolver`] specialized to the 2x3 size.
pub type Solver2x3 = TransposeSolver<2, 3, 6>;
/// [`TransposeSolver`] specialized to the 2x4 size.
pub type Solver2x4 = TransposeSolver<2, 4, 8>;
/// [`TransposeSolver`] specialized to the 2x5 size.
pub type Solver2x5 = TransposeSolver<2, 5, 10>;
/// [`TransposeSolver`] specialized to the 2x6 size.
pub type Solver2x6 = TransposeSolver<2, 6, 12>;
/// [`Solver`] specialized to the 3x2 size.
pub type Solver3x2 = Solver<3, 2, 6>;
/// [`Solver`] specialized to the 3x3 size.
pub type Solver3x3 = Solver<3, 3, 9>;
/// [`TransposeSolver`] specialized to the 3x4 size.
pub type Solver3x4 = TransposeSolver<3, 4, 12>;
/// [`Solver`] specialized to the 4x2 size.
pub type Solver4x2 = Solver<4, 2, 8>;
/// [`Solver`] specialized to the 4x3 size.
pub type Solver4x3 = Solver<4, 3, 12>;
/// [`Solver`] specialized to the 5x2 size.
pub type Solver5x2 = Solver<5, 2, 10>;
/// [`Solver`] specialized to the 6x2 size.
pub type Solver6x2 = Solver<6, 2, 12>;

impl<const W: usize, const H: usize, const N: usize> Default for Solver<W, H, N>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize, const N: usize> Solver<W, H, N>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn new_impl(pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let pdb = Pdb::<W, H, N, Stm>::new_impl(pdb_iteration_callback);

        Self {
            pdb,
            solution: [const { Cell::new(Direction::Up) }; 128],
            solution_ptr: Cell::new(0),
        }
    }

    /// Creates a new [`Solver`] and builds the pattern database.
    ///
    /// Depending on the size of the puzzle, building the pattern database may take several minutes.
    #[must_use]
    pub fn new() -> Self {
        Self::new_impl(None)
    }

    /// See [`Self::new`].
    ///
    /// Runs `pdb_iteration_callback` after each iteration of the breadth-first search used to build
    /// the pattern database.
    pub fn with_pdb_iteration_callback(pdb_iteration_callback: &dyn Fn(PdbIterationStats)) -> Self {
        Self::new_impl(Some(pdb_iteration_callback))
    }

    fn dfs(
        &self,
        depth: u8,
        inverse_last_move: Option<Direction>,
        mut puzzle: Puzzle<W, H>,
    ) -> bool {
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
            if inverse_last_move == Some(dir) {
                continue;
            }

            puzzle = original_puzzle;

            if puzzle.try_move_dir(dir) {
                self.solution[self.solution_ptr.get()].set(dir);
                self.solution_ptr.set(self.solution_ptr.get() + 1);

                if self.dfs(depth - 1, Some(dir.inverse()), puzzle) {
                    return true;
                }

                self.solution_ptr.set(self.solution_ptr.get() - 1);
            }
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
        if !p.try_set_state(puzzle) {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        self.solve_small_puzzle_impl(p, callback)
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

            depth += 2;
        }
    }

    /// Solves `puzzle`, returning an optimal [`Stm`] solution.
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

    /// Returns a reference to the pattern database used by the solver.
    pub fn pdb(&self) -> &Pdb<W, H, N, Stm> {
        &self.pdb
    }
}

impl<const W: usize, const H: usize, const N: usize> TransposeSolver<W, H, N>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N], TransposedPuzzle = Puzzle<H, W>>,
    Puzzle<H, W>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn new_impl(pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        Self(Solver::new_impl(pdb_iteration_callback))
    }

    /// Creates a new [`TransposeSolver`] and builds the pattern database.
    ///
    /// Depending on the size of the puzzle, building the pattern database may take several minutes.
    #[must_use]
    pub fn new() -> Self {
        Self::new_impl(None)
    }

    /// See [`Self::new`].
    ///
    /// Runs `pdb_iteration_callback` after each iteration of the breadth-first search used to build
    /// the pattern database.
    pub fn with_pdb_iteration_callback(pdb_iteration_callback: &dyn Fn(PdbIterationStats)) -> Self {
        Self::new_impl(Some(pdb_iteration_callback))
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
        if !p.try_set_state(puzzle) {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        self.0.solve_small_puzzle_impl(p.transpose(), callback)
    }

    /// Solves `puzzle`, returning an optimal [`Stm`] solution.
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

impl<const W: usize, const H: usize, const N: usize> Deref for TransposeSolver<W, H, N> {
    type Target = Solver<H, W, N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use crate::puzzle::puzzle::Puzzle;

    use super::*;

    #[test]
    fn test_solver_3x3() {
        let solver = Solver3x3::new();
        let puzzle = Puzzle::from_str("7 0 4/5 6 2/3 8 1").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 25);
    }
}
