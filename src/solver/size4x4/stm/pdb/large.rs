//! The large pattern database (1.27 GB): a single 7-8 partition.
//!
//! Both tables hold, for each placement and blank cell, `(value >> 1) & 1` of the group's
//! additive distance and that of its full distance (counting every move), which bounds the
//! solution length on its own. The 7-tile table holds a `u32` per placement: bit `2b` is the
//! additive bit with the blank on cell `b`, bit `2b + 1` the full one. The 8-tile table holds a
//! `u16` per placement, indexed by the blank's rank `r` among the 8 cells without a group tile:
//! bit `r` is the additive bit, bit `8 + r` the full one. Builds in about ten minutes.

use std::{hint::select_unpredictable, sync::atomic::Ordering::Relaxed};

use xxhash_rust::xxh3;

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::{
        pdb::Pdb,
        utils::{
            Board, Move,
            build::{Cost, atomic, bfs},
            index::{mask, placement, placements, shift, shift_rank, shift_table},
            prefetch,
            walk::{Code, walk},
        },
    },
};

/// The top two rows but the blank's home.
const T: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
/// The bottom two rows.
const B: &[u8] = &[8, 9, 10, 11, 12, 13, 14, 15];

/// How moves shift the groups' indices.
const SHIFT: [[u64; 16]; 16] = shift_table([&[T, B], &[]]);

// Where each table starts.
const T_AT: usize = 0;
const B_AT: usize = T_AT + placements(7) * 4;

/// The length of the database in bytes.
const LEN: usize = B_AT + placements(8) * 2;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
///
/// Not yet computed (it needs a full build), so `try_from_bytes` rejects every input for now;
/// `from_bytes_unchecked` works.
// TODO: set from `test_pdb`'s output once the database has been built.
const HASH: u64 = 0;

/// The large pattern database. See the [module documentation](self).
pub struct Large {
    bytes: Box<[u8]>,
}

