//! The medium pattern database (75 MB): the maximum of a 7-7-1 partition and a 6-6-3 one.
//!
//! Each 7-tile group's table holds 2 bits per placement: `(value >> 1) & 3` of its additive
//! distance minimised over the blank's cells and relaxed so that a move changes it by 1 or 3. The
//! 1- and 3-tile groups' tables hold their additive distance as a byte per placement and blank
//! cell. Each 6-tile group's table holds a `u32` per placement: for each blank cell `b`, bit `2b`
//! is `(value >> 1) & 1` of the additive distance and bit `2b + 1` that of the full distance
//! (counting every move), which bounds the solution length on its own. Builds in a minute or two.

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

// The first partition: the top two rows, the bottom two but the far corner, and the corner.
const T: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
const B: &[u8] = &[8, 9, 10, 11, 13, 14, 15];
const S: &[u8] = &[12];
// The second: the top row, and the left and right halves of the rest.
const A: &[u8] = &[1, 2, 3];
const L: &[u8] = &[6, 7, 8, 9, 14, 15];
const R: &[u8] = &[4, 5, 10, 11, 12, 13];

/// The groups, in table order.
const GROUPS: [&[u8]; 6] = [T, B, S, A, L, R];

/// The groups' tiles, as masks.
const MASKS: [u16; 6] = [mask(T), mask(B), mask(S), mask(A), mask(L), mask(R)];

/// How moves shift the groups' indices.
const SHIFT: [[u64; 16]; 16] = shift_table([&[T, B, S], &[A, L, R]]);

/// The size of a 7-tile table: 2 bits per placement.
const SEVEN: usize = placements(7) / 4;

/// The size of a 6-tile table: a `u32` per placement.
const SIX: usize = placements(6) * 4;

// Where each table starts.
const T_AT: usize = 0;
const B_AT: usize = T_AT + SEVEN;
const L_AT: usize = B_AT + SEVEN;
const R_AT: usize = L_AT + SIX;
const A_AT: usize = R_AT + SIX;
const S_AT: usize = A_AT + placements(3) * 16;

/// The length of the database in bytes.
const LEN: usize = S_AT + placements(1) * 16;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0x0d61_c7fc_24c1_c3ad;

/// The medium pattern database. See the [module documentation](self).
pub struct Medium {
    bytes: Box<[u8]>,
}

