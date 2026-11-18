//! The optimal 4x4 STM solver: an IDA* search over any [`Pdb`], and slidy's
//! [`Solver`](SolverT) interface.
//!
//! The search is table-agnostic: it keeps the board in both orientations (`[direct, mirrored]`,
//! mirrored across the main diagonal), generates successors through the move table and an
//! [`automaton`] over moves, and asks the pattern databases to shift their indices, prefetch
//! their entries, and decode, prune and rank each successor. The databases' values are carried
//! down the tree, so tables that hold only codes can decode a child's entries against its
//! parent's values. The search is monomorphised per [`Pdb`] and every lookup is inlined into it,
//! so the tables are chosen at compile time at no cost.

pub mod automaton;

use std::{mem::MaybeUninit, ops::ControlFlow};

use num_traits::ToPrimitive as _;

use crate::{
    algorithm::metric::Stm,
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
    solver::{
        config::SolverConfig,
        search::{Search, iterative_deepening},
        size4x4::stm::{
            pdb::Pdb,
            solver::automaton::{Automaton, Backtrack, Pruning},
            utils::{self, Board, MIRROR},
        },
        solver::{Solver as SolverT, SolverError},
    },
};

/// An optimal solver for 4x4 puzzles in [`Stm`], with pattern databases `P` (one of the sets in
/// [`pdb`](super::pdb), which trade memory for speed).
pub struct Solver<P: Pdb> {
    pdb: P,
    search: Search<128>,
    /// The position being searched, in both orientations.
    boards: [Board; 2],
    /// The current search's depth: the buffers below hold `top + 1` entries (`slots`, `top + 2`).
    top: usize,
    /// `path[depth]`: the blank's cell at each depth of the current path.
    path: Vec<u8>,
    /// `slots[depth][slot]`: successors of the child visited `slot`-th from the node at `depth`,
    /// all generated (and prefetched) before the first child is visited. Uninitialised until
    /// written ([`fill`]), and reached through raw pointers, so a node can keep reading its own
    /// slot while `&mut self` recurses (children only write the slots of lower depths).
    slots: Vec<MaybeUninit<[Expansion<P>; 4]>>,
    /// `ready[depth]`: 1 + the slot in `slots[depth + 1]` holding the successors of the node
    /// about to be visited at `depth`.
    ready: Vec<u8>,
}

/// A position to search from, in both orientations, with its table state.
struct Root<P: Pdb> {
    boards: [Board; 2],
    blank: usize,
    indices: P::Indices,
    values: P::Values,
    bound: u64,
}

impl<P: Pdb> Root<P> {
    /// The root for `internal` (internal labels).
    fn new(pdb: &P, internal: &Board) -> Self {
        let boards = [*internal, utils::reflect(internal)];
        let blank = utils::blank_position(internal);
        let (indices, values, bound) = pdb.root(&boards, blank);
        Self {
            boards,
            blank,
            indices,
            values,
            bound,
        }
    }

    /// A lower bound on the position's solution length, with the solutions' parity (the parity
    /// of the blank's distance from home).
    fn lower_bound(&self) -> u64 {
        self.bound + ((self.bound ^ self.blank as u64) & 1)
    }
}

