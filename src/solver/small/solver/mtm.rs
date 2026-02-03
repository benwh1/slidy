//! Defines the [`Solver`] struct for optimally solving small puzzles in [`Mtm`] using a complete
//! pattern database.
//!
//! [`Mtm`]: crate::algorithm::metric::Mtm

use std::cell::Cell;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{algorithm::Algorithm, axis::Axis, direction::Direction},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{Puzzle, sealed::SmallPuzzle},
    },
    solver::{small::{indexing, pdb::Pdb}, solver::SolverError},
};

/// An optimal solver for `WxH` puzzles in [`Mtm`].
///
/// [`Mtm`]: crate::algorithm::metric::Mtm
pub struct Solver<const W: usize, const H: usize, const N: usize> {
    pdb: Pdb,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
}

/// [`Solver`] specialized to the 2x2 size.
pub type Solver2x2 = Solver<2, 2, 4>;
/// [`Solver`] specialized to the 2x3 size.
pub type Solver2x3 = Solver<2, 3, 6>;
/// [`Solver`] specialized to the 2x4 size.
pub type Solver2x4 = Solver<2, 4, 8>;
/// [`Solver`] specialized to the 2x5 size.
pub type Solver2x5 = Solver<2, 5, 10>;
/// [`Solver`] specialized to the 2x6 size.
pub type Solver2x6 = Solver<2, 6, 12>;
/// [`Solver`] specialized to the 3x2 size.
pub type Solver3x2 = Solver<3, 2, 6>;
/// [`Solver`] specialized to the 3x3 size.
pub type Solver3x3 = Solver<3, 3, 9>;
/// [`Solver`] specialized to the 3x4 size.
pub type Solver3x4 = Solver<3, 4, 12>;
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
    /// Creates a new [`Solver`] and builds the pattern database.
    ///
    /// Depending on the size of the puzzle, building the pattern database may take several minutes.
    #[must_use]
    pub fn new() -> Self {
        let pdb = Pdb::new_mtm::<W, H, N>();

        Self {
            pdb,
            solution: [const { Cell::new(Direction::Up) }; 128],
            solution_ptr: Cell::new(0),
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

    /// Solves `puzzle`, returning an optimal [`Mtm`] solution.
    ///
    /// Returns `None` if `puzzle` is not `WxH`.
    ///
    /// [`Mtm`]: crate::algorithm::metric::Mtm
    pub fn solve<P: SlidingPuzzle>(&self, puzzle: &P) -> Result<Algorithm, SolverError>
    where
        P::Piece: AsPrimitive<u8>,
    {
        let mut p = Puzzle::<W, H>::new();
        if !p.try_set_state(puzzle) {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        // Reset state
        self.solution_ptr.set(0);

        let coord = indexing::encode(p.piece_array());
        let mut depth = self.pdb.get(coord as usize);

        loop {
            if self.dfs(depth, None, p) {
                let mut solution = Algorithm::new();

                for dir in self.solution[..self.solution_ptr.get()]
                    .iter()
                    .map(|c| c.get())
                {
                    solution.push_combine(dir.into());
                }

                return Ok(solution);
            }

            depth += 1;
        }
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
        assert_eq!(solution.len_mtm::<u64>(), 18);
    }
}
