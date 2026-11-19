//! The optimal 4x4 STM solver: an IDA* search over any [`Pdb`], and slidy's
//! [`Solver`](SolverT) interface.
//!
//! The search keeps the board in both orientations (`[direct, mirrored]`, mirrored across the
//! main diagonal), generates successors through the move table and an automaton over moves,
//! and asks the pattern database to shift its indices (prefetching the entries) and to decode,
//! prune and rank each successor. Each node's successors are generated before it is visited,
//! while its siblings are generated or searched, so their table lines load in the meantime. The
//! search is monomorphised per [`Pdb`] and every lookup is inlined into it.

mod automaton;

use std::{mem::MaybeUninit, ops::ControlFlow};

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::metric::Stm,
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
    solver::{
        config::{PdbConfig, SolverConfig},
        search::{Search, iterative_deepening},
        size4x4::stm::{
            pdb::Pdb,
            solver::automaton::{Automaton, Backtrack, Pruning},
            utils::{self, Board, MIRRORED_CELL, MOVES},
        },
        solver::{Solver as SolverT, SolverError},
    },
};

/// An optimal solver for 4x4 puzzles in [`Stm`], with pattern database `P` (one of those in
/// [`pdb`](super::pdb), which trade memory for speed).
pub struct Solver<P: Pdb> {
    pdb: P,
    search: Search<128>,
    /// The position being searched, in both orientations.
    boards: [Board; 2],
    /// `path[depth]`: the blank's cell at each depth of the current path.
    path: Vec<u8>,
}

/// One successor: its indices, the blank's cell and the moved tile (in both orientations), and
/// the automaton's node after the move.
struct Candidate<P: Pdb> {
    indices: P::Indices,
    blanks: [u8; 2],
    tiles: [u8; 2],
    node: u32,
}

/// A node's values and its successors.
struct ExpandedNode<P: Pdb> {
    values: P::Values,
    candidates: [MaybeUninit<Candidate<P>>; 4],
    count: usize,
}

impl<P: Pdb> Default for Solver<P> {
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<P> Solver<P>
where
    P: Pdb,
{
    /// Creates a new [`Solver`] and builds the pattern database.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        Self::with_pdb(P::new(config))
    }

    /// Creates a new [`Solver`] that searches with `pdb`.
    #[must_use]
    pub fn with_pdb(pdb: P) -> Self {
        Self {
            pdb,
            search: Search::new(),
            boards: [[0; 16]; 2],
            path: Vec::new(),
        }
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
        let pdb = unsafe { P::try_from_bytes(bytes) }?;
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
        Self::with_pdb(unsafe { P::from_bytes_unchecked(bytes) })
    }

    /// Returns a reference to the inner pattern database.
    #[must_use]
    pub const fn pdb(&self) -> &P {
        &self.pdb
    }

    fn solve_impl<Puz>(&mut self, puzzle: &Puz, config: SolverConfig) -> Result<(), SolverError>
    where
        Puz: SlidingPuzzle,
    {
        if puzzle.size() != Size::new(4, 4).unwrap() {
            return Err(SolverError::IncompatiblePuzzleSize);
        }

        if !puzzle.is_solvable() {
            return Err(SolverError::Unsolvable);
        }

        let board: Board =
            std::array::from_fn(|cell| puzzle.piece_at(cell as u64).to_u8().unwrap());
        let internal = utils::to_internal(&board);
        let boards = [internal, utils::reflect(&internal)];
        let blank = utils::blank_position(&internal);
        let (indices, values, bound) = self.pdb.root(&boards);
        // Every solution's length has the parity of the blank's distance from home, which in
        // the serpentine order is its cell's parity.
        let lower_bound = bound + ((bound ^ blank as u64) & 1);

        // The duplicate-pruning automaton finds an optimal solution but not every solution, so
        // it serves only searches for one solution that start at or below the optimal length.
        let pruning = config.num_solutions == 1 && config.min <= lower_bound;

        iterative_deepening::<Stm, _, _>(
            self,
            |solver| &mut solver.search,
            config,
            lower_bound,
            |solver, depth| {
                // Every solution's length is at least the bound and has its parity.
                if depth < lower_bound || (depth ^ lower_bound) & 1 != 0 {
                    return ControlFlow::Continue(());
                }
                solver.boards = boards;
                solver.path.resize(depth as usize + 1, 0);
                if depth == 0 {
                    return solver.record(blank);
                }
                let mut root = MaybeUninit::uninit();
                if pruning {
                    solver.expand::<Pruning>(blank, &indices, Pruning::START, values, &mut root);
                    // SAFETY: `expand` wrote it.
                    solver
                        .search_subtree::<Pruning>(blank, depth, unsafe { root.assume_init_ref() })
                } else {
                    solver.expand::<Backtrack>(
                        blank,
                        &indices,
                        Backtrack::START,
                        values,
                        &mut root,
                    );
                    // SAFETY: `expand` wrote it.
                    solver.search_subtree::<Backtrack>(blank, depth, unsafe {
                        root.assume_init_ref()
                    })
                }
            },
        );

        Ok(())
    }

