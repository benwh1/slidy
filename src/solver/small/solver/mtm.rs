use std::cell::Cell;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{algorithm::Algorithm, axis::Axis, direction::Direction},
    puzzle::{
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
    },
    solver::small::{indexing, pdb::Pdb},
};

pub struct Solver<const W: usize, const H: usize, const N: usize> {
    pdb: Pdb,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
}

pub type Solver2x2 = Solver<2, 2, 4>;
pub type Solver2x3 = Solver<2, 3, 6>;
pub type Solver2x4 = Solver<2, 4, 8>;
pub type Solver2x5 = Solver<2, 5, 10>;
pub type Solver2x6 = Solver<2, 6, 12>;
pub type Solver3x2 = Solver<3, 2, 6>;
pub type Solver3x3 = Solver<3, 3, 9>;
pub type Solver3x4 = Solver<3, 4, 12>;
pub type Solver4x2 = Solver<4, 2, 8>;
pub type Solver4x3 = Solver<4, 3, 12>;
pub type Solver5x2 = Solver<5, 2, 10>;
pub type Solver6x2 = Solver<6, 2, 12>;

impl<const W: usize, const H: usize, const N: usize> Solver<W, H, N>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
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
        let puzzle = Puzzle::from_str("7 0 5/8 3 1/2 6 4").unwrap();
        let solution = solver.solve(&puzzle).unwrap();
        println!("{solution}");
    }
}
