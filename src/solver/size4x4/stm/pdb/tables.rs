//! What the databases share: their bytes, and how their tables are built, read and decoded.

use std::{hint::select_unpredictable, sync::atomic::Ordering::Relaxed};

use xxhash_rust::xxh3;

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::utils::{
        Board,
        build::{Cost, as_atomic, breadth_first_search},
    },
};

/// A database's tables: `LEN` bytes, with the [`xxh3`] hash `HASH`.
pub(super) struct Tables<const LEN: usize, const HASH: u64> {
    bytes: Box<[u8]>,
}

impl<const LEN: usize, const HASH: u64> Tables<LEN, HASH> {
    /// Tables that `build` writes, from zeroed bytes.
    pub(super) fn build(build: impl FnOnce(&mut [u8])) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        build(&mut bytes);
        Self { bytes }
    }

    /// The tables in `bytes`, if they have the right length and hash.
    pub(super) fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        (bytes.len() == LEN && xxh3::xxh3_64(&bytes) == HASH).then_some(Self { bytes })
    }

    /// The tables in `bytes`, which must have the right length.
    pub(super) fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        assert_eq!(bytes.len(), LEN, "the database has the wrong length");
        Self { bytes }
    }

    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The byte at `offset`, which is within one of the tables.
    #[inline(always)]
    pub(super) fn byte(&self, offset: usize) -> u8 {
        debug_assert!(offset < LEN);
        // SAFETY: every offset is within its table, and the tables have length `LEN`.
        unsafe { *self.bytes.get_unchecked(offset) }
    }

    /// The little-endian `N` bytes at `offset`, within one of the tables: the tables are
    /// written a byte at a time, low bits first.
    #[inline(always)]
    fn bytes_at<const N: usize>(&self, offset: usize) -> [u8; N] {
        debug_assert!(offset + N <= LEN);
        // SAFETY: as in `byte`.
        unsafe { self.bytes.as_ptr().add(offset).cast::<[u8; N]>().read() }
    }

    /// The `u16` at `offset`, within one of the tables.
    #[inline(always)]
    pub(super) fn u16(&self, offset: usize) -> u16 {
        u16::from_le_bytes(self.bytes_at(offset))
    }

    /// The `u32` at `offset`, within one of the tables.
    #[inline(always)]
    pub(super) fn u32(&self, offset: usize) -> u32 {
        u32::from_le_bytes(self.bytes_at(offset))
    }

    /// The 2-bit code of placement `index` in the code table at `start`.
    #[inline(always)]
    pub(super) fn code(&self, start: usize, index: u32) -> u32 {
        let index = index as usize;
        u32::from(self.byte(start + index / 4) >> (2 * (index % 4))) & 3
    }

    /// The address of `offset`, for prefetching.
    #[inline(always)]
    pub(super) fn address(&self, offset: usize) -> *const u8 {
        self.bytes.as_ptr().wrapping_add(offset)
    }
}

/// Whether `tile` is one of the tiles of `mask`.
#[inline(always)]
pub(super) const fn in_group(mask: u16, tile: u8) -> bool {
    (mask >> tile) & 1 != 0
}

/// A child's value, decoded from its code in a code table (see [`pack_codes`]) against its
/// parent's: 1 or 3 away (the four candidates' codes count up mod 4), or the same when the moved
/// tile is not `in_group`.
#[inline(always)]
pub(super) fn decode_code(code: u32, in_group: bool, parent: u8) -> u8 {
    let parent = u32::from(parent);
    let step = code.wrapping_sub(u32::midpoint(parent, 5)) & 3;
    let child = (parent + 2 * step).wrapping_sub(3);
    select_unpredictable(in_group, child, parent) as u8
}

