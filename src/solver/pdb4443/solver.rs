//! Defines the [`Solver`] struct for solving 4x4 puzzles using pattern databases.

use std::cell::Cell;

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction},
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
    solver::{
        pdb4443::{pattern::Pattern, pdb::Pdb, puzzle::Puzzle},
        solver::SolverError,
    },
};

/// The pdb4443 solver.
pub struct Solver {
    pdb4: Pdb,
    pdb3: Pdb,
    solution: [Cell<Direction>; 80],
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver {
    /// Creates a new instance of the solver.
    #[must_use]
    pub fn new() -> Self {
        let pat4 = Pattern::new(&[1, 2, 5, 6, 0]);
        let pat3 = Pattern::new(&[11, 12, 15, 0]);

        let pdb4 = Pdb::new(pat4);
        let pdb3 = Pdb::new(pat3);

        Self {
            pdb4,
            pdb3,
            solution: [const { Cell::new(Direction::Up) }; 80],
        }
    }

    fn dfs(&self, depth: u8, last_inverse: Option<Direction>, coords: [u32; 4]) -> bool {
        let heuristic = unsafe {
            self.pdb4.pdb().get_unchecked(coords[0] as usize)
                + self.pdb4.pdb().get_unchecked(coords[1] as usize)
                + self.pdb4.pdb().get_unchecked(coords[2] as usize)
                + self.pdb3.pdb().get_unchecked(coords[3] as usize)
        };

        if heuristic > depth {
            return false;
        }

        if depth == 0 {
            return true;
        }

        let mt1 = unsafe {
            *self
                .pdb4
                .transposition_table()
                .get_unchecked(coords[0] as usize)
        };
        let mt2 = unsafe {
            *self
                .pdb4
                .transposition_table()
                .get_unchecked(coords[1] as usize)
        };
        let mt3 = unsafe {
            *self
                .pdb4
                .transposition_table()
                .get_unchecked(coords[2] as usize)
        };
        let mt4 = unsafe {
            *self
                .pdb3
                .transposition_table()
                .get_unchecked(coords[3] as usize)
        };

        for dir in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if last_inverse.is_some_and(|m| m == dir) {
                continue;
            }

            if mt1[dir as usize] == u32::MAX {
                continue;
            }

            let new_coords = [
                mt1[dir as usize],
                mt2[dir.reflect_left_right() as usize],
                mt3[dir.reflect_up_down() as usize],
                mt4[dir as usize],
            ];

            if self.dfs(depth - 1, Some(dir.inverse()), new_coords) {
                self.solution[depth as usize - 1].set(dir);
                return true;
            }
        }

        false
    }

    /// Computes an optimal solution of `puzzle`.
    pub fn solve<P: SlidingPuzzle>(&self, puzzle: &P) -> Result<Algorithm, SolverError> {
        if puzzle.size() != Size::new(4, 4).unwrap() {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        let mut pieces = [0u8; 16];
        for (i, piece) in pieces.iter_mut().enumerate() {
            *piece = puzzle.piece_at(i as u64).to_u8().unwrap();
        }

        let mut puzzle = Puzzle::from(pieces);
        let mut coords = [0; 4];

        coords[0] = puzzle.encode(self.pdb4.pattern()) as u32;
        puzzle.reflect_left_right();
        coords[1] = puzzle.encode(self.pdb4.pattern()) as u32;
        puzzle.reflect_left_right();
        puzzle.reflect_up_down();
        coords[2] = puzzle.encode(self.pdb4.pattern()) as u32;
        puzzle.reflect_up_down();
        coords[3] = puzzle.encode(self.pdb3.pattern()) as u32;

        let entries = [
            self.pdb4.pdb()[coords[0] as usize],
            self.pdb4.pdb()[coords[1] as usize],
            self.pdb4.pdb()[coords[2] as usize],
            self.pdb3.pdb()[coords[3] as usize],
        ];

        let mut depth = entries.iter().copied().sum::<u8>();

        loop {
            if self.dfs(depth, None, coords) {
                let mut solution = Algorithm::new();

                for dir in self.solution[..depth as usize]
                    .iter()
                    .rev()
                    .map(|c| c.get())
                {
                    solution.push_combine(dir.into());
                }

                return Ok(solution);
            }

            depth += 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{puzzle::puzzle::Puzzle, solver::pdb4443::solver::Solver};

    #[test]
    fn test_solver() {
        let puzzle = Puzzle::from_str("12 15 5 1/11 9 2 13/0 10 8 6/14 7 4 3").unwrap();
        let solver = Solver::new();
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm::<u64>(), 58);
    }
}