    /// Puts `tiles` on `to` and the blank on `from`, on both boards: a move, or with the cells
    /// swapped its undo.
    #[inline(always)]
    fn slide(&mut self, to: [u8; 2], from: [u8; 2], tiles: [u8; 2]) {
        for side in 0..2 {
            self.boards[side][usize::from(to[side]) & 15] = tiles[side];
            self.boards[side][usize::from(from[side]) & 15] = 0;
        }
    }

    /// Records the path ending with the blank at `blank` (home) as a solution. Out of line: the
    /// search's hot loop measured 3% slower with it inlined.
    #[cold]
    #[inline(never)]
    fn record(&mut self, blank: usize) -> ControlFlow<()> {
        // `path[depth]`: the blank's cell with `depth` moves left, so the root's is the last.
        self.path[0] = blank as u8;
        let stack = &mut self.search.stack;
        stack.clear();
        let (root, trail) = self.path.split_last().unwrap();
        let trail = trail.iter().rev().map(|&cell| usize::from(cell));
        for direction in utils::directions(usize::from(*root), trail) {
            stack.push(direction);
        }
        self.search.record_solution()
    }

    /// Searches the subtree of the node with the blank at `blank` and `depth` moves left, whose
    /// values and successors are `node`, for solutions, recording each; breaks when the search
    /// should stop.
    fn search_subtree<A>(
        &mut self,
        blank: usize,
        depth: u64,
        node: &ExpandedNode<P>,
    ) -> ControlFlow<()>
    where
        A: Automaton,
    {
        self.path[depth as usize] = blank as u8;
        let blank_cells = [blank as u8, MIRRORED_CELL[blank & 15]];

        // The successors within the bound (the survivors), best first, and their values.
        let mut sort_keys = [0u64; 4];
        let mut values = [MaybeUninit::<P::Values>::uninit(); 4];
        let mut survivors = [0usize; 4];
        let mut survivor_count = 0;
        for candidate_index in 0..node.count {
            // SAFETY: the first `count` candidates were written.
            let candidate = unsafe { node.candidates[candidate_index].assume_init_ref() };
            let evaluation = self.pdb.evaluate(
                &node.values,
                &candidate.indices,
                candidate.blanks,
                candidate.tiles,
                depth,
            );
            if let Some((sort_key, candidate_values)) = evaluation {
                sort_keys[candidate_index] = sort_key;
                values[candidate_index].write(candidate_values);
                // Insertion sort: at most four entries.
                let mut position = survivor_count;
                while position > 0 && sort_keys[survivors[position - 1]] > sort_key {
                    survivors[position] = survivors[position - 1];
                    position -= 1;
                }
                survivors[position] = candidate_index;
                survivor_count += 1;
            }
        }
        let survivors = &survivors[..survivor_count];

        if depth == 1 {
            // A survivor's estimate is 0: it is solved.
            for &candidate_index in survivors {
                // SAFETY: as above.
                let candidate = unsafe { node.candidates[candidate_index].assume_init_ref() };
                self.record(usize::from(candidate.blanks[0]))?;
            }
            return ControlFlow::Continue(());
        }

        // Generate every survivor's successors first, so that their table lines load while the
        // later ones are generated and the earlier ones searched.
        let mut children = [const { MaybeUninit::<ExpandedNode<P>>::uninit() }; 4];
        for (child, &candidate_index) in children.iter_mut().zip(survivors) {
            // SAFETY: survivors' candidates and values were written.
            let (candidate, candidate_values) = unsafe {
                (
                    node.candidates[candidate_index].assume_init_ref(),
                    values[candidate_index].assume_init(),
                )
            };
            self.slide(blank_cells, candidate.blanks, candidate.tiles);
            self.expand::<A>(
                usize::from(candidate.blanks[0]),
                &candidate.indices,
                candidate.node,
                candidate_values,
                child,
            );
            self.slide(candidate.blanks, blank_cells, candidate.tiles);
        }
        for (child, &candidate_index) in children.iter().zip(survivors) {
            // SAFETY: as above, and every survivor's successors were written.
            let (candidate, child) = unsafe {
                (
                    node.candidates[candidate_index].assume_init_ref(),
                    child.assume_init_ref(),
                )
            };
            self.slide(blank_cells, candidate.blanks, candidate.tiles);
            let child_blank = usize::from(candidate.blanks[0]);
            let flow = self.search_subtree::<A>(child_blank, depth - 1, child);
            self.slide(candidate.blanks, blank_cells, candidate.tiles);
            flow?;
        }
        ControlFlow::Continue(())
    }

