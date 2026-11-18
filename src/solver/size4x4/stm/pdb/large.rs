//! The large pattern database (3.5 GB): the maximum of a 7-8 partition and a 9-6 one.
//!
//! The 7- and 8-tile tables hold a `u32` per placement: for each blank cell `b`, bit `2b` is
//! `(value >> 1) & 1` of the group's additive distance and bit `2b + 1` that of its full distance
//! (counting every move), which bounds the solution length on its own. The 9-tile table holds 2
//! bits per placement: `(value >> 1) & 3` of its additive distance minimised over the blank's
//! cells and relaxed so that a move changes it by 1 or 3. The 6-tile table holds a `u16` per
//! placement and blank cell: the additive distance in the low byte, the full one in the high
//! byte. Builds in about half an hour, using up to 15 GB.

use std::{hint::select_unpredictable, sync::atomic::Ordering::Relaxed};

use xxhash_rust::xxh3;

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::{
        pdb::Pdb,
        utils::{
            Board, Move,
            build::{Cost, atomic, bfs, relax},
            index::{mask, placement, placements, shift, shift_table},
            prefetch,
            walk::{Code, walk},
        },
    },
};

// The first partition: the top two rows but the blank's home, and the bottom two.
const T: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
const B: &[u8] = &[8, 9, 10, 11, 12, 13, 14, 15];
// The second: the first nine tiles, and the rest.
const N: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
const R: &[u8] = &[10, 11, 12, 13, 14, 15];

/// The groups, in table order.
const GROUPS: [&[u8]; 4] = [T, B, N, R];

/// The groups' tiles, as masks.
const MASKS: [u16; 4] = [mask(T), mask(B), mask(N), mask(R)];

/// How moves shift the groups' indices.
const SHIFT: [[u64; 16]; 16] = shift_table([&[T, B], &[N, R]]);

// Where each table starts.
const T_AT: usize = 0;
const B_AT: usize = T_AT + placements(7) * 4;
const N_AT: usize = B_AT + placements(8) * 4;
const R_AT: usize = N_AT + placements(9) / 4;

/// The length of the database in bytes.
const LEN: usize = R_AT + placements(6) * 32;

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

/// `((value + 1) >> 1) & 1`: the bit a 7- or 8-tile table holds for `value + 1`. A move changes a
/// value by exactly 1, so a child's bit matches this exactly when its value went up.
#[inline(always)]
const fn up(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

impl Large {
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

    /// The bits of placement `index` in the 7- or 8-tile table at `at`, from blank cell
    /// `blank`'s on.
    #[inline(always)]
    fn bits(&self, at: usize, index: u32, blank: usize) -> u32 {
        let i = at + 4 * index as usize;
        debug_assert!(i + 4 <= LEN);
        // SAFETY: as in `byte`.
        let word = unsafe { self.bytes.as_ptr().add(i).cast::<u32>().read_unaligned() };
        word >> (2 * blank)
    }

    /// A child's additive and full values in the 7- or 8-tile table at `at` of the tiles in
    /// `group`, from its index, the blank's cell and the moved tile, decoded against its
    /// parent's.
    #[inline(always)]
    fn decode_pair(
        &self,
        at: usize,
        group: u16,
        index: u32,
        blank: usize,
        tile: u8,
        [additive, full]: [u8; 2],
    ) -> [u8; 2] {
        // Each value goes up when its bit matches, else down; the additive one only changes
        // when the moved tile is in the group.
        let (additive, full) = (u32::from(additive), u32::from(full));
        let y = self.bits(at, index, blank) ^ (up(additive) | (up(full) << 1));
        let child = (additive + 1).wrapping_sub((y << 1) & 2);
        [
            select_unpredictable((group >> tile) & 1 != 0, child, additive) as u8,
            (full + 1).wrapping_sub(y & 2) as u8,
        ]
    }

    /// The code of placement `index` in the 9-tile table.
    #[inline(always)]
    fn code(&self, index: u32) -> u32 {
        let index = index as usize;
        u32::from(self.byte(N_AT + index / 4) >> (2 * (index % 4))) & 3
    }

    /// A child's value in the 9-tile table, from its index and the moved tile, decoded against
    /// its parent's value.
    #[inline(always)]
    fn decode_nine(&self, index: u32, tile: u8, parent: u8) -> u8 {
        // The child's value is 1 or 3 away (the four candidates' codes count up mod 4), or the
        // same when the moved tile is not in the group.
        let parent = u32::from(parent);
        let step = self.code(index).wrapping_sub(u32::midpoint(parent, 5)) & 3;
        let child = (parent + 2 * step).wrapping_sub(3);
        select_unpredictable((MASKS[2] >> tile) & 1 != 0, child, parent) as u8
    }

    /// The 6-tile table's additive and full values for placement `index` and blank cell `blank`.
    #[inline(always)]
    fn six(&self, index: u32, blank: usize) -> [u8; 2] {
        let i = R_AT + 32 * index as usize + 2 * blank;
        [self.byte(i), self.byte(i + 1)]
    }
}

/// Builds the 7- or 8-tile table of `tiles` into `out`.
fn build_pair(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = atomic(out);
    for (cost, bit) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        bfs(tiles, cost, config, |p, blank, _, d| {
            let bit = 2 * blank as usize + bit;
            out[4 * p as usize + bit / 8].fetch_or(((d >> 1) & 1) << (bit % 8), Relaxed);
        });
    }
}

