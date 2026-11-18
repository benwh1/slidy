//! The huge pattern database (17.0 GB): the maximum of two 9-6 partitions.
//!
//! Each 6-tile table holds a `u16` per placement and blank cell: the group's additive distance in
//! the low byte, its full distance (counting every move) in the high byte. Each 9-tile table
//! holds a `u16` per placement, indexed by the blank's rank `r` among the 7 cells without a group
//! tile: bit `r` is `(value >> 1) & 1` of the additive distance, bit `8 + r` that of the full
//! distance. A full distance bounds the solution length on its own. Builds in about an hour,
//! using up to 25 GB.

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

// The first partition: the bottom row and a half and the rest; the second: the top row and a
// half and the rest.
const R: &[u8] = &[10, 11, 12, 13, 14, 15];
const T: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
const L: &[u8] = &[1, 2, 3, 4, 5, 6];
const B: &[u8] = &[7, 8, 9, 10, 11, 12, 13, 14, 15];

/// The groups' tiles, as masks.
const MASKS: [u16; 4] = [mask(R), mask(T), mask(L), mask(B)];

/// How moves shift the groups' indices.
const SHIFT: [[u64; 16]; 16] = shift_table([&[R, T], &[L, B]]);

/// The size of a 6-tile table: a `u16` per placement and blank cell.
const SIX: usize = placements(6) * 32;

/// The size of a 9-tile table: a `u16` per placement.
const NINE: usize = placements(9) * 2;

// Where each table starts.
const R_AT: usize = 0;
const L_AT: usize = R_AT + SIX;
const T_AT: usize = L_AT + SIX;
const B_AT: usize = T_AT + NINE;

/// The length of the database in bytes.
const LEN: usize = B_AT + NINE;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
///
/// Not yet computed (it needs a full build), so `try_from_bytes` rejects every input for now;
/// `from_bytes_unchecked` works.
// TODO: set from `test_pdb`'s output once the database has been built.
const HASH: u64 = 0;

/// The huge pattern database. See the [module documentation](self).
pub struct Huge {
    bytes: Box<[u8]>,
}

