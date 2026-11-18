//! The tiny pattern database (5.9 MB): the maximum of two 6-6-3 partitions.
//!
//! Each 3-tile group's table holds its additive distance as a byte per placement and blank cell.
//! Each 6-tile group's table holds 2 bits per placement: `(value >> 1) & 3` of its additive
//! distance minimised over the blank's cells and relaxed so that a move changes it by 1 or 3.
//! Builds in about a second.

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

// The groups: the first partition is the top row and the two halves of the rest, the second
// the best of 120 hill-climbing evaluations against it.
const A: &[u8] = &[1, 2, 3];
const B: &[u8] = &[6, 7, 8, 9, 14, 15];
const C: &[u8] = &[4, 5, 10, 11, 12, 13];
const D: &[u8] = &[3, 4, 11];
const E: &[u8] = &[1, 2, 5, 6, 7, 10];
const F: &[u8] = &[8, 9, 12, 13, 14, 15];

/// The groups, in table order.
const GROUPS: [&[u8]; 6] = [A, B, C, D, E, F];

/// The groups' tiles, as masks.
const MASKS: [u16; 6] = [mask(A), mask(B), mask(C), mask(D), mask(E), mask(F)];

/// How moves shift the groups' indices.
const SHIFT: [[u64; 16]; 16] = shift_table([&[A, B, C], &[D, E, F]]);

/// The size of a 3-tile table: a byte per placement and blank cell.
const SMALL: usize = placements(3) * 16;

/// The size of a 6-tile table: 2 bits per placement.
const LARGE: usize = placements(6) / 4;

// Where each table starts.
const A_AT: usize = 0;
const B_AT: usize = A_AT + SMALL;
const C_AT: usize = B_AT + LARGE;
const D_AT: usize = C_AT + LARGE;
const E_AT: usize = D_AT + SMALL;
const F_AT: usize = E_AT + LARGE;

/// The length of the database in bytes.
const LEN: usize = F_AT + LARGE;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0xfe6a_4bd3_1b78_1448;

/// The tiny pattern database. See the [module documentation](self).
pub struct Tiny {
    bytes: Box<[u8]>,
}

impl Tiny {
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

    /// The code of placement `index` in the 6-tile table at `at`.
    #[inline(always)]
    fn code(&self, at: usize, index: u32) -> u32 {
        let index = index as usize;
        u32::from(self.byte(at + index / 4) >> (2 * (index % 4))) & 3
    }

    /// A child's value in the 6-tile table at `at` of the tiles in `group`, from its index and
    /// the moved tile, decoded against its parent's value.
    #[inline(always)]
    fn decode(&self, at: usize, group: u16, index: u32, tile: u8, parent: u8) -> u8 {
        // The child's value is 1 or 3 away (the four candidates' codes count up mod 4), or the
        // same when the moved tile is not in the group.
        let parent = u32::from(parent);
        let step = self.code(at, index).wrapping_sub(u32::midpoint(parent, 5)) & 3;
        let child = (parent + 2 * step).wrapping_sub(3);
        select_unpredictable((group >> tile) & 1 != 0, child, parent) as u8
    }
}

/// Builds the 3-tile table of `tiles` into `out`.
fn build_small(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = atomic(out);
    bfs(tiles, Cost::Additive, config, |p, blank, _, d| {
        out[p as usize * 16 + blank as usize].store(d, Relaxed);
    });
}

/// Builds the 6-tile table of `tiles` into `out`.
fn build_large(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
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

impl Pdb for Tiny {
    /// Each group's index.
    type Indices = [[u32; 6]; 2];
    /// The 6-tile groups' values.
    type Values = [[u8; 4]; 2];

    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        build_small(A, &mut bytes[A_AT..B_AT], config);
        build_large(B, &mut bytes[B_AT..C_AT], config);
        build_large(C, &mut bytes[C_AT..D_AT], config);
        build_small(D, &mut bytes[D_AT..E_AT], config);
        build_large(E, &mut bytes[E_AT..F_AT], config);
        build_large(F, &mut bytes[F_AT..], config);
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
            indices[s] = GROUPS.map(|tiles| placement(tiles, board));
            let value = |tiles, at, lane| {
                walk(
                    tiles,
                    &SHIFT,
                    lane,
                    board,
                    Code::Min2,
                    |n| self.code(at, n.index),
                    |n| self.address(at + n.index as usize / 4),
                )
            };
            values[s] = [
                value(B, B_AT, 0),
                value(C, C_AT, 0),
                value(E, E_AT, 1),
                value(F, F_AT, 1),
            ];
            let a = self.byte(A_AT + 16 * indices[s][0] as usize + blank);
            let d = self.byte(D_AT + 16 * indices[s][3] as usize + blank);
            let [b, c, e, f] = values[s];
            bound = bound.max(a + b + c).max(d + e + f);
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
            let [a, b, c, d, e, f] = indices[s].map(|i| i as usize);
            let blank = usize::from(m.next[s]);
            prefetch(self.address(A_AT + 16 * a + blank));
            prefetch(self.address(B_AT + b / 4));
            prefetch(self.address(C_AT + c / 4));
            prefetch(self.address(D_AT + 16 * d + blank));
            prefetch(self.address(E_AT + e / 4));
            prefetch(self.address(F_AT + f / 4));
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
            let ([a, b, c, ..], v) = (child[s].map(|i| i as usize), &mut values[s]);
            v[0] = self.decode(B_AT, MASKS[1], b as u32, tiles[s], v[0]);
            v[1] = self.decode(C_AT, MASKS[2], c as u32, tiles[s], v[1]);
            let small = self.byte(A_AT + 16 * a + usize::from(blanks[s]));
            let h = u64::from(small + v[0] + v[1]);
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        for s in 0..2 {
            let ([.., d, e, f], v) = (child[s].map(|i| i as usize), &mut values[s]);
            v[2] = self.decode(E_AT, MASKS[4], e as u32, tiles[s], v[2]);
            v[3] = self.decode(F_AT, MASKS[5], f as u32, tiles[s], v[3]);
            let small = self.byte(D_AT + 16 * d + usize::from(blanks[s]));
            let h = u64::from(small + v[2] + v[3]);
            if h >= depth {
                return None;
            }
            best = best.max(h);
            total += h;
        }
        Some(((best << 10) + total, values))
    }
}

impl AsRef<[u8]> for Tiny {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb() {
        super::super::tests::check(Tiny::new(&PdbConfig::default()), HASH);
    }
}
