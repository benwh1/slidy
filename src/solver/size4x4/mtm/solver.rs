//! Defines the [`Solver`] struct for optimally solving 4x4 puzzles using pattern databases.

use std::ops::ControlFlow;

use num_traits::AsPrimitive;

use crate::{
    algorithm::{axis::Axis, direction::Direction, metric::Mtm},
    puzzle::{sliding_puzzle::SlidingPuzzle, small::Puzzle4x4},
    solver::{
        config::{PdbConfig, SolverConfig},
        directions::DIRECTIONS,
        residue,
        search::{Search, iterative_deepening},
        size4x4::mtm::{
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
    fn with_tables_and_pdb(indexing_table: IndexingTable, pdb: Pdb) -> Self {
        Self {
            indexing_table,
            pdb,
            search: Search::new(),
            puzzle: FourBitPuzzle::new(),
        }
    }

    fn with_pdb(pdb: Pdb) -> Self {
        Self::with_tables_and_pdb(IndexingTable::new(), pdb)
    }

    /// Creates a new [`Solver`] and builds the pattern database.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        let indexing_table = IndexingTable::new();
        let pdb = Pdb::new(&indexing_table, config);

        Self::with_tables_and_pdb(indexing_table, pdb)
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

    /// Returns the [`Pdb`] entry of `puzzle`.
    #[inline(always)]
    fn entry(&self, puzzle: &ReducedFourBitPuzzle) -> u64 {
        let coord = self.indexing_table.encode(puzzle.pieces) as usize;

        // SAFETY: We have a test which guarantees that every `ReducedFourBitPuzzle` encodes to an
        // index that is within bounds.
        u64::from(unsafe { self.pdb.get_unchecked(coord) })
    }

    /// Returns the distance of `puzzle` from solved, by following moves that get closer to it.
    fn distance(&self, puzzle: ReducedFourBitPuzzle) -> u64 {
        residue::walk_distance(
            puzzle,
            self.entry(&puzzle),
            |puzzle| puzzle.pieces == ReducedFourBitPuzzle::SOLVED,
            |puzzle, closer| {
                DIRECTIONS
                    .into_iter()
                    .find_map(|dir| {
                        let mut child = *puzzle;
                        while child.do_move(dir) {
                            if self.entry(&child) == closer {
                                return Some(child);
                            }
                        }
                        None
                    })
                    .expect("pattern database has no move closer to solved")
            },
        )
    }

    /// Finds solutions of length `depth` of a state with reduced puzzle `puzzle` and transposed
    /// reduced puzzle `transposed_puzzle`. `parent_h` and `parent_transposed_h` are the distances
    /// of the parent's reduced puzzles from solved. For the starting state, which has no parent,
    /// they are one more than its own distances, which decode to the right values.
    fn dfs(
        &mut self,
        depth: u64,
        last_axis: Option<Axis>,
        puzzle: ReducedFourBitPuzzle,
        transposed_puzzle: ReducedFourBitPuzzle,
        parent_h: u64,
        parent_transposed_h: u64,
    ) -> ControlFlow<()> {
        let h = residue::neighbor_distance(parent_h, self.entry(&puzzle));
        if h > depth {
            return ControlFlow::Continue(());
        }

        let transposed_h =
            residue::neighbor_distance(parent_transposed_h, self.entry(&transposed_puzzle));
        if transposed_h > depth {
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

            let mut puzzle = puzzle;
            let mut transposed_puzzle = transposed_puzzle;

            while puzzle.do_move(dir) {
                transposed_puzzle.do_move(transposed_dir);
                count += 1;

                self.search.stack.push(dir);

                if self
                    .dfs(
                        depth - 1,
                        Some(dir.into()),
                        puzzle,
                        transposed_puzzle,
                        h,
                        transposed_h,
                    )
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

        let h = self.distance(reduced_puzzle);
        let transposed_h = self.distance(transposed_reduced_puzzle);

        iterative_deepening::<Mtm, _, _>(
            self,
            |s| &mut s.search,
            config,
            h.max(transposed_h),
            |s, depth| {
                s.dfs(
                    depth,
                    None,
                    reduced_puzzle,
                    transposed_reduced_puzzle,
                    h + 1,
                    transposed_h + 1,
                )
            },
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

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::puzzle::puzzle::Puzzle;

    #[test]
    fn test_optimal_lengths() {
        // Scrambled with 30 random moves, with lengths checked against `GenericSolver` using
        // `MtmHeuristic(ManhattanDistance(RowGrids))`.
        let cases = [
            ("1 6 4 8/5 0 3 11/9 7 2 12/13 10 14 15", 13),
            ("0 1 3 12/7 8 4 11/5 2 10 15/6 9 13 14", 17),
            ("2 6 3 4/5 1 11 0/9 14 8 7/13 15 10 12", 15),
            ("2 4 0 12/1 7 3 8/14 5 6 10/9 13 11 15", 21),
            ("5 1 3 4/10 9 2 7/13 14 6 11/15 0 12 8", 14),
            ("5 1 3 4/2 6 8 0/13 14 7 15/11 9 12 10", 19),
            ("5 7 1 8/2 11 3 6/9 10 12 4/13 0 14 15", 18),
            ("1 2 4 6/5 9 10 3/13 15 0 11/14 7 12 8", 18),
        ];

        let mut solver = Solver::default();

        for (state, length) in cases {
            let mut puzzle = Puzzle::from_str(state).unwrap();
            let solution = solver.solve(&puzzle).unwrap();
            assert_eq!(solution.len_mtm(), length, "state {state}");

            puzzle.apply_alg(&solution);
            assert!(puzzle.is_solved(), "state {state}");
        }
    }

    #[test]
    fn test_solve_all_optimal() {
        let cases: [(&str, &[&str]); 2] = [
            (
                "5 8 7 9/0 1 13 14/15 6 2 4/11 12 10 3",
                &[
                    "L3UR2D2L2U3R3DL2UR2D3L3U3RD3LUR3ULD2RU2L3UR2D3L2UR3UL3U",
                    "L3UR2D2L2U3R3DL2UR2D3L3U3RD3LUR3UL3UR2D3RU2LD2L2UR3UL3U",
                    "L3U2R2D3L2U3RD2R2U2L2D3RU2RDL3UR3DL3UR2URD3LU3LD3RU2L2U",
                    "L3U2R2D3L2U3R2DLDR2U2L2D3RU2RDL3UR3DL3UR3D2LU3LD3RU2L2U",
                    "L3U2R3D2L3DR3UL3UR3D2LUL2U2R3D2L3DR2U2L2D2R2U2L2D2RU3R2D3L3U3",
                    "L3DR3ULU2RD2L3URURD3RUL3U2R3D3L3U3R2D3L2UR2DL2UR3UL3U",
                    "L3DR3UL3U2R2D2L2DRU3R2DL3D2RU3RD3LU2LUR3D2L2ULUR3DL3U",
                    "L3DR3UL3U2R3D2L3U2R3D2L3DRURDLU3LDR3DL3U2R2DL2D2R2U2L2U",
                ],
            ),
            (
                "7 9 14 2/0 15 13 1/11 8 10 3/6 4 5 12",
                &["L3U2R3D2LU2L2DR2D2LU3R2D3L2U3R2D3L3U2R3DL3UR3DL2U2RD3L2U3"],
            ),
        ];

        let mut solver = Solver::default();

        for (state, expected) in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            let solutions = solver
                .solve_all_optimal(&puzzle)
                .unwrap()
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            assert_eq!(solutions, expected, "state {state}");
        }
    }
}