/// `((value + 1) >> 1) & 1`: the bit a 6-tile table holds for `value + 1`. A move changes a
/// value by exactly 1, so a child's bit matches this exactly when its value went up.
#[inline(always)]
const fn up(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

impl Medium {
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

    /// The code of placement `index` in the 7-tile table at `at`.
    #[inline(always)]
    fn code(&self, at: usize, index: u32) -> u32 {
        let index = index as usize;
        u32::from(self.byte(at + index / 4) >> (2 * (index % 4))) & 3
    }

    /// A child's value in the 7-tile table at `at` of the tiles in `group`, from its index and
    /// the moved tile, decoded against its parent's value.
    #[inline(always)]
    fn decode_seven(&self, at: usize, group: u16, index: u32, tile: u8, parent: u8) -> u8 {
        // The child's value is 1 or 3 away (the four candidates' codes count up mod 4), or the
        // same when the moved tile is not in the group.
        let parent = u32::from(parent);
        let step = self.code(at, index).wrapping_sub(u32::midpoint(parent, 5)) & 3;
        let child = (parent + 2 * step).wrapping_sub(3);
        select_unpredictable((group >> tile) & 1 != 0, child, parent) as u8
    }

    /// The bits of placement `index` in the 6-tile table at `at`, from blank cell `blank`'s on.
    #[inline(always)]
    fn bits(&self, at: usize, index: u32, blank: usize) -> u32 {
        let i = at + 4 * index as usize;
        debug_assert!(i + 4 <= LEN);
        // SAFETY: as in `byte`.
        let word = unsafe { self.bytes.as_ptr().add(i).cast::<u32>().read_unaligned() };
        word >> (2 * blank)
    }

    /// A child's additive and full values in the 6-tile table at `at` of the tiles in `group`,
    /// from its index, the blank's cell and the moved tile, decoded against its parent's.
    #[inline(always)]
    fn decode_six(
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
}

/// Builds the 7-tile table of `tiles` into `out`.
fn build_seven(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let mut min = vec![u8::MAX; placements(tiles.len())];
    let atomic_min = atomic(&mut min);
    bfs(tiles, Cost::Additive, config, |p, _, _, d| {
        atomic_min[p as usize].fetch_min(d, Relaxed);
    });
    relax(tiles, &mut min);
    for (p, &v) in min.iter().enumerate() {
        out[p / 4] |= ((v >> 1) & 3) << (2 * (p % 4));
    }
}

/// Builds the 6-tile table of `tiles` into `out`.
fn build_six(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = atomic(out);
    for (cost, bit) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        bfs(tiles, cost, config, |p, blank, _, d| {
            let bit = 2 * blank as usize + bit;
            out[4 * p as usize + bit / 8].fetch_or(((d >> 1) & 1) << (bit % 8), Relaxed);
        });
    }
}

/// Builds the table of `tiles` with a byte per placement and blank cell into `out`.
fn build_bytes(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = atomic(out);
    bfs(tiles, Cost::Additive, config, |p, blank, _, d| {
        out[p as usize * 16 + blank as usize].store(d, Relaxed);
    });
}

impl Pdb for Medium {
    /// Each group's index.
    type Indices = [[u32; 6]; 2];
    /// The 7-tile groups' values, then the 6-tile groups' additive and full values.
    type Values = [[u8; 6]; 2];

    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        build_seven(T, &mut bytes[T_AT..B_AT], config);
        build_seven(B, &mut bytes[B_AT..L_AT], config);
        build_six(L, &mut bytes[L_AT..R_AT], config);
        build_six(R, &mut bytes[R_AT..A_AT], config);
        build_bytes(A, &mut bytes[A_AT..S_AT], config);
        build_bytes(S, &mut bytes[S_AT..], config);
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
        let mut values = [[0; 6]; 2];
        let mut bound = 0;
        for (s, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&t| t == 0).unwrap();
            indices[s] = GROUPS.map(|tiles| placement(tiles, board));
            let seven = |tiles, at| {
                walk(
                    tiles,
                    &SHIFT,
                    0,
                    board,
                    Code::Min2,
                    |n| self.code(at, n.index),
                    |n| self.address(at + n.index as usize / 4),
                )
            };
            let six = |tiles, at, code, bit: u32| {
                walk(
                    tiles,
                    &SHIFT,
                    1,
                    board,
                    code,
                    |n| (self.bits(at, n.index, n.blank as usize) >> bit) & 1,
                    |n| self.address(at + 4 * n.index as usize),
                )
            };
            values[s] = [
                seven(T, T_AT),
                seven(B, B_AT),
                six(L, L_AT, Code::Additive, 0),
                six(L, L_AT, Code::Full, 1),
                six(R, R_AT, Code::Additive, 0),
                six(R, R_AT, Code::Full, 1),
            ];
            let one = self.byte(S_AT + 16 * indices[s][2] as usize + blank);
            let three = self.byte(A_AT + 16 * indices[s][3] as usize + blank);
            let [t, b, la, lf, ra, rf] = values[s];
            bound = bound.max(t + b + one).max(three + la + ra).max(lf).max(rf);
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
                *index = index.wrapping_add(select_unpredictable(moved, delta[g / 3], 0));
            }
            let [t, b, one, three, l, r] = indices[s].map(|i| i as usize);
            let blank = usize::from(m.next[s]);
            prefetch(self.address(T_AT + t / 4));
            prefetch(self.address(B_AT + b / 4));
            prefetch(self.address(S_AT + 16 * one + blank));
            prefetch(self.address(A_AT + 16 * three + blank));
            prefetch(self.address(L_AT + 4 * l));
            prefetch(self.address(R_AT + 4 * r));
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
            let ([t, b, one, ..], v) = (child[s], &mut values[s]);
            v[0] = self.decode_seven(T_AT, MASKS[0], t, tiles[s], v[0]);
            v[1] = self.decode_seven(B_AT, MASKS[1], b, tiles[s], v[1]);
            let one = self.byte(S_AT + 16 * one as usize + usize::from(blanks[s]));
            let h = u64::from(one + v[0] + v[1]);
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        for s in 0..2 {
            let ([.., three, l, r], v) = (child[s], &mut values[s]);
            let blank = usize::from(blanks[s]);
            [v[2], v[3]] = self.decode_six(L_AT, MASKS[4], l, blank, tiles[s], [v[2], v[3]]);
            [v[4], v[5]] = self.decode_six(R_AT, MASKS[5], r, blank, tiles[s], [v[4], v[5]]);
            let three = self.byte(A_AT + 16 * three as usize + blank);
            let h = u64::from((three + v[2] + v[4]).max(v[3]).max(v[5]));
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        Some(((best << 10) + total, values))
    }
}

impl AsRef<[u8]> for Medium {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "builds 7-tile tables: about a minute"]
    fn test_pdb() {
        super::super::tests::check(Medium::new(&PdbConfig::default()), HASH);
    }
}
