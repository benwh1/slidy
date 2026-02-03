//! Defines the [`Solver`] struct for optimally solving small puzzles in [`Stm`] using a complete
//! pattern database.
//!
//! [`Stm`]: crate::algorithm::metric::Stm

use std::cell::Cell;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::small::{indexing, pdb::Pdb},
};

/// An optimal solver for WxH puzzles in [`Stm`].
///
/// [`Stm`]: crate::algorithm::metric::Stm
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

impl<const W: usize, const H: usize, const N: usize> Solver<W, H, N>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    /// Creates a new [`Solver`] and builds the pattern database.
    ///
    /// Depending on the size of the puzzle, building the pattern database may take several minutes.
    pub fn new() -> Self {
        let pdb = Pdb::new_stm::<W, H, N>();

        Self {
            pdb,
            solution: [const { Cell::new(Direction::Up) }; 128],
            solution_ptr: Cell::new(0),
        }
    }

    fn dfs(
        &self,
        depth: u8,
        inverse_last_move: Option<Direction>,
        mut puzzle: Puzzle<W, H>,
    ) -> bool {
        let coord = indexing::encode(puzzle.piece_array());
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

    /// Solves `puzzle`, returning an optimal [`Stm`] solution.
    ///
    /// Returns `None` if `puzzle` is not WxH.
    ///
    /// [`Stm`]: crate::algorithm::metric::Stm
    pub fn solve<P: SlidingPuzzle>(&self, puzzle: &P) -> Option<Algorithm>
    where
        P::Piece: AsPrimitive<u8>,
    {
        let mut p = Puzzle::<W, H>::new();
        if !p.try_set_state(puzzle) {
            return None;
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

                return Some(solution);
            }

            depth += 2;
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
        assert_eq!(solution.len_stm::<u64>(), 25);
    }
}
