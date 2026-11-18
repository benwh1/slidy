//! The optimal 4x4 STM solver: an IDA* search over any [`Pdb`], and slidy's
//! [`Solver`](SolverT) interface.
//!
//! The search keeps the board in both orientations (`[direct, mirrored]`, mirrored across the
//! main diagonal), generates successors through the move table and an [`automaton`] over moves,
//! and asks the pattern database to shift its indices (prefetching the entries) and to decode,
//! prune and rank each successor. Each node's successors are generated before it is visited,
//! while its siblings are generated or searched, so their table lines load in the meantime. The
//! search is monomorphised per [`Pdb`] and every lookup is inlined into it.

pub mod automaton;

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
            utils::{self, Board, MIRROR, MOVES},
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
struct Expansion<P: Pdb> {
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

        let board: Board = std::array::from_fn(|i| puzzle.piece_at(i as u64).to_u8().unwrap());
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
            |s| &mut s.search,
            config,
            lower_bound,
            |s, depth| {
                // Every solution's length is at least the bound and has its parity.
                if depth < lower_bound || (depth ^ lower_bound) & 1 != 0 {
                    return ControlFlow::Continue(());
                }
                s.boards = boards;
                s.path.resize(depth as usize + 1, 0);
                if depth == 0 {
                    return s.record(blank);
                }
                if pruning {
                    let root = s.expand::<Pruning>(blank, &indices, Pruning::START, values);
                    s.dfs::<Pruning>(blank, depth, &root)
                } else {
                    let root = s.expand::<Backtrack>(blank, &indices, Backtrack::START, values);
                    s.dfs::<Backtrack>(blank, depth, &root)
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
        // `path[d]`: the blank's cell with `d` moves left, so the root's is the last.
        self.path[0] = blank as u8;
        let stack = &mut self.search.stack;
        stack.clear();
        let (root, trail) = self.path.split_last().unwrap();
        let trail = trail.iter().rev().map(|&c| usize::from(c));
        for d in utils::directions(usize::from(*root), trail) {
            stack.push(d);
        }
        self.search.record_solution()
    }

    /// Searches the subtree of the node with the blank at `blank` and `depth` moves left, whose
    /// values and successors are `node`, for solutions, recording each; breaks when the search
    /// should stop.
    fn dfs<A>(&mut self, blank: usize, depth: u64, node: &Expansion<P>) -> ControlFlow<()>
    where
        A: Automaton,
    {
        self.path[depth as usize] = blank as u8;
        let here = [blank as u8, MIRROR[blank & 15]];

        // The successors within the bound, best first, and their values.
        let mut keys = [0u64; 4];
        let mut values = [MaybeUninit::<P::Values>::uninit(); 4];
        let mut order = [0usize; 4];
        let mut n = 0;
        for k in 0..node.count {
            // SAFETY: the first `count` candidates were written.
            let c = unsafe { node.candidates[k].assume_init_ref() };
            if let Some((key, v)) =
                self.pdb
                    .evaluate(&node.values, &c.indices, c.blanks, c.tiles, depth)
            {
                keys[k] = key;
                values[k].write(v);
                // Insertion sort: at most four entries.
                let mut j = n;
                while j > 0 && keys[order[j - 1]] > key {
                    order[j] = order[j - 1];
                    j -= 1;
                }
                order[j] = k;
                n += 1;
            }
        }

        if depth == 1 {
            // A survivor's estimate is 0: it is solved.
            for &k in &order[..n] {
                // SAFETY: as above.
                let c = unsafe { node.candidates[k].assume_init_ref() };
                self.record(usize::from(c.blanks[0]))?;
            }
            return ControlFlow::Continue(());
        }

        // Generate every survivor's successors first, so that their table lines load while the
        // later ones are generated and the earlier ones searched.
        let mut children = [const { MaybeUninit::<Expansion<P>>::uninit() }; 4];
        for (slot, &k) in order[..n].iter().enumerate() {
            // SAFETY: survivors' candidates and values were written.
            let (c, v) = unsafe {
                (
                    node.candidates[k].assume_init_ref(),
                    values[k].assume_init(),
                )
            };
            self.slide(here, c.blanks, c.tiles);
            let child = self.expand::<A>(usize::from(c.blanks[0]), &c.indices, c.node, v);
            children[slot].write(child);
            self.slide(c.blanks, here, c.tiles);
        }
        for (slot, &k) in order[..n].iter().enumerate() {
            // SAFETY: as above, and every survivor's successors were written.
            let (c, child) = unsafe {
                (
                    node.candidates[k].assume_init_ref(),
                    children[slot].assume_init_ref(),
                )
            };
            self.slide(here, c.blanks, c.tiles);
            let flow = self.dfs::<A>(usize::from(c.blanks[0]), depth - 1, child);
            self.slide(c.blanks, here, c.tiles);
            flow?;
        }
        ControlFlow::Continue(())
    }

    /// The node with the blank at `blank`, indices `indices`, automaton node `node` and values
    /// `values`: its values and its successors, each move the automaton allows.
    #[inline(always)]
    fn expand<A>(
        &self,
        blank: usize,
        indices: &P::Indices,
        node: u32,
        values: P::Values,
    ) -> Expansion<P>
    where
        A: Automaton,
    {
        let list = &MOVES[blank & 15][(node & 15) as usize];
        let mut out = Expansion {
            values,
            candidates: [const { MaybeUninit::uninit() }; 4],
            count: usize::from(list.n),
        };
        for (k, m) in list.moves[..out.count].iter().enumerate() {
            let tiles = [
                self.boards[0][usize::from(m.next[0]) & 15],
                self.boards[1][usize::from(m.next[1]) & 15],
            ];
            out.candidates[k].write(Candidate {
                indices: self.pdb.shifted(indices, &self.boards, m, tiles),
                blanks: m.next,
                tiles,
                node: A::next(node, m.dir),
            });
        }
        out
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
    use std::{str::FromStr as _, sync::LazyLock};

    use super::*;
    use crate::{
        algorithm::algorithm::Algorithm,
        puzzle::puzzle::Puzzle,
        solver::{config::SolverConfig, size4x4::stm::pdb::Tiny},
    };

    /// The tiny set's solver, built once and shared by the tests.
    static SOLVER: LazyLock<std::sync::Mutex<Solver<Tiny>>> =
        LazyLock::new(|| std::sync::Mutex::new(Solver::default()));

    fn solver() -> std::sync::MutexGuard<'static, Solver<Tiny>> {
        SOLVER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn test_solver() {
        let puzzle = Puzzle::from_str("12 15 5 1/11 9 2 13/0 10 8 6/14 7 4 3").unwrap();
        let mut solver = solver();

        let solution = solver.solve(&puzzle).unwrap();
        assert_eq!(solution.len_stm(), 58);
        let mut solved = puzzle.clone();
        solved.apply_alg(&solution);
        assert!(solved.is_solved());

        let solution = solver.solve(&puzzle).unwrap();
        drop(solver);
        assert_eq!(solution.len_stm(), 58);
    }

    #[test]
    fn test_solved() {
        let puzzle = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0").unwrap();
        assert_eq!(solver().solve(&puzzle), Ok(Algorithm::new()));
    }

    #[test]
    fn test_solve_all_optimal() {
        let puzzle = Puzzle::from_str("1 11 14 15/0 9 4 12/3 10 7 8/13 5 6 2").unwrap();
        let mut solutions = solver()
            .solve_all_optimal(&puzzle)
            .unwrap()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        solutions.sort();
        let expected = [
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
        ];
        assert_eq!(solutions, expected);
    }

    /// Every solution of a length, and a minimum above the optimum, match the old solver's.
    #[test]
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the solver is used until the end"
    )]
    fn test_matches_old_solver() {
        let puzzle = Puzzle::from_str("5 1 2 3/9 6 7 4/0 10 11 8/13 14 15 12").unwrap();
        let mut old = crate::solver::size4x4::stm::old::solver::Solver::default();
        let mut solver = solver();
        let optimal = solver.solve(&puzzle).unwrap().len_stm();
        assert_eq!(optimal, old.solve(&puzzle).unwrap().len_stm());

        let lengths = |solutions: Vec<Algorithm>| {
            let mut lengths = solutions.iter().map(Algorithm::len_stm).collect::<Vec<_>>();
            lengths.sort_unstable();
            lengths
        };
        let above = || SolverConfig {
            min: optimal + 1,
            num_solutions: 3,
            ..Default::default()
        };
        let ours = solver.solve_collect(&puzzle, above()).unwrap();
        for solution in &ours {
            let mut solved = puzzle.clone();
            solved.apply_alg(solution);
            assert!(solved.is_solved());
        }
        assert_eq!(
            lengths(ours),
            lengths(old.solve_collect(&puzzle, above()).unwrap())
        );

        let all = |length| SolverConfig {
            min: length,
            max: length,
            num_solutions: u64::MAX,
            ..Default::default()
        };
        for length in [optimal, optimal + 8] {
            let sorted = |solutions: Vec<Algorithm>| {
                let mut out = solutions
                    .iter()
                    .map(Algorithm::to_string)
                    .collect::<Vec<_>>();
                out.sort();
                out
            };
            let ours = sorted(solver.solve_collect(&puzzle, all(length)).unwrap());
            assert!(!ours.is_empty());
            assert_eq!(
                ours,
                sorted(old.solve_collect(&puzzle, all(length)).unwrap())
            );
        }
    }

    #[test]
    fn test_zero_solutions() {
        let mut solver = solver();
        let puzzle = Puzzle::from_str("6 12 0 3/11 1 10 4/9 7 8 13/15 14 2 5").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(solver.solve_collect(&puzzle, config), Ok(Vec::new()));

        let unsolvable = Puzzle::from_str("2 1 3 4/5 6 7 8/9 10 11 12/13 14 15 0").unwrap();
        let config = SolverConfig {
            num_solutions: 0,
            ..Default::default()
        };
        assert_eq!(
            solver.solve_collect(&unsolvable, config),
            Err(SolverError::Unsolvable)
        );
    }

    #[test]
    fn test_wrong_size() {
        let puzzle = Puzzle::new(Size::new(3, 3).unwrap());
        assert_eq!(
            solver().solve(&puzzle),
            Err(SolverError::IncompatiblePuzzleSize)
        );
    }
}
