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
        pdb::{Pdb, Search},
        utils::{
            Board, Move,
            build::{Cost, as_atomic, breadth_first_search},
            index::{
                index_shift_table, index_shifts, placement_count, placement_index,
                shifted_blank_rank, tile_mask,
            },
            prefetch,
            walk::{Encoding, exact_value},
        },
    },
};

/// The top two rows but the blank's home.
const TOP_SEVEN: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
/// The bottom two rows.
const BOTTOM_EIGHT: &[u8] = &[8, 9, 10, 11, 12, 13, 14, 15];

/// The groups' tiles as masks.
const TOP_SEVEN_MASK: u16 = tile_mask(TOP_SEVEN);
const BOTTOM_EIGHT_MASK: u16 = tile_mask(BOTTOM_EIGHT);

/// How moves shift the groups' indices.
const INDEX_SHIFTS: [[u64; 16]; 16] = index_shift_table([&[TOP_SEVEN, BOTTOM_EIGHT], &[]]);

/// The size of the 7-tile table: a `u32` per placement.
const SEVEN_TILE_TABLE_LEN: usize = placement_count(7) * 4;

/// The size of the 8-tile table: a `u16` per placement.
const EIGHT_TILE_TABLE_LEN: usize = placement_count(8) * 2;

// Where each table starts.
const TOP_SEVEN_START: usize = 0;
const BOTTOM_EIGHT_START: usize = TOP_SEVEN_START + SEVEN_TILE_TABLE_LEN;

/// The length of the database in bytes.
const LEN: usize = BOTTOM_EIGHT_START + EIGHT_TILE_TABLE_LEN;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0x67d0_6ca4_066a_b09c;

/// The large pattern database (1.27 GB): a single 7-8 partition. Builds in about ten
/// minutes.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Large {
    bytes: Box<[u8]>,
}