impl Pdb for Large {
    /// Each group's index.
    type Indices = [[u32; 4]; 2];
    /// The 7- and 8-tile groups' additive and full values, then the 9-tile group's value.
    type Values = [[u8; 5]; 2];

    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        build_pair(T, &mut bytes[T_AT..B_AT], config);
        build_pair(B, &mut bytes[B_AT..N_AT], config);
        {
            let mut min = vec![u8::MAX; placements(9)];
            let atomic_min = atomic(&mut min);
            bfs(N, Cost::Additive, config, |p, _, _, d| {
                atomic_min[p as usize].fetch_min(d, Relaxed);
            });
            relax(N, &mut min);
            let out = &mut bytes[N_AT..R_AT];
            for (p, &v) in min.iter().enumerate() {
                out[p / 4] |= ((v >> 1) & 3) << (2 * (p % 4));
            }
        }
        let out = atomic(&mut bytes[R_AT..]);
        for (cost, byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
            bfs(R, cost, config, |p, blank, _, d| {
                out[32 * p as usize + 2 * blank as usize + byte].store(d, Relaxed);
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
        let mut indices = [[0; 4]; 2];
        let mut values = [[0; 5]; 2];
        let mut bound = 0;
        for (s, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&t| t == 0).unwrap();
            indices[s] = GROUPS.map(|tiles| placement(tiles, board));
            let pair = |tiles, at, code, bit: u32| {
                walk(
                    tiles,
                    &SHIFT,
                    0,
                    board,
                    code,
                    |n| (self.bits(at, n.index, n.blank as usize) >> bit) & 1,
                    |n| self.address(at + 4 * n.index as usize),
                )
            };
            let nine = walk(
                N,
                &SHIFT,
                1,
                board,
                Code::Min2,
                |n| self.code(n.index),
                |n| self.address(N_AT + n.index as usize / 4),
            );
            values[s] = [
                pair(T, T_AT, Code::Additive, 0),
                pair(T, T_AT, Code::Full, 1),
                pair(B, B_AT, Code::Additive, 0),
                pair(B, B_AT, Code::Full, 1),
                nine,
            ];
            let [ta, tf, ba, bf, n] = values[s];
            let [ra, rf] = self.six(indices[s][3], blank);
            bound = bound.max(ta + ba).max(tf).max(bf).max(n + ra).max(rf);
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
            let delta = shift(&SHIFT, &boards[s], m, s, tile);
            for (g, group) in MASKS.into_iter().enumerate() {
                let moved = (group >> tile) & 1 != 0;
                let index = &mut indices[s][g];
                *index = index.wrapping_add(select_unpredictable(moved, delta[g / 2], 0));
            }
            let [t, b, n, r] = indices[s].map(|i| i as usize);
            let blank = usize::from(m.next[s]);
            prefetch(self.address(T_AT + 4 * t));
            prefetch(self.address(B_AT + 4 * b));
            prefetch(self.address(N_AT + n / 4));
            prefetch(self.address(R_AT + 32 * r + 2 * blank));
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
            let ([t, b, ..], v, tile) = (child[s], &mut values[s], tiles[s]);
            let blank = usize::from(blanks[s]);
            [v[0], v[1]] = self.decode_pair(T_AT, MASKS[0], t, blank, tile, [v[0], v[1]]);
            [v[2], v[3]] = self.decode_pair(B_AT, MASKS[1], b, blank, tile, [v[2], v[3]]);
            let h = u64::from((v[0] + v[2]).max(v[1]).max(v[3]));
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        for s in 0..2 {
            let ([.., n, r], v) = (child[s], &mut values[s]);
            v[4] = self.decode_nine(n, tiles[s], v[4]);
            let [ra, rf] = self.six(r, usize::from(blanks[s]));
            let h = u64::from((v[4] + ra).max(rf));
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
    #[ignore = "builds a 9-tile table: about half an hour and 15 GB"]
    fn test_pdb() {
        super::super::tests::check(Large::new(&PdbConfig::default()), HASH);
    }
}