/// `((value + 1) >> 1) & 1`: the bit a 9-tile table holds for `value + 1`. A move changes a value
/// by exactly 1, so a child's bit matches this exactly when its value went up.
#[inline(always)]
const fn up(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

/// The blank's rank among the cells of `board` without a tile of `group`.
fn rank(board: &Board, group: u16) -> u32 {
    let blank = board.iter().position(|&t| t == 0).unwrap();
    board[..blank].iter().filter(|&&t| (group >> t) & 1 == 0).count() as u32
}

impl Huge {
    #[inline(always)]
    fn byte(&self, i: usize) -> u8 {
        debug_assert!(i < LEN);
        // SAFETY: every index is within its table, and the database has length `LEN`.
        unsafe { *self.bytes.get_unchecked(i) }
    }

    #[inline(always)]
    fn address(&self, i: usize) -> *const u8 {
        self.bytes.as_ptr().wrapping_add(i)
    }

    /// The additive and full values of placement `index` and blank cell `blank` in the 6-tile
    /// table at `at`.
    #[inline(always)]
    fn six(&self, at: usize, index: u32, blank: usize) -> [u8; 2] {
        let i = at + 32 * index as usize + 2 * blank;
        [self.byte(i), self.byte(i + 1)]
    }

    /// The bits of placement `index` in the 9-tile table at `at`, from blank rank `rank`'s on.
    #[inline(always)]
    fn bits(&self, at: usize, index: u32, rank: u32) -> u32 {
        let i = at + 2 * index as usize;
        debug_assert!(i + 2 <= LEN);
        // SAFETY: as in `byte`.
        let half = unsafe { self.bytes.as_ptr().add(i).cast::<u16>().read_unaligned() };
        u32::from(half) >> rank
    }

    /// A child's additive and full values in the 9-tile table at `at` of the tiles in `group`,
    /// from its index, the blank's rank and the moved tile, decoded against its parent's.
    #[inline(always)]
    fn decode(
        &self,
        at: usize,
        group: u16,
        index: u32,
        rank: u32,
        tile: u8,
        [additive, full]: [u8; 2],
    ) -> [u8; 2] {
        // Each value goes up when its bit matches, else down; the additive one only changes
        // when the moved tile is in the group.
        let (additive, full) = (u32::from(additive), u32::from(full));
        let y = self.bits(at, index, rank) ^ (up(additive) | (up(full) << 8));
        let child = (additive + 1).wrapping_sub((y << 1) & 2);
        [
            select_unpredictable((group >> tile) & 1 != 0, child, additive) as u8,
            (full + 1).wrapping_sub((y >> 7) & 2) as u8,
        ]
    }
}

/// Builds the 6-tile table of `tiles` into `out`.
fn build_six(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = atomic(out);
    for (cost, byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        bfs(tiles, cost, config, |p, blank, _, d| {
            out[32 * p as usize + 2 * blank as usize + byte].store(d, Relaxed);
        });
    }
}

/// Builds the 9-tile table of `tiles` into `out`.
fn build_nine(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = atomic(out);
    for (cost, byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        bfs(tiles, cost, config, |p, _, rank, d| {
            out[2 * p as usize + byte].fetch_or(((d >> 1) & 1) << rank, Relaxed);
        });
    }
}

impl Pdb for Huge {
    /// Each group's index, then the blank's rank among each 9-tile group's free cells.
    type Indices = [[u32; 6]; 2];
    /// The 9-tile groups' additive and full values.
    type Values = [[u8; 4]; 2];

    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        build_six(R, &mut bytes[R_AT..L_AT], config);
        build_six(L, &mut bytes[L_AT..T_AT], config);
        build_nine(T, &mut bytes[T_AT..B_AT], config);
        build_nine(B, &mut bytes[B_AT..], config);
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
        let mut indices = [[0; 6]; 2];
        let mut values = [[0; 4]; 2];
        let mut bound = 0;
        for (s, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&t| t == 0).unwrap();
            indices[s] = [
                placement(R, board),
                placement(T, board),
                placement(L, board),
                placement(B, board),
                rank(board, mask(T)),
                rank(board, mask(B)),
            ];
            let nine = |tiles, at, lane, code, bit: u32| {
                walk(
                    tiles,
                    &SHIFT,
                    lane,
                    board,
                    code,
                    |n| {
                        let rank = (n.free & ((1 << n.blank) - 1)).count_ones();
                        (self.bits(at, n.index, rank) >> bit) & 1
                    },
                    |n| self.address(at + 2 * n.index as usize),
                )
            };
            values[s] = [
                nine(T, T_AT, 0, Code::Additive, 0),
                nine(T, T_AT, 0, Code::Full, 8),
                nine(B, B_AT, 1, Code::Additive, 0),
                nine(B, B_AT, 1, Code::Full, 8),
            ];
            let [ta, tf, ba, bf] = values[s];
            let [ra, rf] = self.six(R_AT, indices[s][0], blank);
            let [la, lf] = self.six(L_AT, indices[s][2], blank);
            bound = bound.max(ra + ta).max(rf).max(tf).max(la + ba).max(lf).max(bf);
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
            let (tile, board) = (tiles[s], &boards[s]);
            let delta = shift(&SHIFT, board, m, s, tile);
            let index = &mut indices[s];
            for (g, group) in MASKS.into_iter().enumerate() {
                let moved = (group >> tile) & 1 != 0;
                index[g] = index[g].wrapping_add(select_unpredictable(moved, delta[g / 2], 0));
            }
            index[4] = shift_rank(index[4], board, m, s, MASKS[1], tile);
            index[5] = shift_rank(index[5], board, m, s, MASKS[3], tile);
            let [r, t, l, b, ..] = index.map(|i| i as usize);
            let blank = usize::from(m.next[s]);
            prefetch(self.address(R_AT + 32 * r + 2 * blank));
            prefetch(self.address(T_AT + 2 * t));
            prefetch(self.address(L_AT + 32 * l + 2 * blank));
            prefetch(self.address(B_AT + 2 * b));
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
        // One partition in both orientations, then the other, stopping at the first that prunes.
        for s in 0..2 {
            let ([r, t, _, _, rank, _], v) = (child[s], &mut values[s]);
            [v[0], v[1]] = self.decode(T_AT, MASKS[1], t, rank, tiles[s], [v[0], v[1]]);
            let [ra, rf] = self.six(R_AT, r, usize::from(blanks[s]));
            let h = u64::from((ra + v[0]).max(rf).max(v[1]));
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        for s in 0..2 {
            let ([_, _, l, b, _, rank], v) = (child[s], &mut values[s]);
            [v[2], v[3]] = self.decode(B_AT, MASKS[3], b, rank, tiles[s], [v[2], v[3]]);
            let [la, lf] = self.six(L_AT, l, usize::from(blanks[s]));
            let h = u64::from((la + v[2]).max(lf).max(v[3]));
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        Some(((best << 10) + total, values))
    }
}

impl AsRef<[u8]> for Huge {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "builds two 9-tile tables: about an hour and 25 GB"]
    fn test_pdb() {
        super::super::tests::check(Huge::new(&PdbConfig::default()), HASH);
    }
}