impl<P> Solver<P>
where
    P: Pdb,
{
    /// Creates a new [`Solver`] that searches with `pdb`.
    #[must_use]
    pub fn new(pdb: P) -> Self {
        Self {
            pdb,
            search: Search::new(),
            boards: [[0; 16]; 2],
            top: 0,
            path: Vec::new(),
            slots: Vec::new(),
            ready: Vec::new(),
        }
    }

    /// The pattern databases.
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
        let root = Root::new(&self.pdb, &utils::to_internal(&board));
        let lower_bound = root.lower_bound();

        // The duplicate-pruning automaton finds an optimal solution but not every solution, so
        // it serves only searches for one solution that start at or below the optimal length.
        let pruning = config.num_solutions == 1 && config.min <= lower_bound;

        iterative_deepening::<Stm, _, _>(
            self,
            |s| &mut s.search,
            config,
            lower_bound,
            |s, depth| {
                if pruning {
                    s.search_depth::<Pruning>(&root, depth)
                } else {
                    s.search_depth::<Backtrack>(&root, depth)
                }
            },
        );

        Ok(())
    }

    /// Search for every solution of exactly `depth` moves from `root` that automaton `A` allows,
    /// recording each; breaks when the search should stop.
    fn search_depth<A>(&mut self, root: &Root<P>, depth: u64) -> ControlFlow<()>
    where
        A: Automaton,
    {
        // Every solution's length is at least the bound and has its parity.
        if depth < root.bound || (depth ^ root.bound) & 1 != 0 {
            return ControlFlow::Continue(());
        }
        let top = usize::try_from(depth).expect("depth fits in memory");
        if self.slots.len() < top + 2 {
            self.slots.resize_with(top + 2, MaybeUninit::uninit);
            self.path.resize(top + 1, 0);
            self.ready.resize(top + 1, 0);
        }
        self.top = top;
        self.boards = root.boards;
        if depth == 0 {
            // A zero bound: the position is solved.
            return self.record(root.blank);
        }
        // The root's successors go where a parent would have put them.
        // SAFETY: top + 1 < slots.len(), and nothing else uses the slot now.
        unsafe {
            fill(self.pre().add(top + 1).cast(), root.values, |out| {
                self.expand::<A>(root.blank, root.indices, A::START, out)
            });
            *self.ready().add(top) = 1;
        }
        self.dfs::<A>(root.blank, depth)
    }

    /// The buffers as raw pointers (see the fields), which writes through them leave valid.
    #[inline(always)]
    fn pre(&mut self) -> *mut [Expansion<P>; 4] {
        self.slots.as_mut_ptr().cast()
    }

    #[inline(always)]
    fn path(&mut self) -> *mut u8 {
        self.path.as_mut_ptr()
    }

    #[inline(always)]
    fn ready(&mut self) -> *mut u8 {
        self.ready.as_mut_ptr()
    }

    /// Put `tiles` at `to` and the blank at `from`, on both boards: a move,
    /// or with the cells swapped its undo.
    #[inline(always)]
    fn slide(&mut self, to: [usize; 2], from: [usize; 2], tiles: [u8; 2]) {
        for side in 0..2 {
            // SAFETY: cells are board positions.
            unsafe {
                *self.boards[side].get_unchecked_mut(to[side]) = tiles[side];
                *self.boards[side].get_unchecked_mut(from[side]) = 0;
            }
        }
    }

    /// The successors written for the node at `depth` (its parent's slot
    /// `ready[depth] - 1`), borrowed apart from `self`: the node keeps
    /// reading them while it recurses, and its children only write the
    /// slots of lower depths.
    #[inline(always)]
    fn successors<'s>(&self, depth: u64) -> &'s Expansion<P> {
        // SAFETY: depth <= top; the parent sits at depth + 1 and wrote slot ready - 1 < 4 before
        // descending here, and nothing writes that slot until this node returns.
        unsafe {
            let ready = *self.ready.as_ptr().add(depth as usize) as usize;
            &*self
                .slots
                .as_ptr()
                .cast::<[Expansion<P>; 4]>()
                .add(depth as usize + 1)
                .cast::<Expansion<P>>()
                .add(ready - 1)
        }
    }

    /// Generate the successors of the child reached by `c` into
    /// `pre[depth][slot]`: apply the move, expand, undo it.
    /// (Inlined: an out-of-line call measured -0.1% against -1.6%.)
    #[inline(always)]
    fn expand_child<A>(
        &mut self,
        here: [usize; 2],
        c: &Candidate<P>,
        values: P::Values,
        depth: u64,
        slot: usize,
    ) where
        A: Automaton,
    {
        let next = c.blanks();
        self.slide(here, next, c.tiles);
        // SAFETY: depth <= top and slot < 4; the slot belongs to this node's children and nobody
        // else uses it now.
        unsafe {
            let pre = self
                .pre()
                .add(depth as usize)
                .cast::<Expansion<P>>()
                .add(slot);
            fill(pre, values, |out| {
                self.expand::<A>(next[0], c.indices, c.node, out)
            });
        }
        self.slide(next, here, c.tiles);
    }

    /// Record the path ending with the blank at `blank` (home) as a solution. Out of line: the
    /// search's hot loop measured 3% slower with it inlined.
    #[cold]
    #[inline(never)]
    fn record(&mut self, blank: usize) -> ControlFlow<()> {
        // `path[d]`: the blank's cell with `d` moves left, so the root's is `path[top]`.
        let cells = &mut self.path[..=self.top];
        cells[0] = blank as u8;
        let stack = &mut self.search.stack;
        stack.clear();
        let trail = cells[..self.top].iter().rev().map(|&c| usize::from(c));
        for d in utils::directions(usize::from(cells[self.top]), trail) {
            stack.push(d);
        }
        self.search.record_solution()
    }

    /// Search the subtree below `blank` for solutions, recording each; breaks when the search
    /// should stop. The node's successors were generated before it was visited.
    fn dfs<A>(&mut self, blank: usize, depth: u64) -> ControlFlow<()>
    where
        A: Automaton,
    {
        if depth == 0 {
            // Every partition covers every tile, so a zero estimate means solved.
            return self.record(blank);
        }
        let here = [blank, MIRROR[blank & 15] as usize];
        let pre = self.successors(depth);
        let mut order = [0usize; 4];
        let mut decoded = [MaybeUninit::<P::Decoded>::uninit(); 4];
        let n = self.survivors(pre, depth, &mut order, &mut decoded);

        // Sibling pipelining: generate every survivor's successors now,
        // best first, so their table lines load while the later siblings
        // are generated (the first survivor's) or during the earlier
        // siblings' subtrees (the rest), instead of on entry. Generating
        // the first one here too measured -3.8% wall on 10k. In failing
        // iterations every survivor is visited, so none of this is
        // wasted. (Children at depth 0 return without expanding, hence
        // the depth test.)
        if depth >= 2 {
            for slot in 0..n {
                // SAFETY: slot < n <= 4, and survivors' entries were
                // written. Read in place, not copied: a copy spills the
                // parent's indices around the move loop (-4% wall with one
                // table base freeing the registers).
                let (c, d) = unsafe {
                    let ck = *order.get_unchecked(slot);
                    (
                        pre.candidates.get_unchecked(ck).assume_init_ref(),
                        decoded.get_unchecked(ck).assume_init_read(),
                    )
                };
                let values = P::child(pre.values, d, c.tiles);
                self.expand_child::<A>(here, c, values, depth, slot);
            }
        }

        for slot in 0..n {
            // Survivor `slot` finds its successors in slot `slot`.
            // SAFETY: 1 <= depth <= top, slot < n <= 4, and survivors' entries were written.
            let c = unsafe {
                *self.ready().add(depth as usize - 1) = slot as u8 + 1;
                *self.path().add(depth as usize) = blank as u8;
                let ck = *order.get_unchecked(slot);
                pre.candidates.get_unchecked(ck).assume_init_read()
            };
            let next = c.blanks();
            self.slide(here, next, c.tiles);
            let flow = self.dfs::<A>(next[0], depth - 1);
            self.slide(next, here, c.tiles);
            flow?;
        }
        ControlFlow::Continue(())
    }

    /// The candidates of `pre` within `depth`, best first: writes their
    /// indices to `order` and their decoded estimates (by candidate) to
    /// `decoded`, and returns how many there are. The set decodes each
    /// candidate's entries against the node's values, short-circuiting in
    /// prune order, and keys survivors as it goes, so ordering costs only the
    /// sort, no extra table loads.
    #[inline(always)]
    fn survivors(
        &self,
        pre: &Expansion<P>,
        depth: u64,
        order: &mut [usize; 4],
        decoded: &mut [MaybeUninit<P::Decoded>; 4],
    ) -> usize {
        let decoder = P::decoder(pre.values);
        let mut keys = [0u64; 4];
        let mut n = 0;
        for k in 0..pre.count {
            // SAFETY: entries below count (at most 4) were all written first.
            let c = unsafe { pre.candidates.get_unchecked(k).assume_init_read() };
            if let Some((key, d)) = self
                .pdb
                .evaluate(&c.indices, c.blanks, c.tiles, &decoder, depth)
            {
                // SAFETY: n <= k < count <= 4.
                unsafe {
                    *keys.get_unchecked_mut(k) = key;
                    decoded.get_unchecked_mut(k).write(d);
                    *order.get_unchecked_mut(n) = k;
                }
                n += 1;
            }
        }
        // Insertion sort by key; at most four entries, so the quadratic loop is
        // a handful of predictable branches. (Indices are below 4; the masks
        // only drop the bounds checks.)
        for k in 1..n {
            let ck = order[k & 3];
            let mut j = k;
            while j > 0 && keys[order[(j - 1) & 3] & 3] > keys[ck & 3] {
                order[j & 3] = order[(j - 1) & 3];
                j -= 1;
            }
            order[j & 3] = ck;
        }
        n
    }

    /// Generate the successors of the node with the blank at `blank`: every
    /// move the automaton `node` allows, with its table state shifted from
    /// `base`, written to `out` in generation order with its table lines
    /// prefetched. Returns how many were written.
    #[inline(always)]
    fn expand<A>(
        &self,
        blank: usize,
        base: P::Indices,
        node: u32,
        out: &mut [MaybeUninit<Candidate<P>>; 4],
    ) -> usize
    where
        A: Automaton,
    {
        let boards = &self.boards;
        // The usable moves are looked up in a table: the edge test, the
        // directions the automaton allows (which exclude the undo move) and
        // the geometry the scan needs are all precomputed per (blank cell,
        // allowed mask).
        // SAFETY: blank is a board position and the mask is 0..16.
        let list = unsafe {
            utils::MOVES
                .get_unchecked(blank)
                .get_unchecked((node & 15) as usize)
        };
        for k in 0..list.n as usize {
            // SAFETY: k < n <= 4; the move table lists existing neighbours only.
            let (m, tiles) = unsafe {
                let m = list.moves.get_unchecked(k);
                let tiles = [
                    *boards[0].get_unchecked(m.next[0] as usize),
                    *boards[1].get_unchecked(m.next[1] as usize),
                ];
                (m, tiles)
            };
            let node = A::next(node, m.dir);
            let candidate = Candidate {
                indices: P::shifted(base, boards, m, tiles),
                blanks: m.next,
                tiles,
                node,
            };
            self.pdb.prefetch(&candidate.indices, m.next);
            // SAFETY: k < n <= 4.
            unsafe { out.get_unchecked_mut(k) }.write(candidate);
        }
        list.n as usize
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

/// Write a node's `values` and successors (`expand` fills the candidates
/// and returns how many) into `slot`.
///
/// # Safety
///
/// `slot` must be valid for writes and not in use: it may hold anything
/// (or nothing), so it is written field by field through raw pointers, and
/// it is readable as an [`Expansion`] afterwards.
#[inline(always)]
unsafe fn fill<P>(
    slot: *mut Expansion<P>,
    values: P::Values,
    expand: impl FnOnce(&mut [MaybeUninit<Candidate<P>>; 4]) -> usize,
) where
    P: Pdb,
{
    // SAFETY: the caller's contract; a reference to `MaybeUninit`
    // candidates is valid whatever they hold.
    unsafe {
        (&raw mut (*slot).values).write(values);
        let count = expand(&mut (*slot).candidates);
        (&raw mut (*slot).count).write(count);
    }
}

/// One successor move: the set's state after it, the move, and the
/// duplicate-pruning node after it ([`fsm`]).
struct Candidate<P: Pdb> {
    indices: P::Indices,
    blanks: [u8; 2],
    tiles: [u8; 2],
    node: u32,
}

impl<P: Pdb> Candidate<P> {
    fn blanks(&self) -> [usize; 2] {
        [self.blanks[0] as usize, self.blanks[1] as usize]
    }
}

/// A node's successors, generated before the node is visited.
struct Expansion<P: Pdb> {
    candidates: [MaybeUninit<Candidate<P>>; 4],
    count: usize,
    /// The expanded node's exact values, which its candidates decode
    /// against.
    values: P::Values,
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
        LazyLock::new(|| std::sync::Mutex::new(Solver::new(Tiny::build())));

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
