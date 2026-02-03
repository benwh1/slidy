//! Defines the [`Solver`] struct for optimally solving 4x4 puzzles using pattern databases.

use std::cell::Cell;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{algorithm::Algorithm, axis::Axis, direction::Direction},
    puzzle::{sliding_puzzle::SlidingPuzzle, small::Puzzle4x4},
    solver::{
        size4x4::mtm::{
            base_5_table::Base5Table,
            indexing_table::IndexingTable,
            pdb::Pdb,
            puzzle::{FourBitPuzzle, ReducedFourBitPuzzle},
        },
        solver::SolverError,
        statistics::{PdbIterationStats, SolverIterationStats},
    },
};

/// An optimal solver for 4x4 puzzles in [`Mtm`].
///
/// [`Mtm`]: crate::algorithm::metric::Mtm
pub struct Solver {
    indexing_table: IndexingTable,
    base_5_table: Base5Table,
    pdb: Pdb,
    solution: [Cell<Direction>; 128],
    solution_ptr: Cell<usize>,
    puzzle: Cell<FourBitPuzzle>,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver {
    fn new_impl(pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();
        let pdb = Pdb::new(&indexing_table, &base_5_table, pdb_iteration_callback);

        Self {
            indexing_table,
            base_5_table,
            pdb,
            solution: [const { Cell::new(Direction::Up) }; 128],
            solution_ptr: Cell::new(0),
            puzzle: Cell::new(FourBitPuzzle::new()),
        }
    }

    /// Creates a new [`Solver`] and builds the pattern database.
    ///
    /// Building the pattern database takes several minutes.
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
        last_axis: Option<Axis>,
        mut puzzle: ReducedFourBitPuzzle,
        mut transposed_puzzle: ReducedFourBitPuzzle,
    ) -> bool {
        let coord = self
            .indexing_table
            .encode(puzzle.pieces, &self.base_5_table) as usize;

        // SAFETY: We have a test which guarantees that every `ReducedFourBitPuzzle` encodes to an
        // index that is within bounds.
        let heuristic = unsafe { self.pdb.get_unchecked(coord) };

        if heuristic > depth {
            return false;
        }

        let coord = self
            .indexing_table
            .encode(transposed_puzzle.pieces, &self.base_5_table) as usize;

        // SAFETY: See above.
        let heuristic = unsafe { self.pdb.get_unchecked(coord) };

        if heuristic > depth {
            return false;
        }

        if depth == 0 {
            let mut p = self.puzzle.get();
            for mv in &self.solution[..self.solution_ptr.get()] {
                p.do_move(mv.get());
            }
            return p.pieces() == Puzzle4x4::SOLVED;
        }

        let original_puzzle = puzzle;
        let original_transposed = transposed_puzzle;

        for (dir, transposed_dir) in [
            (Direction::Up, Direction::Left),
            (Direction::Left, Direction::Up),
            (Direction::Down, Direction::Right),
            (Direction::Right, Direction::Down),
        ] {
            if last_axis.is_some_and(|a| a == dir.into()) {
                continue;
            }

            let mut amount = 0;

            puzzle = original_puzzle;
            transposed_puzzle = original_transposed;

            while puzzle.do_move(dir) {
                transposed_puzzle.do_move(transposed_dir);
                amount += 1;

                self.solution[self.solution_ptr.get()].set(dir);
                self.solution_ptr.set(self.solution_ptr.get() + 1);

                if self.dfs(depth - 1, Some(dir.into()), puzzle, transposed_puzzle) {
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
        let mut four_bit_puzzle = FourBitPuzzle::new();
        if !four_bit_puzzle.puzzle.try_set_state(puzzle) {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let reduced_puzzle = four_bit_puzzle.reduced();
        let transposed_reduced_puzzle = four_bit_puzzle.transposed().reduced();

        // Reset state
        self.solution_ptr.set(0);
        self.puzzle.set(four_bit_puzzle);

        let coord = self
            .indexing_table
            .encode(reduced_puzzle.pieces, &self.base_5_table);
        let mut depth = self.pdb.get(coord as usize);

        loop {
            if self.dfs(depth, None, reduced_puzzle, transposed_reduced_puzzle) {
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
    ///
    /// [`Mtm`]: crate::algorithm::metric::Mtm
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
