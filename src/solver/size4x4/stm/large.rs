//! Optimal IDA* search over a [`TableSet`].
//!
//! The search is table-agnostic: it keeps the board in both orientations
//! (`[direct, mirrored]`, mirrored across the main diagonal), generates
//! successors through the move table and the duplicate-pruning automaton
//! (`fsm`), and asks the table set to shift its indices, prefetch its
//! entries, and decode, prune and rank each successor. A set's values are
//! carried down the tree, so a set whose tables hold only codes (bit
//! tables) can decode a child's entries against its parent's values. The
//! search is monomorphised per set and every set method is inlined into
//! it, so a set is chosen at compile time at no cost.

use std::{marker::PhantomData, mem::MaybeUninit, ops::ControlFlow};

use crate::{
    puzzle::{sliding_puzzle::SlidingPuzzle as _, small::Puzzle4x4},
    solver::size4x4::stm::{
        fsm,
        pdb::region::{Dbs, Layout},
        utils::{self, Board, MIRROR, Move},
    },
};

/// A combination of pattern databases: everything the search needs to
/// know about its tables. Implemented by zero-sized marker types, chosen
/// as the search's type parameter; every method is inlined into the
/// search. Pairs are `[direct, mirrored]`.
///
/// The one implementation is [`Gen`](super::generic::Gen), for any partitions and table
/// formats: its [`Layout`], its index state (Lehmer indices per group and orientation, shifted
/// incrementally), the values it carries (exact values for its coded tables, found at the root
/// by walking downhill), and the prune test with its best-first key. Keep the methods' data in
/// registers: a value stored at a computed index and read back wider fails store forwarding, and
/// references to locals can keep them in memory.
pub trait TableSet: 'static {
    /// Where the set's tables sit in its combined file.
    const LAYOUT: Layout;
    /// A position's table state in both orientations: its indices, and
    /// anything else a move updates incrementally.
    type Indices: Copy;
    /// A node's exact table values (both orientations), carried down the
    /// tree for its children to decode against.
    type Values: Copy;
    /// A node's values set up for decoding its children's entries.
    type Decoder;
    /// A surviving child's decoded estimates, from which [`Self::child`]
    /// makes its values.
    type Decoded: Copy;

    /// The root's table state, its exact values and its lower bound, with
    /// the blank at `blank` on the direct board (the position is not
    /// solved).
    fn root(dbs: Dbs<'_>, boards: &[Board; 2], blank: usize) -> (Self::Indices, Self::Values, u64);

    /// The table state after move `m` on `boards`, which moves `tiles`
    /// into the blank.
    fn shifted(base: Self::Indices, boards: &[Board; 2], m: &Move, tiles: [u8; 2])
    -> Self::Indices;

    /// Prefetch the table lines [`Self::evaluate`] will read for a child
    /// with state `c` and the blank at `blanks`.
    fn prefetch(dbs: Dbs<'_>, c: &Self::Indices, blanks: [u8; 2]);

    /// A node's values set up for [`Self::evaluate`].
    fn decoder(values: Self::Values) -> Self::Decoder;

    /// A child with state `c`, the blank at `blanks` and the moved tile
    /// `tiles`: `None` when its estimate reaches `depth` (the moves left at
    /// its parent), else its best-first sort key (smaller first) and its
    /// decoded estimates.
    fn evaluate(
        dbs: Dbs<'_>,
        c: &Self::Indices,
        blanks: [u8; 2],
        tiles: [u8; 2],
        decoder: &Self::Decoder,
        depth: u64,
    ) -> Option<(u64, Self::Decoded)>;

    /// A surviving child's values from its parent's `values`, its decoded
    /// estimates and the moved tiles.
    fn child(values: Self::Values, decoded: Self::Decoded, tiles: [u8; 2]) -> Self::Values;
}

/// Which move sequences the search may take, as an automaton over moves. A node is
/// `(state << 4) | allowed`, `allowed` the mask of directions (slidy's: up 0, left 1, down 2,
/// right 3, a tile's move) the next move may take.
pub trait Automaton: 'static {
    /// The root's node.
    const START: u32;

    /// The node after a move in direction `dir` from `node`.
    fn next(node: u32, dir: u8) -> u32;
}

/// Forbids undoing the previous move and one of each pair of equivalent move sequences of equal
/// length ([`fsm`]): still reaches every position by a shortest path, so it finds an optimal
/// solution, but not every one.
pub struct Pruning;

