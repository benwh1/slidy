//! Defines the [`Solver`] struct for solving 4x4 puzzles using pattern databases.

use std::cell::{Cell, Ref, RefCell};

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::direction::Direction,
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
    solver::{
        config::SolverConfig,
        size4x4::stm::{pattern::Pattern, pdb::Pdb, puzzle::Puzzle as Puzzle4},
        solver::{Solver as SolverT, SolverError},
        stack::Stack,
        statistics::{PdbIterationStats, SolverIterationStats},
    },
};

/// The pdb4443 solver.
pub struct Solver {
    pdb4: Pdb,
    pdb3: Pdb,
    stack: Stack<80>,
    solutions_found: Cell<u64>,
    config: RefCell<Option<SolverConfig>>,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver {
    fn new_impl(pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let pat4 = Pattern::new(&[1, 2, 5, 6, 0]);
        let pat3 = Pattern::new(&[11, 12, 15, 0]);

        let pdb4 = Pdb::new(pat4, pdb_iteration_callback);
        let pdb3 = Pdb::new(pat3, pdb_iteration_callback);

        Self {
            pdb4,
            pdb3,
            stack: Stack::default(),
            solutions_found: Cell::new(0),
            config: RefCell::new(None),
        }
    }

    /// Creates a new [`Solver`] and builds the transposition tables and pattern databases.
    #[must_use]
    pub fn new() -> Self {
        Self::new_impl(None)
    }

    /// See [`Self::new`].
    ///
    /// Runs `pdb_iteration_callback` after each iteration of the breadth-first search used to build
    /// the pattern databases.
    pub fn with_pdb_iteration_callback(pdb_iteration_callback: &dyn Fn(PdbIterationStats)) -> Self {
        Self::new_impl(Some(pdb_iteration_callback))
    }

    fn cfg(&self) -> Ref<'_, SolverConfig> {
        let borrow = self.config.borrow();
        Ref::map(borrow, |b| b.as_ref().unwrap())
    }

    fn dfs(&self, depth: u8, last_inverse: Option<Direction>, coords: [u32; 4]) -> bool {
        // SAFETY: The entries in `coords` all come from encoding a puzzle (in `solve`) or from the
        // transposition table (in `dfs`), and we have tests to guarantee that these values are all
        // within bounds.
        //
        // Using `unsafe` here gives a small performance improvement.
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
            self.solutions_found.update(|n| n + 1);
            if let Some(f) = &self.cfg().solution_callback {
                if f(self.stack.to_alg()).is_break() {
                    return true;
                }
            }

            return self.cfg().num_solutions == self.solutions_found.get();
        }

        // SAFETY: See above.
        let (mt1, mt2, mt3, mt4) = unsafe {
            (
                *self
                    .pdb4
                    .transposition_table()
                    .get_unchecked(coords[0] as usize),
                *self
                    .pdb4
                    .transposition_table()
                    .get_unchecked(coords[1] as usize),
                *self
                    .pdb4
                    .transposition_table()
                    .get_unchecked(coords[2] as usize),
                *self
                    .pdb3
                    .transposition_table()
                    .get_unchecked(coords[3] as usize),
            )
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

            self.stack.push(dir);

            if self.dfs(depth - 1, Some(dir.inverse()), new_coords) {
                return true;
            }

            self.stack.pop();
        }

        false
    }

    fn solve_impl<P>(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
    where
        P: SlidingPuzzle,
    {
        if puzzle.size() != Size::new(4, 4).unwrap() {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

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

        let mut pieces = [0; 16];
        for (i, piece) in pieces.iter_mut().enumerate() {
            *piece = puzzle.piece_at(i as u64).to_u8().unwrap();
        }

        let mut puzzle = Puzzle4::from(pieces);
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

        let start_heuristic = entries.iter().copied().sum::<u8>();
        let min = if start_heuristic % 2 == min % 2 {
            min
        } else {
            min + 1
        };

        let mut depth = start_heuristic.max(min);
        let mut first_solution_depth: Option<u8> = None;

        while depth <= max {
            if first_solution_depth.is_some_and(|fd| {
                depth_beyond_optimal.is_some_and(|e| depth > fd.saturating_add(e))
            }) {
                break;
            }

            let found_before = self.solutions_found.get();
            if self.dfs(depth, None, coords) {
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

            depth = match depth.checked_add(2) {
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

impl<P> SolverT<P> for Solver
where
    P: SlidingPuzzle,
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
        puzzle::puzzle::Puzzle,
        solver::{size4x4::stm::solver::Solver, solver::Solver as _},
    };

    #[test]
    fn test_solver() {
        let puzzle = Puzzle::from_str("12 15 5 1/11 9 2 13/0 10 8 6/14 7 4 3").unwrap();
        let solver = Solver::new();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 58);

        // Test it twice to make sure the internal state gets reset properly
        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 58);
    }

    #[test]
    fn test_solve_all_optimal() {
        let puzzle = Puzzle::from_str("1 11 14 15/0 9 4 12/3 10 7 8/13 5 6 2").unwrap();
        let solver = Solver::new();

        let solutions = solver
            .solve_all_optimal(&puzzle)
            .unwrap()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let expected = [
            "LU2L2DRULD2R2URDLUL2URD3LU3RD3RU3LD3RU3LD2LU2",
            "LU2LDLURDLDR2URDLUL2URD3LU3RD3RU3LD3RU3LD2LU2",
            "LU2LDLURD2LURD2LU3RD3LU3RD2RURDLU2LD2RDLURULDLU2",
            "L2ULDRU2LDRD2LU2RD2RU3LD2RURDLU2LD3RU2LDLDR2UL2U2",
            "L2ULDRU2LDRD2LU2R2ULD3RU2RDLU2LD3RU3LD2LDR2UL2U2",
            "L2ULDRU2LDR2ULD3LU2RD2RU2RDLU2LD3RU3LD2LDR2UL2U2",
            "L2URUL2DRULD2R2URDLULULD2RDLU2RD2RU3LD3RU3LD2LU2",
            "L2URULDLURDLDR2URDLULULD2RDLU2RD2RU3LD3RU3LD2LU2",
            "L2URULDLDR2U2L2D2RDLU2RURDRDLULD2RU3LD3RU3LD2LU2",
            "L2URULDLDR2URDLU2L2D2RDLU2RURDLD2RU3LD3RU3LD2LU2",
        ];

        assert_eq!(solutions, expected);
    }
}
