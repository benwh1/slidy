//! Exact values of a position's coded pattern-database entries.
//!
//! A coded entry holds `(value >> 1) & 1` or `(value >> 1) & 3` of its state's distance, and a
//! state's colour gives the distance's parity, so the entry gives the distance mod 4 or mod 8.
//! The search decodes its children against their parent's exact values, so at the root each
//! coded table is walked downhill to its goal: some move always leads to a lower distance, and
//! the codes tell which. The walk's length is the root's value.

use crate::solver::size4x4::stm::utils::{
    Board, NBR,
    build::flood,
    index::{lehmer, mask, placement},
    prefetch,
};

/// What a coded table holds, and so how a walk moves.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// `(value >> 1) & 1` of the additive distance: a group tile moves into the blank's region.
    Additive,
    /// `(value >> 1) & 1` of the full distance: the blank moves.
    Full,
    /// `(value >> 1) & 3` of the placement's relaxed minimum additive distance (which a move
    /// changes by 1 or 3): a group tile moves to any neighbouring cell without one.
    Min2,
}

/// A state of a walk: the placement's index, the blank's cell and the cells without a group tile.
#[derive(Clone, Copy)]
pub struct Node {
    pub index: u32,
    pub blank: u32,
    pub free: u16,
}

/// The value at `board` of the coded table of `tiles` (which `code` describes), whose entries
/// `entry` reads (and `address` locates, for prefetching). `table` is a [`shift_table`] that
/// holds the group in partition `lane`.
///
/// # Panics
///
/// Panics if the table's codes lead nowhere, which only wrong tables do.
///
/// [`shift_table`]: super::index::shift_table
pub fn walk(
    tiles: &[u8],
    table: &[[u64; 16]; 16],
    lane: usize,
    board: &Board,
    code: Code,
    entry: impl Fn(Node) -> u32,
    address: impl Fn(Node) -> *const u8,
) -> u8 {
    let group = mask(tiles);
    let mut tiles_on: Board = board.map(|t| if (group >> t) & 1 != 0 { t } else { 0 });
    let mut node = Node {
        index: placement(tiles, board),
        blank: board.iter().position(|&t| t == 0).unwrap() as u32,
        free: (0..16).filter(|&c| tiles_on[c] == 0).fold(0, |m, c| m | (1 << c)),
    };
    let goal = lehmer(tiles);
    // The value's parity: in the serpentine order a cell's colour is its parity, and an additive
    // move moves a group tile, a full one the blank.
    let parity = if code == Code::Full {
        node.blank & 1
    } else {
        tiles.iter().fold(0, |p, &t| {
            p ^ ((board.iter().position(|&c| c == t).unwrap() as u32 ^ u32::from(t)) & 1)
        })
    };
    let modulus = if code == Code::Min2 { 8 } else { 4 };
    let mut residue = parity | (entry(node) << 1);
    let mut value = 0u32;
    while node.index != goal || (code == Code::Full && node.blank != 0) {
        // The moves: (from, to), the tile on `from` moving to `to`.
        let mut moves = [(0u32, 0u32); 48];
        let mut n = 0;
        let mut push = |from: u32, to: u32| {
            moves[n] = (from, to);
            n += 1;
        };
        match code {
            Code::Full => {
                let mut nbrs = NBR[node.blank as usize];
                while nbrs != 0 {
                    push(nbrs.trailing_zeros(), node.blank);
                    nbrs &= nbrs - 1;
                }
            }
            Code::Additive | Code::Min2 => {
                let into = if code == Code::Additive {
                    flood(1 << node.blank, node.free)
                } else {
                    node.free
                };
                let mut cells = !node.free;
                while cells != 0 {
                    let from = cells.trailing_zeros();
                    cells &= cells - 1;
                    let mut to = NBR[from as usize] & into;
                    while to != 0 {
                        push(from, to.trailing_zeros());
                        to &= to - 1;
                    }
                }
            }
        }
        let next = |(from, to): (u32, u32)| {
            let tile = tiles_on[from as usize];
            if tile == 0 {
                // The blank moves to a cell without a group tile.
                return Node { blank: from, ..node };
            }
            let row = &table[usize::from(tile)];
            let lane = |x: u64| (x >> (32 * lane)) as u32;
            let (lo, hi) = (from.min(to), from.max(to));
            let delta = (lo + 1..hi).fold(lane(row[0]), |sum, c| {
                sum + lane(row[usize::from(tiles_on[c as usize])])
            });
            Node {
                index: if to > from {
                    node.index + delta
                } else {
                    node.index - delta
                },
                blank: from,
                free: node.free ^ (1 << from) ^ (1 << to),
            }
        };
        for &m in &moves[..n] {
            prefetch(address(next(m)));
        }
        // A neighbour's value is 1 or 3 away, and those values have different codes; the lower
        // ones are 1 and (for `Min2`) 3 less.
        let lower = |by: u32| ((residue + modulus - by) % modulus) >> 1;
        let found = moves[..n].iter().find_map(|&m| {
            let child = next(m);
            let e = entry(child);
            [1, 3]
                .into_iter()
                .take(if code == Code::Min2 { 2 } else { 1 })
                .find(|&by| lower(by) == e)
                .map(|by| (m, child, by))
        });
        let ((from, to), child, by) = found.expect("the table's codes lead downhill");
        tiles_on[to as usize] = tiles_on[from as usize];
        tiles_on[from as usize] = 0;
        node = child;
        residue = (residue + modulus - by) % modulus;
        value += by;
        assert!(value < 256, "the table's codes lead downhill");
    }
    value as u8
}