impl Automaton for Pruning {
    const START: u32 = fsm::START;

    #[inline(always)]
    fn next(node: u32, dir: u8) -> u32 {
        // SAFETY: a live automaton state and a direction 0..4 index the automaton (16-bit states:
        // half the footprint of packed nodes, -12% L2 misses), whose state's group lies inside
        // the mask table.
        unsafe {
            let next = u32::from(
                *fsm::FSM
                    .get_unchecked((node >> 4) as usize)
                    .get_unchecked(dir as usize),
            );
            let mask = u32::from(*fsm::MASK_OF.get_unchecked((next >> 6) as usize));
            (next << 4) | mask
        }
    }
}

/// Forbids only undoing the previous move, so it finds every solution of a length.
pub struct Backtrack;

impl Automaton for Backtrack {
    const START: u32 = 15;

    #[inline(always)]
    fn next(_node: u32, dir: u8) -> u32 {
        15 & !(1 << ((dir + 2) & 3))
    }
}

/// A position to search from, in both orientations, with its table state.
pub struct Root<H: TableSet> {
    boards: [Board; 2],
    blank: usize,
    indices: H::Indices,
    values: H::Values,
    bound: u64,
}

impl<H: TableSet> Root<H> {
    /// The root for `internal` (internal labels), whose tables `dbs` holds.
    #[must_use]
    pub fn new(internal: &Board, dbs: Dbs<'_>) -> Self {
        let boards = [*internal, utils::reflect(internal)];
        let blank = utils::blank_position(internal);
        let (indices, values, bound) = H::root(dbs, &boards, blank);
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
    #[must_use]
    pub fn lower_bound(&self) -> u64 {
        self.bound + ((self.bound ^ self.blank as u64) & 1)
    }

    /// The blank's internal cell.
    #[must_use]
    pub fn blank(&self) -> usize {
        self.blank
    }
}

/// The search's buffers, kept between searches.
pub struct Engine<H: TableSet> {
    /// `slots[depth]`: see [`Search::pre`].
    slots: Vec<MaybeUninit<[Expansion<H>; 4]>>,
    path: Vec<u8>,
    ready: Vec<u8>,
}

impl<H: TableSet> Default for Engine<H> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            path: Vec::new(),
            ready: Vec::new(),
        }
    }
}

impl<H: TableSet> Engine<H> {
    /// Search for every solution of exactly `depth` moves from `root` that automaton `A` allows,
    /// calling `found` with each: `cells[d]` is the blank's internal cell with `d` moves left
    /// (`cells[depth]` the root's, `cells[0]` home). Stops when `found` breaks.
    pub fn search<A>(
        &mut self,
        dbs: Dbs<'_>,
        root: &Root<H>,
        depth: u64,
        found: &mut dyn FnMut(&[u8]) -> ControlFlow<()>,
    ) -> ControlFlow<()>
    where
        A: Automaton,
    {
        // Every solution's length is at least the bound and has its parity.
        if depth < root.bound || (depth ^ root.bound) & 1 != 0 {
            return ControlFlow::Continue(());
        }
        if depth == 0 {
            // A zero bound: the position is solved.
            return found(&[root.blank as u8]);
        }
        let top = usize::try_from(depth).expect("depth fits in memory");
        if self.slots.len() < top + 2 {
            self.slots.resize_with(top + 2, MaybeUninit::uninit);
            self.path.resize(top + 1, 0);
            self.ready.resize(top + 1, 0);
        }
        let mut search = Search::<H, A> {
            dbs,
            boards: root.boards,
            path: self.path.as_mut_ptr(),
            pre: self.slots.as_mut_ptr().cast::<[Expansion<H>; 4]>(),
            ready: self.ready.as_mut_ptr(),
            top,
            found,
            automaton: PhantomData,
        };
        // The root's successors go where a parent would have put them.
        // SAFETY: top + 1 < slots.len(), and nothing else uses the slot now.
        unsafe {
            fill(search.pre.add(top + 1).cast(), root.values, |out| {
                expand::<H, A>(dbs, &search.boards, root.blank, root.indices, A::START, out)
            });
            *search.ready.add(top) = 1;
        }
        search.dfs(root.blank, depth)
    }
}