/// A child's additive and full values, decoded from `bits` (its additive bit, and its full one
/// `full_offset` bits higher; see [`build_cell_bit_table`] and [`build_rank_bit_table`]) against
/// its parent's: a move changes each by exactly 1, up when its bit matches the bit of the
/// parent's value plus 1. The additive value only changes when the moved tile is `in_group`.
#[inline(always)]
pub(super) fn decode_bits(
    bits: u32,
    full_offset: u32,
    in_group: bool,
    [additive, full]: [u8; 2],
) -> [u8; 2] {
    let bit_if_increased = |value: u32| ((value + 1) >> 1) & 1;
    let (additive, full) = (u32::from(additive), u32::from(full));
    let expected = bit_if_increased(additive) | (bit_if_increased(full) << full_offset);
    let went_down = bits ^ expected;
    let child_additive = (additive + 1).wrapping_sub((went_down << 1) & 2);
    let child_full = (full + 1).wrapping_sub((went_down >> (full_offset - 1)) & 2);
    [
        select_unpredictable(in_group, child_additive, additive) as u8,
        child_full as u8,
    ]
}

/// The blank's rank among the cells of `board` without a tile of `group_mask`.
pub(super) fn blank_rank(board: &Board, group_mask: u16) -> u32 {
    let blank = board.iter().position(|&tile| tile == 0).unwrap();
    board[..blank]
        .iter()
        .filter(|&&tile| !in_group(group_mask, tile))
        .count() as u32
}

/// Builds into `out` the table of `tiles` holding, for each placement and blank cell, a byte for
/// each of `costs`' distances.
pub(super) fn build_distance_table(
    tiles: &[u8],
    costs: &[Cost],
    out: &mut [u8],
    config: &PdbConfig,
    threads: usize,
) {
    let out = as_atomic(out);
    for (position, &cost) in costs.iter().enumerate() {
        breadth_first_search(tiles, cost, config, threads, |index, blank, _, distance| {
            let entry = 16 * index as usize + blank as usize;
            out[costs.len() * entry + position].store(distance, Relaxed);
        });
    }
}

/// Packs a group's relaxed minimums into its code table, which `out` holds zeroed: 2 bits per
/// placement, `(value >> 1) & 3`.
pub(super) fn pack_codes(minimums: &[u8], out: &mut [u8]) {
    for (index, &minimum) in minimums.iter().enumerate() {
        out[index / 4] |= ((minimum >> 1) & 3) << (2 * (index % 4));
    }
}

/// Builds into `out` (zeroed) the bit table of `tiles` with a `u32` per placement: for each blank
/// cell `b`, bit `2b` is `(value >> 1) & 1` of the additive distance and bit `2b + 1` that of the
/// full distance.
pub(super) fn build_cell_bit_table(
    tiles: &[u8],
    out: &mut [u8],
    config: &PdbConfig,
    threads: usize,
) {
    let out = as_atomic(out);
    for (cost, cost_bit) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        breadth_first_search(tiles, cost, config, threads, |index, blank, _, distance| {
            // The bytes start zeroed, so only set bits need writing.
            if (distance >> 1) & 1 != 0 {
                let bit = 2 * blank as usize + cost_bit;
                out[4 * index as usize + bit / 8].fetch_or(1 << (bit % 8), Relaxed);
            }
        });
    }
}

/// Builds into `out` (zeroed) the bit table of `tiles` with a `u16` per placement: for the
/// blank's rank `r` among the cells without a group tile, bit `r` is `(value >> 1) & 1` of the
/// additive distance and bit `8 + r` that of the full distance.
pub(super) fn build_rank_bit_table(
    tiles: &[u8],
    out: &mut [u8],
    config: &PdbConfig,
    threads: usize,
) {
    let out = as_atomic(out);
    for (cost, cost_byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        breadth_first_search(tiles, cost, config, threads, |index, _, rank, distance| {
            if (distance >> 1) & 1 != 0 {
                out[2 * index as usize + cost_byte].fetch_or(1 << rank, Relaxed);
            }
        });
    }
}
