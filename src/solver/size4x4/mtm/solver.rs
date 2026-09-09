//! Defines the [`Solver`] struct for optimally solving 4x4 puzzles using pattern databases.

use std::cell::{Cell, Ref, RefCell};

use num_traits::AsPrimitive;

use crate::{
    algorithm::{axis::Axis, direction::Direction},
    puzzle::{sliding_puzzle::SlidingPuzzle, small::Puzzle4x4},
    solver::{
        config::SolverConfig,
        size4x4::mtm::{
            base_5_table::Base5Table,
            indexing_table::IndexingTable,
            pdb::Pdb,
            puzzle::{FourBitPuzzle, ReducedFourBitPuzzle},
        },
        solver::{Solver as SolverT, SolverError},
        stack::Stack,
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
    stack: Stack<128>,
    puzzle: Cell<FourBitPuzzle>,
    solutions_found: Cell<u64>,
    config: RefCell<Option<SolverConfig>>,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver {
    fn with_tables_and_pdb(
        indexing_table: IndexingTable,
        base_5_table: Base5Table,
        pdb: Pdb,
    ) -> Self {
        Self {
            indexing_table,
            base_5_table,
            pdb,
            stack: Stack::default(),
            puzzle: Cell::new(FourBitPuzzle::new()),
            solutions_found: Cell::new(0),
            config: RefCell::new(None),
        }
    }

    fn new_impl(pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) -> Self {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();
        let pdb = Pdb::new(&indexing_table, &base_5_table, pdb_iteration_callback);

        Self::with_tables_and_pdb(indexing_table, base_5_table, pdb)
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

    /// See [`Self::new`].
    ///
    /// Initializes a [`Solver`] using a boxed byte slice containing the pre-computed pattern
    /// database data.
    ///
    /// The length of the data is checked, and the [`xxh3`] hash is computed and checked against a
    /// known value to verify integrity.
    ///
    /// # Safety
    ///
    /// Despite the correctness checks described above, this function is unsafe because it is
    /// still technically possible for `bytes` to contain incorrect data in the event of a hash
    /// collision.
    ///
    /// If the data is incorrect, then running the solver can cause undefined behavior.
    ///
    /// [`xxh3`]: xxhash_rust::xxh3
    #[must_use]
    pub unsafe fn try_with_pdb_bytes(bytes: Box<[u8]>) -> Option<Self> {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();

        // SAFETY: Checks are performed in `Pdb::try_from_bytes` which almost certainly detects
        // invalid data, but ultimately the caller is responsible.
        let pdb = unsafe { Pdb::try_from_bytes(bytes) }?;

        Some(Self::with_tables_and_pdb(indexing_table, base_5_table, pdb))
    }

    /// See [`Self::try_with_pdb_bytes`].
    ///
    /// # Safety
    ///
    /// The caller is responsible for the correctness of the data contained in `bytes`. No
    /// correctness checks are performed.
    #[must_use]
    pub unsafe fn with_pdb_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();

        // SAFETY: Responsibility of the caller.
        let pdb = unsafe { Pdb::from_bytes_unchecked(bytes) };

        Self::with_tables_and_pdb(indexing_table, base_5_table, pdb)
    }

    fn cfg(&self) -> Ref<'_, SolverConfig> {
        let borrow = self.config.borrow();
        Ref::map(borrow, |b| b.as_ref().unwrap())
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
            for dir in self.stack.iter() {
                p.do_move(dir);
            }

            if p.pieces() == Puzzle4x4::SOLVED {
                self.solutions_found.update(|n| n + 1);
                if let Some(f) = &self.cfg().solution_callback {
                    if f(self.stack.to_alg()).is_break() {
                        return true;
                    }
                }

                return self.cfg().num_solutions == self.solutions_found.get();
            }

            return false;
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

            let mut count = 0;

            puzzle = original_puzzle;
            transposed_puzzle = original_transposed;

            while puzzle.do_move(dir) {
                transposed_puzzle.do_move(transposed_dir);
                count += 1;

                self.stack.push(dir);

                if self.dfs(depth - 1, Some(dir.into()), puzzle, transposed_puzzle) {
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
        let mut four_bit_puzzle = FourBitPuzzle::new();
        if !four_bit_puzzle.puzzle.try_set_state(puzzle) {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let reduced_puzzle = four_bit_puzzle.reduced();
        let transposed_reduced_puzzle = four_bit_puzzle.conjugate_with_transpose().reduced();

        let min = config.min;
        let max = config.max;
        let depth_beyond_optimal = config.depth_beyond_optimal;

        // Reset state
        self.stack.clear();
        self.puzzle.set(four_bit_puzzle);
        self.solutions_found.set(0);
        *self.config.borrow_mut() = Some(config);

        let coord = self
            .indexing_table
            .encode(reduced_puzzle.pieces, &self.base_5_table);
        let mut depth = self.pdb.get(coord as usize).max(min);
        let mut first_solution_depth: Option<u8> = None;

        while depth <= max {
            if first_solution_depth.is_some_and(|fd| {
                depth_beyond_optimal.is_some_and(|e| depth > fd.saturating_add(e))
            }) {
                break;
            }

            let found_before = self.solutions_found.get();
            if self.dfs(depth, None, reduced_puzzle, transposed_reduced_puzzle) {
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

    /// Returns a reference to the data contained in the pattern database, in order to allow it to
    /// be written to disk.
    pub fn pdb_bytes(&self) -> &[u8] {
        self.pdb.as_ref()
    }
}

impl<P> SolverT<P> for Solver
where
    P: SlidingPuzzle,
    P::Piece: AsPrimitive<u8>,
{
    fn is_initialised(&self) -> bool {
        true
    }

    fn init(&mut self) {}

    fn solve_with_config(&self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}