/// Solve one position (`puzzle`, slidy's labels) optimally with table set `H`, whose tables
/// `dbs` holds: the blank's moves as `ULDR`, or `None` when it is unsolvable.
#[must_use]
pub fn solve<H>(puzzle: &Board, dbs: Dbs<'_>) -> Option<String>
where
    H: TableSet,
{
    if !Puzzle4x4::try_from(*puzzle).ok()?.is_solvable() {
        return None;
    }
    let root = Root::<H>::new(&utils::to_internal(puzzle), dbs);
    let mut engine = Engine::default();
    let mut solution = None;
    let mut depth = root.lower_bound();
    while solution.is_none() {
        let _ = engine.search::<Pruning>(dbs, &root, depth, &mut |cells| {
            // The blank's cells after each move, first move first.
            let trail = cells[..cells.len() - 1].iter().rev();
            solution = Some(utils::solution_string(
                root.blank,
                trail.map(|&c| usize::from(c)),
            ));
            ControlFlow::Break(())
        });
        depth += 2;
    }
    solution
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
unsafe fn fill<H>(
    slot: *mut Expansion<H>,
    values: H::Values,
    expand: impl FnOnce(&mut [MaybeUninit<Candidate<H>>; 4]) -> usize,
) where
    H: TableSet,
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
struct Candidate<H: TableSet> {
    indices: H::Indices,
    blanks: [u8; 2],
    tiles: [u8; 2],
    node: u32,
}

impl<H: TableSet> Candidate<H> {
    fn blanks(&self) -> [usize; 2] {
        [self.blanks[0] as usize, self.blanks[1] as usize]
    }
}

/// A node's successors, generated before the node is visited.
struct Expansion<H: TableSet> {
    candidates: [MaybeUninit<Candidate<H>>; 4],
    count: usize,
    /// The expanded node's exact values, which its candidates decode
    /// against.
    values: H::Values,
}

struct Search<'a, 'f, H: TableSet, A> {
    dbs: Dbs<'a>,
    boards: [Board; 2],
    /// `path[depth]`: the blank's cell at each depth of the current path.
    path: *mut u8,
    /// `pre[depth][slot]`: successors of the child visited `slot`-th from the node at `depth`,
    /// all generated (and prefetched) before the first child is visited. In the [`Engine`]'s
    /// buffer, uninitialised until written ([`fill`]), and reached through a raw pointer, so a
    /// node can keep reading its own slot while `&mut self` recurses (children only write the
    /// slots of lower depths).
    pre: *mut [Expansion<H>; 4],
    /// `ready[depth]`: 1 + the slot in `pre[depth + 1]` holding the successors of the node about
    /// to be visited at `depth`.
    ready: *mut u8,
    /// The root's depth: the buffers hold `top + 1` entries (`pre`, `top + 2`).
    top: usize,
    found: &'f mut dyn FnMut(&[u8]) -> ControlFlow<()>,
    automaton: PhantomData<A>,
}

impl<H: TableSet, A: Automaton> Search<'_, '_, H, A> {
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
    fn successors<'s>(&self, depth: u64) -> &'s Expansion<H> {
        // SAFETY: depth <= top; the parent sits at depth + 1 and wrote slot ready - 1 < 4 before
        // descending here, and nothing writes that slot until this node returns.
        unsafe {
            let ready = *self.ready.add(depth as usize) as usize;
            &*self
                .pre
                .add(depth as usize + 1)
                .cast::<Expansion<H>>()
                .add(ready - 1)
        }
    }

    /// Generate the successors of the child reached by `c` into
    /// `pre[depth][slot]`: apply the move, expand, undo it.
    /// (Inlined: an out-of-line call measured -0.1% against -1.6%.)
    #[inline(always)]
    fn expand_child(
        &mut self,
        here: [usize; 2],
        c: &Candidate<H>,
        values: H::Values,
        depth: u64,
        slot: usize,
    ) {
        let next = c.blanks();
        self.slide(here, next, c.tiles);
        // SAFETY: depth <= top and slot < 4; the slot belongs to this node's children and nobody
        // else uses it now.
        unsafe {
            let pre = self
                .pre
                .add(depth as usize)
                .cast::<Expansion<H>>()
                .add(slot);
            fill(pre, values, |out| {
                expand::<H, A>(self.dbs, &self.boards, next[0], c.indices, c.node, out)
            });
        }
        self.slide(next, here, c.tiles);
    }

    /// Pass the path ending with the blank at `blank` (home) to `found`.
    #[cold]
    #[inline(never)]
    fn record(&mut self, blank: usize) -> ControlFlow<()> {
        // SAFETY: path holds top + 1 entries, all written on the way down.
        unsafe {
            *self.path = blank as u8;
            (self.found)(std::slice::from_raw_parts(self.path, self.top + 1))
        }
    }

    /// Search the subtree below `blank` for solutions, passing each to `found`; breaks when it
    /// does. The node's successors were generated before it was visited.
    fn dfs(&mut self, blank: usize, depth: u64) -> ControlFlow<()> {
        if depth == 0 {
            // Every partition covers every tile, so a zero estimate means solved.
            return self.record(blank);
        }
        let here = [blank, MIRROR[blank & 15] as usize];
        let pre = self.successors(depth);
        let mut order = [0usize; 4];
        let mut decoded = [MaybeUninit::<H::Decoded>::uninit(); 4];
        let n = survivors::<H>(self.dbs, pre, depth, &mut order, &mut decoded);

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
                let values = H::child(pre.values, d, c.tiles);
                self.expand_child(here, c, values, depth, slot);
            }
        }

        for slot in 0..n {
            // Survivor `slot` finds its successors in slot `slot`.
            // SAFETY: 1 <= depth <= top, slot < n <= 4, and survivors' entries were written.
            let c = unsafe {
                *self.ready.add(depth as usize - 1) = slot as u8 + 1;
                *self.path.add(depth as usize) = blank as u8;
                let ck = *order.get_unchecked(slot);
                pre.candidates.get_unchecked(ck).assume_init_read()
            };
            let next = c.blanks();
            self.slide(here, next, c.tiles);
            let flow = self.dfs(next[0], depth - 1);
            self.slide(next, here, c.tiles);
            flow?;
        }
        ControlFlow::Continue(())
    }
}