/// `((value + 1) >> 1) & 1`: the bit a table holds for `value + 1`. A move changes a value by
/// exactly 1, so a child's bit matches this exactly when its value went up.
#[inline(always)]
const fn up(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

/// The blank's rank among the cells of `board` without a tile of `group`.
fn rank(board: &Board, group: u16) -> u32 {
    let blank = board.iter().position(|&t| t == 0).unwrap();
    board[..blank].iter().filter(|&&t| (group >> t) & 1 == 0).count() as u32
}

impl Large {
    #[inline(always)]
    fn address(&self, i: usize) -> *const u8 {
        self.bytes.as_ptr().wrapping_add(i)
    }

    /// The 7-tile table's bits for placement `index`, from blank cell `blank`'s on.
    #[inline(always)]
    fn seven(&self, index: u32, blank: usize) -> u32 {
        let i = T_AT + 4 * index as usize;
        debug_assert!(i + 4 <= LEN);
        // SAFETY: every index is within its table, and the database has length `LEN`.
        let word = unsafe { self.bytes.as_ptr().add(i).cast::<u32>().read_unaligned() };
        word >> (2 * blank)
    }

    /// The 8-tile table's bits for placement `index`, from blank rank `rank`'s on.
    #[inline(always)]
    fn eight(&self, index: u32, rank: u32) -> u32 {
        let i = B_AT + 2 * index as usize;
        debug_assert!(i + 2 <= LEN);
        // SAFETY: as in `seven`.
        let half = unsafe { self.bytes.as_ptr().add(i).cast::<u16>().read_unaligned() };
        u32::from(half) >> rank
    }
}

/// A child's additive and full values from `bits` (its additive bit, and its full one `at`
/// bits higher), decoded against its parent's; the additive value only changes when `moved`.
#[inline(always)]
fn decode(bits: u32, at: u32, moved: bool, [additive, full]: [u8; 2]) -> [u8; 2] {
    // Each value goes up when its bit matches, else down.
    let (additive, full) = (u32::from(additive), u32::from(full));
    let y = bits ^ (up(additive) | (up(full) << at));
    let child = (additive + 1).wrapping_sub((y << 1) & 2);
    [
        select_unpredictable(moved, child, additive) as u8,
        (full + 1).wrapping_sub((y >> (at - 1)) & 2) as u8,
    ]
}

impl Pdb for Large {
    /// Each group's index, and the blank's rank among the 8-tile group's free cells.
    type Indices = [[u32; 3]; 2];
    /// Each group's additive and full values.
    type Values = [[u8; 4]; 2];

    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        let (seven, eight) = bytes.split_at_mut(B_AT);
        let (seven, eight) = (atomic(seven), atomic(eight));
        for (cost, bit) in [(Cost::Additive, 0), (Cost::Full, 1)] {
            bfs(T, cost, config, |p, blank, _, d| {
                let bit = 2 * blank as usize + bit;
                seven[4 * p as usize + bit / 8].fetch_or(((d >> 1) & 1) << (bit % 8), Relaxed);
            });
        }
        for (cost, byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
            bfs(B, cost, config, |p, _, rank, d| {
                eight[2 * p as usize + byte].fetch_or(((d >> 1) & 1) << rank, Relaxed);
            });
        }
        Self { bytes }
    }

    unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.len() != LEN || xxh3::xxh3_64(&bytes) != HASH {
            return None;
        }
        // SAFETY: We checked above that the data is (almost certainly) correct.
        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        assert_eq!(bytes.len(), LEN, "the database has the wrong length");
        Self { bytes }
    }

    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64) {
        let mut indices = [[0; 3]; 2];
        let mut values = [[0; 4]; 2];
        let mut bound = 0;
        for (s, board) in boards.iter().enumerate() {
            indices[s] = [placement(T, board), placement(B, board), rank(board, mask(B))];
            let seven = |code, bit: u32| {
                walk(
                    T,
                    &SHIFT,
                    0,
                    board,
                    code,
                    |n| (self.seven(n.index, n.blank as usize) >> bit) & 1,
                    |n| self.address(T_AT + 4 * n.index as usize),
                )
            };
            let eight = |code, bit: u32| {
                walk(
                    B,
                    &SHIFT,
                    0,
                    board,
                    code,
                    |n| {
                        let rank = (n.free & ((1 << n.blank) - 1)).count_ones();
                        (self.eight(n.index, rank) >> bit) & 1
                    },
                    |n| self.address(B_AT + 2 * n.index as usize),
                )
            };
            values[s] = [
                seven(Code::Additive, 0),
                seven(Code::Full, 1),
                eight(Code::Additive, 0),
                eight(Code::Full, 8),
            ];
            let [ta, tf, ba, bf] = values[s];
            bound = bound.max(ta + ba).max(tf).max(bf);
        }
        (indices, values, u64::from(bound))
    }

    #[inline(always)]
    fn shifted(
        &self,
        parent: &Self::Indices,
        boards: &[Board; 2],
        m: &Move,
        tiles: [u8; 2],
    ) -> Self::Indices {
        let mut indices = *parent;
        for s in 0..2 {
            let tile = tiles[s];
            let [delta, _] = shift(&SHIFT, &boards[s], m, s, tile);
            let [t, b, rank] = &mut indices[s];
            *t = t.wrapping_add(select_unpredictable((mask(T) >> tile) & 1 != 0, delta, 0));
            *b = b.wrapping_add(select_unpredictable((mask(B) >> tile) & 1 != 0, delta, 0));
            *rank = shift_rank(*rank, &boards[s], m, s, mask(B), tile);
            prefetch(self.address(T_AT + 4 * *t as usize));
            prefetch(self.address(B_AT + 2 * *b as usize));
        }
        indices
    }

    #[inline(always)]
    fn evaluate(
        &self,
        parent: &Self::Values,
        child: &Self::Indices,
        blanks: [u8; 2],
        tiles: [u8; 2],
        depth: u64,
    ) -> Option<(u64, Self::Values)> {
        let mut values = *parent;
        let (mut best, mut total) = (0, 0);
        for s in 0..2 {
            let ([t, b, rank], v, tile) = (child[s], &mut values[s], tiles[s]);
            let seven = self.seven(t, usize::from(blanks[s]));
            [v[0], v[1]] = decode(seven, 1, (mask(T) >> tile) & 1 != 0, [v[0], v[1]]);
            let eight = self.eight(b, rank);
            [v[2], v[3]] = decode(eight, 8, (mask(B) >> tile) & 1 != 0, [v[2], v[3]]);
            let h = u64::from((v[0] + v[2]).max(v[1]).max(v[3]));
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        Some(((best << 10) + total, values))
    }
}

impl AsRef<[u8]> for Large {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "builds an 8-tile table: several minutes and a few gigabytes"]
    fn test_pdb() {
        super::super::tests::check(Large::new(&PdbConfig::default()), HASH);
    }
}
