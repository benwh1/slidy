//! Defines the [`Solver`] struct for optimally solving 4x4 puzzles using pattern databases.

use std::ops::ControlFlow;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{axis::Axis, direction::Direction},
    puzzle::{sliding_puzzle::SlidingPuzzle, small::Puzzle4x4},
    solver::{
        config::{PdbConfig, SolverConfig},
        size4x4::mtm::{
            base_5_table::Base5Table,
            indexing_table::IndexingTable,
            pdb::Pdb,
            puzzle::{FourBitPuzzle, ReducedFourBitPuzzle},
        },
        solver::{Solver as SolverT, SolverError},
        stack::Stack,
        statistics::SolverIterationStats,
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
    puzzle: FourBitPuzzle,
    solutions_found: u64,
    config: Option<SolverConfig>,
}

impl Default for Solver {
    fn default() -> Self {
        Self::new(&PdbConfig::default())
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
            stack: Stack::new(),
            puzzle: FourBitPuzzle::new(),
            solutions_found: 0,
            config: None,
        }
    }

    /// Creates a new [`Solver`] and builds the pattern database.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();
        let pdb = Pdb::new(&indexing_table, &base_5_table, config);

        Self::with_tables_and_pdb(indexing_table, base_5_table, pdb)
    }

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

    fn dfs(
        &mut self,
        depth: u64,
        last_axis: Option<Axis>,
        mut puzzle: ReducedFourBitPuzzle,
        mut transposed_puzzle: ReducedFourBitPuzzle,
    ) -> ControlFlow<()> {
        let coord = self
            .indexing_table
            .encode(puzzle.pieces, &self.base_5_table) as usize;

        // SAFETY: We have a test which guarantees that every `ReducedFourBitPuzzle` encodes to an
        // index that is within bounds.
        let heuristic = unsafe { self.pdb.get_unchecked(coord) } as u64;
        if heuristic > depth {
            return ControlFlow::Continue(());
        }

        let coord = self
            .indexing_table
            .encode(transposed_puzzle.pieces, &self.base_5_table) as usize;

        // SAFETY: See above.
        let heuristic = unsafe { self.pdb.get_unchecked(coord) } as u64;
        if heuristic > depth {
            return ControlFlow::Continue(());
        }

        if depth == 0 {
            let mut p = self.puzzle;
            for dir in self.stack.iter() {
                p.do_move(dir);
            }

            if p.pieces() == Puzzle4x4::SOLVED {
                self.solutions_found += 1;
                if let Some(f) = &self.config.as_ref().unwrap().solution_callback {
                    if f(self.stack.to_alg()).is_break() {
                        return ControlFlow::Break(());
                    }
                }

                if self.config.as_ref().unwrap().num_solutions == self.solutions_found {
                    return ControlFlow::Break(());
                }
            }

            return ControlFlow::Continue(());
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

                if self
                    .dfs(depth - 1, Some(dir.into()), puzzle, transposed_puzzle)
                    .is_break()
                {
                    return ControlFlow::Break(());
                }
            }

            self.stack.remove_n(count);
        }

        ControlFlow::Continue(())
    }

    fn solve_impl<P>(&mut self, puzzle: &P, config: SolverConfig) -> Result<(), SolverError>
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
        self.puzzle = four_bit_puzzle;
        self.solutions_found = 0;
        self.config = Some(config);

        let coord = self
            .indexing_table
            .encode(reduced_puzzle.pieces, &self.base_5_table);
        let hval = self.pdb.get(coord as usize) as u64;
        let mut depth = hval.max(min);

        if depth > max {
            return Ok(());
        }

        let mut first_solution_depth = None;

        loop {
            // Run DFS. This checks against `num_solutions` and the return value of the solution
            // callback.
            if self
                .dfs(depth, None, reduced_puzzle, transposed_reduced_puzzle)
                .is_break()
            {
                break;
            }

            // Set first solution depth.
            if first_solution_depth.is_none() && self.solutions_found > 0 {
                first_solution_depth = Some(depth);
            }

            // Run end of iteration callback and check return value.
            if let Some(f) = &self.config.as_ref().unwrap().end_of_iter_callback {
                if f(SolverIterationStats { depth }).is_break() {
                    break;
                }
            }

            // Go to next depth.
            depth = match depth.checked_add(1) {
                Some(d) => d,
                None => break,
            };

            // Check against `max`.
            if depth > max {
                break;
            }

            // Check against `depth_beyond_optimal`.
            if first_solution_depth.is_some_and(|first| depth - first > depth_beyond_optimal) {
                break;
            }
        }

        Ok(())
    }

    /// Returns a reference to the inner [`Pdb`].
    #[must_use]
    pub fn pdb(&self) -> &Pdb {
        &self.pdb
    }
}

impl<P> SolverT<P> for Solver
where
    P: SlidingPuzzle,
    P::Piece: AsPrimitive<u8>,
{
    type Context = ();

    fn is_initialized_with_context(&self, _context: &Self::Context) -> bool {
        true
    }

    fn init_with_context(&mut self, _context: &Self::Context) {}

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
        _context: &Self::Context,
    ) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}