/// The candidates of `pre` within `depth`, best first: writes their
/// indices to `order` and their decoded estimates (by candidate) to
/// `decoded`, and returns how many there are. The set decodes each
/// candidate's entries against the node's values, short-circuiting in
/// prune order, and keys survivors as it goes, so ordering costs only the
/// sort, no extra table loads.
#[inline(always)]
fn survivors<H>(
    dbs: Dbs<'_>,
    pre: &Expansion<H>,
    depth: u64,
    order: &mut [usize; 4],
    decoded: &mut [MaybeUninit<H::Decoded>; 4],
) -> usize
where
    H: TableSet,
{
    let decoder = H::decoder(pre.values);
    let mut keys = [0u64; 4];
    let mut n = 0;
    for k in 0..pre.count {
        // SAFETY: entries below count (at most 4) were all written first.
        let c = unsafe { pre.candidates.get_unchecked(k).assume_init_read() };
        if let Some((key, d)) = H::evaluate(dbs, &c.indices, c.blanks, c.tiles, &decoder, depth) {
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
fn expand<H, A>(
    dbs: Dbs<'_>,
    boards: &[Board; 2],
    blank: usize,
    base: H::Indices,
    node: u32,
    out: &mut [MaybeUninit<Candidate<H>>; 4],
) -> usize
where
    H: TableSet,
    A: Automaton,
{
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
            indices: H::shifted(base, boards, m, tiles),
            blanks: m.next,
            tiles,
            node,
        };
        H::prefetch(dbs, &candidate.indices, m.next);
        // SAFETY: k < n <= 4.
        unsafe { out.get_unchecked_mut(k) }.write(candidate);
    }
    list.n as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The automaton forbids undoing the previous move, and of the
    /// shortest duplicate pair - 1.5 laps around a 2x2 block one way or
    /// the other, which reach the same position - it allows exactly one.
    #[test]
    fn automaton_prunes_backtracks_and_the_six_move_loop() {
        // Slidy directions: up 0, left 1, down 2, right 3.
        let walk = |dirs: &[u8]| {
            let mut node = Pruning::START;
            for &dir in dirs {
                if node & (1 << dir) == 0 {
                    return None;
                }
                node = Pruning::next(node, dir);
            }
            Some(node)
        };
        for dir in 0..4 {
            assert!(walk(&[dir]).is_some());
            assert!(walk(&[dir, (dir + 2) % 4]).is_none());
        }
        let down_left = walk(&[2, 1, 0, 3, 2, 1]);
        let left_down = walk(&[1, 2, 3, 0, 1, 2]);
        assert!(down_left.is_some());
        assert!(left_down.is_none());
    }
}
