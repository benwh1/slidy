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

use std::mem::MaybeUninit;

use crate::{
    puzzle::{sliding_puzzle::SlidingPuzzle as _, small::Puzzle4x4},
    solver::size4x4::stm::{
        fsm,
        pdb::{Dbs, Layout},
        tables::{self, Board, MIRROR, Move},
    },
};

const MAX_PATH: usize = 85;

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

/// Solve one position optimally with table set `H`, whose tables `dbs`
/// holds.
#[must_use]
pub fn solve<H>(puzzle: &Board, dbs: Dbs<'_>) -> Option<String>
where
    H: TableSet,
{
    let start_state = Puzzle4x4::try_from(*puzzle).unwrap();
    if start_state.is_solved() {
        return Some(String::new());
    }
    if !start_state.is_solvable() {
        return None;
    }
    let internal = tables::to_internal(puzzle);
    let boards = [internal, tables::reflect(&internal)];
    let blank = tables::blank_position(&internal);
    let (root, values, distance) = H::root(dbs, &boards, blank);

    // Every slot is written before it is read, so they start uninitialised:
    // no allocation and no fill per solve.
    let mut slots = [const { MaybeUninit::<[Expansion<H>; 4]>::uninit() }; MAX_PATH];
    let mut search = Search {
        dbs,
        boards,
        path: [0; MAX_PATH],
        pre: slots.as_mut_ptr().cast::<[Expansion<H>; 4]>(),
        ready: [0; MAX_PATH],
    };
    // Thresholds keep the parity of the blank's distance from home.
    let mut threshold = 2 - (blank % 2) as u64;
    if distance > threshold {
        threshold = distance + (threshold.wrapping_sub(distance) & 1);
    }
    loop {
        // The root's successors go where a parent would have put them.
        let top = threshold as usize;
        // SAFETY: the threshold stays far below MAX_PATH - 1, and nothing
        // else uses the slot now.
        unsafe {
            fill(search.pre.add(top + 1).cast(), values, |out| {
                expand::<H>(dbs, &search.boards, blank, root, fsm::START, out)
            });
        }
        search.ready[top] = 1;
        if search.dfs(blank, threshold) {
            break;
        }
        threshold += 2;
    }

    // The blank's cells after each move, first move first.
    let trail = search.path[..threshold as usize].iter().rev();
    Some(tables::solution_string(
        blank,
        trail.map(|&cell| cell as usize),
    ))
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

struct Search<'a, H: TableSet> {
    dbs: Dbs<'a>,
    boards: [Board; 2],
    /// The blank's cell at each depth of the solution found.
    path: [u8; MAX_PATH],
    /// `pre[depth][slot]`: successors of the child visited `slot`-th from
    /// the node at `depth`, all generated (and prefetched) before the
    /// first child is visited. On [`solve`]'s stack, uninitialised until
    /// written ([`fill`]), and reached through a raw pointer, so a node
    /// can keep reading its own slot while `&mut self` recurses (children
    /// only write the slots of lower depths).
    pre: *mut [Expansion<H>; 4],
    /// `ready[depth]`: 1 + the slot in `pre[depth + 1]` holding the
    /// successors of the node about to be visited at `depth`.
    ready: [u8; MAX_PATH],
}

impl<H: TableSet> Search<'_, H> {
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
        // SAFETY: depth < threshold < MAX_PATH; the parent sits at depth + 1
        // and wrote slot ready - 1 < 4 before descending here, and nothing
        // writes that slot until this node returns.
        unsafe {
            let ready = *self.ready.get_unchecked(depth as usize) as usize;
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
        // SAFETY: depth <= threshold < MAX_PATH and slot < 4; the slot
        // belongs to this node's children and nobody else uses it now.
        unsafe {
            let pre = self
                .pre
                .add(depth as usize)
                .cast::<Expansion<H>>()
                .add(slot);
            fill(pre, values, |out| {
                expand::<H>(self.dbs, &self.boards, next[0], c.indices, c.node, out)
            });
        }
        self.slide(next, here, c.tiles);
    }

    /// Search the subtree below `blank`; true when a solution was found.
    /// The node's successors were generated before it was visited.
    fn dfs(&mut self, blank: usize, depth: u64) -> bool {
        if depth == 0 {
            self.path[0] = blank as u8;
            return true;
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
            // SAFETY: depth < MAX_PATH, slot < n <= 4, and survivors'
            // entries were written.
            let c = unsafe {
                *self.ready.get_unchecked_mut(depth as usize - 1) = slot as u8 + 1;
                *self.path.get_unchecked_mut(depth as usize) = blank as u8;
                let ck = *order.get_unchecked(slot);
                pre.candidates.get_unchecked(ck).assume_init_read()
            };
            let next = c.blanks();
            self.slide(here, next, c.tiles);
            let found = self.dfs(next[0], depth - 1);
            self.slide(next, here, c.tiles);
            if found {
                return true;
            }
        }
        false
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
fn expand<H>(
    dbs: Dbs<'_>,
    boards: &[Board; 2],
    blank: usize,
    base: H::Indices,
    node: u32,
    out: &mut [MaybeUninit<Candidate<H>>; 4],
) -> usize
where
    H: TableSet,
{
    // The usable moves are looked up in a table: the edge test, the
    // directions the automaton allows (which exclude the undo move) and
    // the geometry the scan needs are all precomputed per (blank cell,
    // allowed mask).
    let state = (node >> 4) as usize;
    // SAFETY: blank is a board position and the mask is 0..16.
    let list = unsafe {
        tables::MOVES
            .get_unchecked(blank)
            .get_unchecked((node & 15) as usize)
    };
    for k in 0..list.n as usize {
        // SAFETY: k < n <= 4; a live automaton state and a direction 0..4
        // index the automaton (16-bit states: half the footprint of packed
        // nodes, -12% L2 misses), whose state's group lies inside the mask
        // table; the move table lists existing neighbours only.
        let (m, node, tiles) = unsafe {
            let m = list.moves.get_unchecked(k);
            let next = u32::from(*fsm::FSM.get_unchecked(state).get_unchecked(m.dir as usize));
            let mask = u32::from(*fsm::MASK_OF.get_unchecked((next >> 6) as usize));
            let tiles = [
                *boards[0].get_unchecked(m.next[0] as usize),
                *boards[1].get_unchecked(m.next[1] as usize),
            ];
            (m, (next << 4) | mask, tiles)
        };
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
            let mut node = fsm::START;
            for &dir in dirs {
                if node & (1 << dir) == 0 {
                    return None;
                }
                let state = u32::from(fsm::FSM[(node >> 4) as usize][dir as usize]);
                node = (state << 4) | u32::from(fsm::MASK_OF[(state >> 6) as usize]);
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