    /// Writes the node with the blank at `blank`, indices `indices`, automaton node `node` and
    /// values `values` to `out`: its values and its successors, each move the automaton allows.
    /// Written in place, field by field: returned by value, the node was copied.
    #[inline(always)]
    fn expand<A>(
        &self,
        blank: usize,
        indices: &P::Indices,
        node: u32,
        values: P::Values,
        out: &mut MaybeUninit<ExpandedNode<P>>,
    ) where
        A: Automaton,
    {
        let moves = &MOVES[blank & 15][(node & 15) as usize];
        let out = out.as_mut_ptr();
        // SAFETY: `out` points to an `ExpandedNode`, whose fields are written without reading
        // them, and whose candidates are `MaybeUninit`, valid whatever they hold.
        let candidates = unsafe {
            (&raw mut (*out).values).write(values);
            (&raw mut (*out).count).write(usize::from(moves.count));
            &mut (*out).candidates
        };
        for (candidate, tile_move) in candidates
            .iter_mut()
            .zip(&moves.moves[..usize::from(moves.count)])
        {
            let tiles = [
                self.boards[0][usize::from(tile_move.blank_after[0]) & 15],
                self.boards[1][usize::from(tile_move.blank_after[1]) & 15],
            ];
            candidate.write(Candidate {
                indices: self.pdb.shifted(indices, &self.boards, tile_move, tiles),
                blanks: tile_move.blank_after,
                tiles,
                node: A::next(node, tile_move.direction),
            });
        }
    }
}