/// `((value + 1) >> 1) & 1`: the bit a table holds for `value + 1`. A move changes a value by
/// exactly 1, so a child's bit matches this exactly when its value went up.
#[inline(always)]
const fn bit_if_increased(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

/// The blank's rank among the cells of `board` without a tile of `group_mask`.
fn blank_rank(board: &Board, group_mask: u16) -> u32 {
    let blank = board.iter().position(|&tile| tile == 0).unwrap();
    board[..blank]
        .iter()
        .filter(|&&tile| (group_mask >> tile) & 1 == 0)
        .count() as u32
}

impl Large {
    #[inline(always)]
    fn address_of(&self, offset: usize) -> *const u8 {
        self.bytes.as_ptr().wrapping_add(offset)
    }

    /// The 7-tile table's bits for placement `index`, from blank cell `blank`'s on.
    #[inline(always)]
    fn seven_tile_bits(&self, index: u32, blank: usize) -> u32 {
        let offset = TOP_SEVEN_START + 4 * index as usize;
        debug_assert!(offset + 4 <= LEN);
        // SAFETY: every offset is within its table, and the database has length `LEN`.
        let word = unsafe {
            self.bytes
                .as_ptr()
                .add(offset)
                .cast::<u32>()
                .read_unaligned()
        };
        word >> (2 * blank)
    }

    /// The 8-tile table's bits for placement `index`, from blank rank `rank`'s on.
    #[inline(always)]
    fn eight_tile_bits(&self, index: u32, rank: u32) -> u32 {
        let offset = BOTTOM_EIGHT_START + 2 * index as usize;
        debug_assert!(offset + 2 <= LEN);
        // SAFETY: as in `seven_tile_bits`.
        let half = unsafe {
            self.bytes
                .as_ptr()
                .add(offset)
                .cast::<u16>()
                .read_unaligned()
        };
        u32::from(half) >> rank
    }
}

/// A child's additive and full values from `bits` (its additive bit, and its full one
/// `full_offset` bits higher), decoded against its parent's; the additive value only changes
/// when the moved tile is `in_group`.
#[inline(always)]
fn decode_bits(bits: u32, full_offset: u32, in_group: bool, [additive, full]: [u8; 2]) -> [u8; 2] {
    // Each value goes up when its bit matches, else down.
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

impl Pdb for Large {
    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        let (top_seven, bottom_eight) = bytes.split_at_mut(BOTTOM_EIGHT_START);
        let (top_seven, bottom_eight) = (as_atomic(top_seven), as_atomic(bottom_eight));
        for (cost, cost_bit) in [(Cost::Additive, 0), (Cost::Full, 1)] {
            breadth_first_search(TOP_SEVEN, cost, config, |index, blank, _, distance| {
                // The bytes start zeroed, so only set bits need writing.
                if (distance >> 1) & 1 != 0 {
                    let bit = 2 * blank as usize + cost_bit;
                    top_seven[4 * index as usize + bit / 8].fetch_or(1 << (bit % 8), Relaxed);
                }
            });
        }
        for (cost, cost_byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
            breadth_first_search(BOTTOM_EIGHT, cost, config, |index, _, rank, distance| {
                if (distance >> 1) & 1 != 0 {
                    bottom_eight[2 * index as usize + cost_byte].fetch_or(1 << rank, Relaxed);
                }
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
}

impl Search for Large {
    /// Each group's index (top, bottom), and the blank's rank among the 8-tile group's free
    /// cells.
    type Indices = [[u32; 3]; 2];
    /// Each group's additive and full values (top, then bottom).
    type Values = [[u8; 4]; 2];

    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64) {
        let mut indices = [[0; 3]; 2];
        let mut values = [[0; 4]; 2];
        let mut bound = 0;
        for (side, board) in boards.iter().enumerate() {
            indices[side] = [
                placement_index(TOP_SEVEN, board),
                placement_index(BOTTOM_EIGHT, board),
                blank_rank(board, BOTTOM_EIGHT_MASK),
            ];
            let seven_tile_value = |encoding, bit: u32| {
                exact_value(
                    TOP_SEVEN,
                    &INDEX_SHIFTS,
                    0,
                    board,
                    encoding,
                    |state| (self.seven_tile_bits(state.index, state.blank as usize) >> bit) & 1,
                    |state| self.address_of(TOP_SEVEN_START + 4 * state.index as usize),
                )
            };
            let eight_tile_value = |encoding, bit: u32| {
                exact_value(
                    BOTTOM_EIGHT,
                    &INDEX_SHIFTS,
                    0,
                    board,
                    encoding,
                    |state| {
                        let rank = (state.free_cells & ((1 << state.blank) - 1)).count_ones();
                        (self.eight_tile_bits(state.index, rank) >> bit) & 1
                    },
                    |state| self.address_of(BOTTOM_EIGHT_START + 2 * state.index as usize),
                )
            };
            values[side] = [
                seven_tile_value(Encoding::AdditiveBit, 0),
                seven_tile_value(Encoding::FullBit, 1),
                eight_tile_value(Encoding::AdditiveBit, 0),
                eight_tile_value(Encoding::FullBit, 8),
            ];
            let [top_additive, top_full, bottom_additive, bottom_full] = values[side];
            bound = bound
                .max(top_additive + bottom_additive)
                .max(top_full)
                .max(bottom_full);
        }
        (indices, values, u64::from(bound))
    }

    #[inline(always)]
    fn shifted(
        &self,
        parent: &Self::Indices,
        boards: &[Board; 2],
        tile_move: &Move,
        tiles: [u8; 2],
    ) -> Self::Indices {
        let mut indices = *parent;
        for side in 0..2 {
            let moved = tiles[side];
            let board = &boards[side];
            let [delta, _] = index_shifts(&INDEX_SHIFTS, board, tile_move, side, moved);
            let [top, bottom, rank] = &mut indices[side];
            let top_moved = (TOP_SEVEN_MASK >> moved) & 1 != 0;
            *top = top.wrapping_add(select_unpredictable(top_moved, delta, 0));
            let bottom_moved = (BOTTOM_EIGHT_MASK >> moved) & 1 != 0;
            *bottom = bottom.wrapping_add(select_unpredictable(bottom_moved, delta, 0));
            *rank = shifted_blank_rank(*rank, board, tile_move, side, BOTTOM_EIGHT_MASK, moved);
            prefetch(self.address_of(TOP_SEVEN_START + 4 * *top as usize));
            prefetch(self.address_of(BOTTOM_EIGHT_START + 2 * *bottom as usize));
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
        for side in 0..2 {
            let [top_index, bottom_index, rank] = child[side];
            let [top_additive, top_full, bottom_additive, bottom_full] = &mut values[side];
            let moved = tiles[side];
            let top_bits = self.seven_tile_bits(top_index, usize::from(blanks[side]));
            let top_moved = (TOP_SEVEN_MASK >> moved) & 1 != 0;
            [*top_additive, *top_full] =
                decode_bits(top_bits, 1, top_moved, [*top_additive, *top_full]);
            let bottom_bits = self.eight_tile_bits(bottom_index, rank);
            let bottom_moved = (BOTTOM_EIGHT_MASK >> moved) & 1 != 0;
            [*bottom_additive, *bottom_full] = decode_bits(
                bottom_bits,
                8,
                bottom_moved,
                [*bottom_additive, *bottom_full],
            );
            let estimate = u64::from(
                (*top_additive + *bottom_additive)
                    .max(*top_full)
                    .max(*bottom_full),
            );
            if estimate >= depth {
                return None;
            }
            best = best.max(estimate);
            total += estimate;
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
