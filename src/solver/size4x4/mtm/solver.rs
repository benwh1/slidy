//! Defines the [`Solver`] struct for optimally solving 4x4 puzzles using pattern databases.

use std::ops::ControlFlow;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{axis::Axis, direction::Direction, metric::Mtm},
    puzzle::{sliding_puzzle::SlidingPuzzle, small::Puzzle4x4},
    solver::{
        config::{PdbConfig, SolverConfig},
        search::{Search, iterative_deepening},
        size4x4::mtm::{
            base_5_table::Base5Table,
            indexing_table::IndexingTable,
            pdb::Pdb,
            puzzle::{FourBitPuzzle, ReducedFourBitPuzzle},
        },
        solver::{Solver as SolverT, SolverError},
    },
};

/// An optimal solver for 4x4 puzzles in [`Mtm`].
///
/// [`Mtm`]: Mtm
pub struct Solver {
    indexing_table: IndexingTable,
    base_5_table: Base5Table,
    pdb: Pdb,
    search: Search<128>,
    puzzle: FourBitPuzzle,
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
            search: Search::new(),
            puzzle: FourBitPuzzle::new(),
        }
    }

    fn with_pdb(pdb: Pdb) -> Self {
        Self::with_tables_and_pdb(IndexingTable::new(), Base5Table::new(), pdb)
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
        // SAFETY: Checks are performed in `Pdb::try_from_bytes` which almost certainly detects
        // invalid data, but ultimately the caller is responsible.
        let pdb = unsafe { Pdb::try_from_bytes(bytes) }?;

        Some(Self::with_pdb(pdb))
    }

    /// See [`Self::try_with_pdb_bytes`].
    ///
    /// # Safety
    ///
    /// The caller is responsible for the correctness of the data contained in `bytes`. No
    /// correctness checks are performed.
    #[must_use]
    pub unsafe fn with_pdb_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        // SAFETY: Responsibility of the caller.
        Self::with_pdb(unsafe { Pdb::from_bytes_unchecked(bytes) })
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
            for dir in self.search.stack.iter() {
                p.do_move(dir);
            }

            if p.pieces() == Puzzle4x4::SOLVED {
                return self.search.record_solution();
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

                self.search.stack.push(dir);

                if self
                    .dfs(depth - 1, Some(dir.into()), puzzle, transposed_puzzle)
                    .is_break()
                {
                    return ControlFlow::Break(());
                }
            }

            self.search.stack.remove_n(count);
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

        self.puzzle = four_bit_puzzle;

        let coord = self
            .indexing_table
            .encode(reduced_puzzle.pieces, &self.base_5_table);
        let hval = self.pdb.get(coord as usize) as u64;

        iterative_deepening::<Mtm, _, _>(
            self,
            |s| &mut s.search,
            config,
            hval,
            |s, depth| s.dfs(depth, None, reduced_puzzle, transposed_reduced_puzzle),
        );

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

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &P,
        config: SolverConfig,
        _context: &Self::Context,
    ) -> Result<(), SolverError> {
        self.solve_impl(puzzle, config)
    }
}