impl<Puz, P> SolverT<Puz> for Solver<P>
where
    Puz: SlidingPuzzle,
    P: Pdb,
{
    type Context = ();

    fn solve_with_config_and_context(
        &mut self,
        puzzle: &Puz,
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
    use crate::{
        algorithm::algorithm::Algorithm, puzzle::puzzle::Puzzle, solver::size4x4::stm::pdb::Small,
    };

    /// Random positions and every optimal solution of each, from `slidy generate -n 8 --seed 1
    /// --random-state` and `slidy solve --all-optimal` (slidy-cli 0.5.0).
    const ALL_OPTIMAL: [(&str, &[&str]); 10] = [
        ("1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0", &[""]),
        (
            "13 12 4 2/7 11 15 3/9 10 5 0/8 14 6 1",
            &[
                "DRULD2RU3LDRURDRULDRDLDLU2RDL2UR3D2LUL2UR3D2LULURURDLUL2",
                "DRULD2RU3LDRUR2DLURD2LDLU2RDL2UR3D2LUL2UR3D2LULURURDLUL2",
            ],
        ),
        (
            "3 9 2 15/12 0 8 14/11 6 7 1/4 10 13 5",
            &[
                "DLU2LUR2D2RU2LDL2D2RUR2U2L3D2R3DLURUL2D2RURULUL2D3R3U3L3",
                "DLU2LD2RUR2U2LDLDR2UL2ULD2R3DLURULULD3RU3RDLUL2D3R3U3L3",
                "DLU2R2ULD2RUL3UR3DL3D2RUR2DLURUL2ULD2RDRURULUL2D3R3U3L3",
                "RU2L2DLD2RURURUL3D2RUR2D2LULU2RDLULD2RDRURDL3UR2URDLDLU2RUL2",
                "RU2L2DLD2RURURD2LU2LULD2R2U2L2D2RDRURULURD3L3UR2URDLDLU2RUL2",
                "RU2L2DLD2RURURD2LU2LULD2R2URUL3D2RURULD3RURDL3UR2URDLDLU3L",
                "RU2L2DLD2RURURD2LU2LULD2R2URUL3D2RDRURDL3URURULD2RURDLDLU3L",
                "RU2L2DLD2RURURD2LU2LULD2R2URUL3DR2ULDLDRDRURDL3UR2URDLDLU3L",
                "RU2L2DLD2RURURD2LU2L2DR2URULDLULD2RU2RDLD2RURDL3UR2URDLDLU3L",
                "RU2L2DLD2RURURD2LU2L2DR2URULDLULD2RDRURDL3URU2RDLDRURDLDLU3L",
                "RU2L2DLD2RURURD2LU2L2DR2URULDLULDRURDL2DRDRURDL3UR2URDLDLU3L",
                "RU2L2DLD2RURURD2LU2RULDL2DR2ULULD2RU2RDLD2RURDL3UR2URDLDLU3L",
                "RU2L2DLD2RURURD2LU2RULDL2DR2ULULD2RDRURDL3URU2RDLDRURDLDLU3L",
                "RU2L2DLD2RURURD2LU2RULDL2DR2ULULDRURDL2DRDRURDL3UR2URDLDLU3L",
                "RU2LDL2D2RURULULD2R3UL2ULD3RUR2UL2DRDLULDR3ULU2RD2L2DRU3L2",
                "RU2LDL2D2RURULULD2R3UL2ULD3RUR2UL2DRDLULDR3UL2DRU3RD2LU2L2",
                "RU2LDL2D2RUR2UL2ULD2R3DLU2LULD3R2URUL2DLDRUR2DL2URU2RD2LU2L2",
                "RU2LDRD2L2U2LD2RURURUL3D2R3DL2U3LD3R2ULUR2ULDRDL2DR2ULU2L2",
                "RULULDLD2RUR2U2LDLULD2RU2LD3RURURDLULDRDLULDR3U2LDLDRURULUL2",
                "RULULDLD2RUR2U2LDLULD2RU2LD3RURURDLULDRDLULDR3ULURDL2DRU3L2",
                "RULULDLD2RUR2U2LDLULD2RU2LD3RURURDLULDRDLULDR3UL2DRU2RDLU2L2",
                "RULURD3L2U2RULDLD2RURURUL3D2R3DL2U3LD3R2ULUR2DL2DR2ULU2L2",
                "RUL3D2RURU2L2D2R3U2LDLULD3RURURDLULDRDLULDR3U2LDLDRURULUL2",
                "RUL3D2RURU2L2D2R3U2LDLULD3RURURDLULDRDLULDR3ULURDL2DRU3L2",
                "RUL3D2RURU2L2D2R3U2LDLULD3RURURDLULDRDLULDR3UL2DRU2RDLU2L2",
                "RDL2U2LD2RUR2DLU2LULD2RDRU2RUL2DR2DLU2RDLUL2D2R2DLULDR3U3L3",
                "RDL2UR2DLU2L2D2R2U2LULD2R2URUL2DR2DLU2RDLUL2D2R2DLULDR3U3L3",
            ],
        ),
        (
            "9 0 11 1/15 3 7 2/5 14 8 13/10 12 6 4",
            &[
                "ULU2RDL2UR2DLD2LU2RDRDL2U2RDR2ULDRDL2U2RURD2LU2LDLU",
                "ULU2RDL2UR2DLD2LU2RDRDL2UR3ULDRDL2U2RURD2LU2LD2LU2",
                "ULU2RDL2UR2DLD2LU2RDRDL2UR3ULDRDL2ULUR2URD2LU2LDLU",
                "ULU2RDL2UR2DLD2LU2RDR2ULD2L2U2RDR2DL2U2RURD2LU2LDLU",
                "ULU2RDL2UR2DLD2LU2RDR2ULD2L2UR3DL2U2RURD2LU2LD2LU2",
                "ULU2RDL2UR2DLD2LU2RDR2ULD2L2UR3DL2ULUR2URD2LU2LDLU",
                "URULDLULUR2DLD2LU3R2D3L2U2RDR2DL2ULUR3ULDRDL2ULU",
                "URULDLULUR2DLD2LU3R2D3L2URULDR3DL2U2R2ULDRDL2ULU",
                "URULDLULUR2DLD2LU3R2D3L2UR3DL2U2LDRUR2ULDRDL2ULU",
                "URULDLULUR2DLD2LU3R2DRULD3L2U2RDR2DL2ULUR3DL2ULU",
                "URULDLULUR2DLD2LU3R2DRULD3L2URULDR3DL2U2R2DL2ULU",
                "URULDLULUR2DLD2LU3R2DRULD3L2UR3DL2U2LDRUR2DL2ULU",
                "URULDLULUR2DLD2LU3R3DLD2L2U2RDR2DL2ULUR2URD2L2ULU",
                "URULDLULUR2DLD2LU3R3DLD2L2URULDR3DL2U2RURD2L2ULU",
                "URULDLULUR2DLD2LU3R3DLD2L2UR3DL2U2LDRURURD2L2ULU",
                "L2U3RD2LURDR2ULD2L2U3R2DL2UR2D2RDL2U2R2ULDRDL3U2",
                "L2U3RD2LURDR2ULD2L2U3R2DL2UR2DRULD2RDL2U2R2DL3U2",
                "L2U3RD2LURDR2ULD2L2U3R2DL2UR3DLDRDL2U2RURD2L3U2",
                "L2URU2RDL2UR2DLDRDL2UR3ULDRDL3U2R2URD2LU2LD3LU3",
                "L2URU2RDL2UR2DLDR2ULD2L2UR3DL3U2R2URD2LU2LD3LU3",
                "L2UR2DL2URU2RDL2UR2DLDR2ULDRDL3U2R2URD2LU2LD3LU3",
                "L2UR3UL2URD3L2URULURDRULDRDRDL3U2R3ULDRDL2DLU3",
                "L2UR3UL2URD3L2URULURDRULDR2ULD2RDL3U2R3DL2DLU3",
                "L2UR3UL2URD3L2URULUR2DLURD2RDL3U2R3ULDRDL2DLU3",
                "L2UR3UL2URD3L2URULUR2DLURDRULD2RDL3U2R3DL2DLU3",
                "L2UR3UL2URD3L2URULUR2DLUR2DLDRDL3U2R2URD2L2DLU3",
            ],
        ),
        (
            "9 10 5 3/4 13 12 2/8 7 6 0/11 14 15 1",
            &[
                "URD2LUR2D2LU3RD2RU2LDLDLDRUR2DL3UR2URD2L3URURUL2",
                "URD2LUR2D2LULDRU3RD2RU2LDLDR2DL3UR2URD2L3URURUL2",
            ],
        ),
        (
            "3 8 13 2/12 14 6 1/10 4 0 5/15 11 7 9",
            &[
                "RDLULDRDRURULDLULUR2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDLULDRDRURULDLULUR2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RDLULDRDRURULDLDLURULUR2DL2UR3DLDRUL3UR3DLDLDR2UL2DRULULU",
                "RDL2URD2RURULDL2U2R2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDL2URD2RURULDL2U2R2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RDLDRUL2UR2URD2L3URURDRDL2DLURDR2UL2DRULU2LDR2UL2DR2URDLUL2",
                "RDLDRUL2UR2URD2L3URURDRDL2DR2ULDL2UR2DLU3LDR2UL2DR2URDLUL2",
                "RDLDRURULDLULD2RU2LUR2DL2UR3DLDRUL3UR3DLDL2DR3UL2DRULULU",
                "RDLDRURULDLULDRULUR2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDLDRURULDLULDRULUR2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RDLDRURULDLULDRDLURULUR2DL2UR3DLDRUL3UR3DLDLDR2UL2DRULULU",
                "RDLDRURULDL2URDLU2R2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDLDRURULDL2URDLU2R2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RD2LU2LDR3ULD2L2URULUR2DL2UR3DLDRUL3UR3DLDLDR2UL2DRULULU",
                "RD2LULUR2DL2UR2URD2LDRUL2U2RDRDLDL2UR2DLU3LDR2UL2DR2URDLUL2",
                "RDRULDLULDRDRULULUR2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDRULDLULDRDRULULUR2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RDRULDLULDRDRULDLURULUR2DL2UR3DLDRUL3UR3DLDLDR2UL2DRULULU",
                "RDRULDL2URD2RUL2U2R2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDRULDL2URD2RUL2U2R2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RDRULDLDRULULD2RU2LUR2DL2UR3DLDRUL3UR3DLDL2DR3UL2DRULULU",
                "RDRULDLDRULULDRULUR2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDRULDLDRULULDRULUR2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RDRULDLDRULULDRDLURULUR2DL2UR3DLDRUL3UR3DLDLDR2UL2DRULULU",
                "RDRULDLDRUL2URDLU2R2DL2UR3DLDRUL3UR3DLDLDLURDR2UL2DRULULU",
                "RDRULDLDRUL2URDLU2R2DL2UR3DLDRUL3UR3DLDLDR2ULDL2UR2DLU2LU",
                "RDRULD2LU2LDR2DL2URULUR2DL2UR3DLDRUL3UR3DLDLDR2UL2DRULULU",
                "RDRULD2LU2RD2LULU2RDLUR3D2L2UR2DL3DR3UL3URUR2D2L2DRUL2U2",
                "RDRULD2LU2RD2LULURULDRUR2D2L2UR2DL3DR3UL3URUR2D2L2DRUL2U2",
                "RDRULD2LULUR2DL2U2RDLUR3D2LDRUL2UR2DLDL2U2RDRULUR2DLD2LULU2",
                "RDRULD2LULUR2DL2URULDRUR2D2LDRUL2UR2DLDL2U2RDRULUR2DLD2LULU2",
                "RDRULD2LULUR2DL2URULDR2D2L2U2RUR2DLDRUL2UR2DLDLDR2UL2DRUL2U2",
                "R2DL2DRUL2UR3DL3U2R2DL2UR3D2LUL2UR3DLDLDLURDR2UL2DRULULU",
                "R2DL2DRUL2UR3DL3U2R2DL2UR3D2LUL2UR3DLDLDR2ULDL2UR2DLU2LU",
                "R2DL2DRUL2UR3DL3U2R2DL2UR3D2L2DLURDR2ULUL2UR3DLDLDRULULU",
                "R2DL2DRUL2UR3DL3U2R2DL2UR3D2L2DLURDR2UL2DRU2L2UR3DLDLULU",
                "R2DL2DRUL2UR3DL3U2R2DL2UR3D2L2DR2ULUL2UR3DLD2L2UR2DLU2LU",
                "R2DL2DRUL2UR3DL3U2R2DL2UR3D2L2DR2ULDL2UR2UL2UR3DLD2LU2LU",
            ],
        ),
        (
            "9 4 2 15/6 1 12 8/10 3 13 0/11 5 7 14",
            &[
                "DRDRU3LDLD2RU2LDR2U2L2DR3ULDRD2L2U2R2DLU2L2",
                "DRDRU3LDLD2RU2LDR3DLU3L2DR3ULDRDLDLU2RUL2",
                "DRDRU3LDLD2RU2LDR3DL2URU2L2DR3ULDRDL2URUL2",
                "DRDRU3LDLD2RUR2DL2U2LDR2U2L2DR3ULDRDL2URUL2",
                "DRDRURDLU3LDLD2RU2LDR2U2L2DR3ULDRDLDLU2RUL2",
            ],
        ),
        (
            "14 6 10 11/1 5 0 9/15 3 13 7/2 4 12 8",
            &[
                "URDRDLULURURD2LU2RD3LULDRU2LDLDR2UL2U2R3DLDRUL2DLU2",
                "RULDR2DLU3RD2LU2RD3LULDRU2LDLDR2UL2U2R3DLDRUL2DLU2",
                "R2DLU2LDRU2RD2LU2RD3LULDRU2LDLDR2UL2U2R3DLDRUL2DLU2",
            ],
        ),
        (
            "14 6 7 4/10 0 9 11/8 12 15 3/13 5 1 2",
            &[
                "URUL2DRURDLDLURUL2DRULD2RDR2U2LD2RU2LDLU2RD3LU2RD2LU2LU",
                "URUL2DRURDLDLURULDLURDLDRDR2U2LD2RU2LDLU2RD3LU2RD2LU2LU",
                "URUL2DRDLURUL2DRULD2R2URD2LULU2RDRD2L2U2R2ULD3LUR2UL3U",
                "URUL2DRDLURULDLURDLDR2URD2LULU2RDRD2L2U2R2ULD3LUR2UL3U",
                "URUL2DRDLURURDLUL2DRULD2RDR2U2LD2RU2LDLU2RD3LU2RD2LU2LU",
                "URUL2DRDLURURDLULDLURDLDRDR2U2LD2RU2LDLU2RD3LU2RD2LU2LU",
                "LU2LD2RUR2DL2U2LD2RUR2DLDRULDLU3LD2RURURDLD2LUR2U2LDLUL",
                "LU2LD2RUR2DL2U2LD2RUR2D2LURDL2U3LD2RURURDLD2LUR2U2LDLUL",
                "LULDRUR2DL2U2LD2RUR2D2LULURUL2D2RDRU3RD2LURD2LURU2LDLUL",
                "LUR2DLU2LDLURD2RU2LDLDR2URD2LULURURD2LURD2LULDRURU2LDL2U",
            ],
        ),
        (
            "1 11 14 15/0 9 4 12/3 10 7 8/13 5 6 2",
            &[
                "L2ULDRU2LDR2ULD3LU2RD2RU2RDLU2LD3RU3LD2LDR2UL2U2",
                "L2ULDRU2LDRD2LU2R2ULD3RU2RDLU2LD3RU3LD2LDR2UL2U2",
                "L2ULDRU2LDRD2LU2RD2RU3LD2RURDLU2LD3RU2LDLDR2UL2U2",
                "L2URUL2DRULD2R2URDLULULD2RDLU2RD2RU3LD3RU3LD2LU2",
                "L2URULDLDR2U2L2D2RDLU2RURDRDLULD2RU3LD3RU3LD2LU2",
                "L2URULDLDR2URDLU2L2D2RDLU2RURDLD2RU3LD3RU3LD2LU2",
                "L2URULDLURDLDR2URDLULULD2RDLU2RD2RU3LD3RU3LD2LU2",
                "LU2L2DRULD2R2URDLUL2URD3LU3RD3RU3LD3RU3LD2LU2",
                "LU2LDLURD2LURD2LU3RD3LU3RD2RURDLU2LD2RDLURULDLU2",
                "LU2LDLURDLDR2URDLUL2URD3LU3RD3RU3LD3RU3LD2LU2",
            ],
        ),
    ];

    #[test]
    fn test_solver() {
        let mut solver = Solver::<Small>::default();
        for (state, expected) in ALL_OPTIMAL {
            let puzzle = Puzzle::from_str(state).unwrap();
            let solutions = solver.solve_all_optimal(&puzzle).unwrap();

            for solution in &solutions {
                let mut solved = puzzle.clone();
                solved.apply_alg(solution);
                assert!(solved.is_solved(), "{puzzle}: {solution} does not solve it");
            }

            let mut solutions = solutions
                .iter()
                .map(Algorithm::to_string)
                .collect::<Vec<_>>();
            solutions.sort();

            let mut expected = expected.to_vec();
            expected.sort_unstable();

            assert_eq!(solutions, expected, "state {state}");

            let solution = solver.solve(&puzzle).unwrap().to_string();
            assert!(solutions.contains(&solution));
        }
    }
}
