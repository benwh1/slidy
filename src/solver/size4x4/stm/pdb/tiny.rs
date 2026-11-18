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
        pdb::{Pdb, Search},
        utils::{
            Board, Move,
            build::{Cost, as_atomic, breadth_first_search, relax_minimums},
            index::{index_shift_table, index_shifts, placement_count, placement_index, tile_mask},
            prefetch,
            walk::{Encoding, exact_value},
        },
    },
};

// The groups: the first partition is the top row and the two halves of the rest, the second
// the best of 120 hill-climbing evaluations against it.
const TOP_ROW: &[u8] = &[1, 2, 3];
const LEFT_SIX: &[u8] = &[6, 7, 8, 9, 14, 15];
const RIGHT_SIX: &[u8] = &[4, 5, 10, 11, 12, 13];
const RIGHT_COLUMN: &[u8] = &[3, 4, 11];
const UPPER_SIX: &[u8] = &[1, 2, 5, 6, 7, 10];
const BOTTOM_SIX: &[u8] = &[8, 9, 12, 13, 14, 15];

/// The groups, in table order (and in the order of [`Tiny`]'s indices).
const GROUPS: [&[u8]; 6] = [
    TOP_ROW,
    LEFT_SIX,
    RIGHT_SIX,
    RIGHT_COLUMN,
    UPPER_SIX,
    BOTTOM_SIX,
];

/// The groups' tiles as masks, in [`GROUPS`] order.
const GROUP_MASKS: [u16; 6] = [
    tile_mask(TOP_ROW),
    tile_mask(LEFT_SIX),
    tile_mask(RIGHT_SIX),
    tile_mask(RIGHT_COLUMN),
    tile_mask(UPPER_SIX),
    tile_mask(BOTTOM_SIX),
];

/// How moves shift the groups' indices.
const INDEX_SHIFTS: [[u64; 16]; 16] = index_shift_table([
    &[TOP_ROW, LEFT_SIX, RIGHT_SIX],
    &[RIGHT_COLUMN, UPPER_SIX, BOTTOM_SIX],
]);

/// The size of a 3-tile table: a byte per placement and blank cell.
const THREE_TILE_TABLE_LEN: usize = placement_count(3) * 16;

/// The size of a 6-tile table: 2 bits per placement.
const SIX_TILE_TABLE_LEN: usize = placement_count(6) / 4;

// Where each table starts.
const TOP_ROW_START: usize = 0;
const LEFT_SIX_START: usize = TOP_ROW_START + THREE_TILE_TABLE_LEN;
const RIGHT_SIX_START: usize = LEFT_SIX_START + SIX_TILE_TABLE_LEN;
const RIGHT_COLUMN_START: usize = RIGHT_SIX_START + SIX_TILE_TABLE_LEN;
const UPPER_SIX_START: usize = RIGHT_COLUMN_START + THREE_TILE_TABLE_LEN;
const BOTTOM_SIX_START: usize = UPPER_SIX_START + SIX_TILE_TABLE_LEN;

/// The length of the database in bytes.
const LEN: usize = BOTTOM_SIX_START + SIX_TILE_TABLE_LEN;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0xfe6a_4bd3_1b78_1448;

/// The tiny pattern database (5.9 MB): the maximum of two 6-6-3 partitions. Builds in
/// about a second.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Tiny {
    bytes: Box<[u8]>,
}

impl Tiny {
    #[inline(always)]
    fn byte(&self, offset: usize) -> u8 {
        debug_assert!(offset < LEN);
        // SAFETY: every offset is within its table, and the database has length `LEN`.
        unsafe { *self.bytes.get_unchecked(offset) }
    }

    #[inline(always)]
    fn address_of(&self, offset: usize) -> *const u8 {
        self.bytes.as_ptr().wrapping_add(offset)
    }

    /// The value of placement `index` and blank cell `blank` in the 3-tile table at `start`.
    #[inline(always)]
    fn three_tile_value(&self, start: usize, index: u32, blank: usize) -> u8 {
        self.byte(start + 16 * index as usize + blank)
    }

    /// The code of placement `index` in the 6-tile table at `start`.
    #[inline(always)]
    fn six_tile_code(&self, start: usize, index: u32) -> u32 {
        let index = index as usize;
        u32::from(self.byte(start + index / 4) >> (2 * (index % 4))) & 3
    }

    /// A child's value in the 6-tile table at `start` of the tiles in `group_mask`, from its
    /// index and the moved tile, decoded against its parent's value.
    #[inline(always)]
    fn decode_six_tile(
        &self,
        start: usize,
        group_mask: u16,
        index: u32,
        moved: u8,
        parent: u8,
    ) -> u8 {
        // The child's value is 1 or 3 away (the four candidates' codes count up mod 4), or the
        // same when the moved tile is not in the group.
        let parent = u32::from(parent);
        let step = self
            .six_tile_code(start, index)
            .wrapping_sub(u32::midpoint(parent, 5))
            & 3;
        let child = (parent + 2 * step).wrapping_sub(3);
        let in_group = (group_mask >> moved) & 1 != 0;
        select_unpredictable(in_group, child, parent) as u8
    }
}

/// Builds the 3-tile table of `tiles` into `out`.
fn build_three_tile_table(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = as_atomic(out);
    breadth_first_search(
        tiles,
        Cost::Additive,
        config,
        |index, blank, _, distance| {
            out[index as usize * 16 + blank as usize].store(distance, Relaxed);
        },
    );
}

/// Builds the 6-tile table of `tiles` into `out`.
fn build_six_tile_table(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let mut minimums = vec![u8::MAX; placement_count(tiles.len())];
    let atomic_minimums = as_atomic(&mut minimums);
    breadth_first_search(tiles, Cost::Additive, config, |index, _, _, distance| {
        atomic_minimums[index as usize].fetch_min(distance, Relaxed);
    });
    relax_minimums(tiles, &mut minimums);
    for (index, &minimum) in minimums.iter().enumerate() {
        out[index / 4] |= ((minimum >> 1) & 3) << (2 * (index % 4));
    }
}

impl Pdb for Tiny {
    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        let tables = [
            (TOP_ROW, TOP_ROW_START, LEFT_SIX_START),
            (LEFT_SIX, LEFT_SIX_START, RIGHT_SIX_START),
            (RIGHT_SIX, RIGHT_SIX_START, RIGHT_COLUMN_START),
            (RIGHT_COLUMN, RIGHT_COLUMN_START, UPPER_SIX_START),
            (UPPER_SIX, UPPER_SIX_START, BOTTOM_SIX_START),
            (BOTTOM_SIX, BOTTOM_SIX_START, LEN),
        ];
        for (tiles, start, end) in tables {
            let out = &mut bytes[start..end];
            if tiles.len() == 3 {
                build_three_tile_table(tiles, out, config);
            } else {
                build_six_tile_table(tiles, out, config);
            }
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

impl Search for Tiny {
    /// Each group's index: top row, left six, right six, right column, upper six, bottom six.
    type Indices = [[u32; 6]; 2];
    /// The 6-tile groups' values: left, right, upper, bottom.
    type Values = [[u8; 4]; 2];

    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64) {
        let mut indices = [[0; 6]; 2];
        let mut values = [[0; 4]; 2];
        let mut bound = 0;
        for (side, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&tile| tile == 0).unwrap();
            indices[side] = GROUPS.map(|tiles| placement_index(tiles, board));
            let six_tile_value = |tiles, start, partition| {
                exact_value(
                    tiles,
                    &INDEX_SHIFTS,
                    partition,
                    board,
                    Encoding::RelaxedMinimum,
                    |state| self.six_tile_code(start, state.index),
                    |state| self.address_of(start + state.index as usize / 4),
                )
            };
            values[side] = [
                six_tile_value(LEFT_SIX, LEFT_SIX_START, 0),
                six_tile_value(RIGHT_SIX, RIGHT_SIX_START, 0),
                six_tile_value(UPPER_SIX, UPPER_SIX_START, 1),
                six_tile_value(BOTTOM_SIX, BOTTOM_SIX_START, 1),
            ];
            let [top_row_index, _, _, right_column_index, ..] = indices[side];
            let top_row = self.three_tile_value(TOP_ROW_START, top_row_index, blank);
            let right_column = self.three_tile_value(RIGHT_COLUMN_START, right_column_index, blank);
            let [left, right, upper, bottom] = values[side];
            bound = bound
                .max(top_row + left + right)
                .max(right_column + upper + bottom);
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
            let deltas = index_shifts(&INDEX_SHIFTS, &boards[side], tile_move, side, moved);
            for (group, mask) in GROUP_MASKS.into_iter().enumerate() {
                let in_group = (mask >> moved) & 1 != 0;
                let delta = select_unpredictable(in_group, deltas[group / 3], 0);
                indices[side][group] = indices[side][group].wrapping_add(delta);
            }
            let [top_row, left, right, right_column, upper, bottom] =
                indices[side].map(|index| index as usize);
            let blank = usize::from(tile_move.blank_after[side]);
            prefetch(self.address_of(TOP_ROW_START + 16 * top_row + blank));
            prefetch(self.address_of(LEFT_SIX_START + left / 4));
            prefetch(self.address_of(RIGHT_SIX_START + right / 4));
            prefetch(self.address_of(RIGHT_COLUMN_START + 16 * right_column + blank));
            prefetch(self.address_of(UPPER_SIX_START + upper / 4));
            prefetch(self.address_of(BOTTOM_SIX_START + bottom / 4));
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
        for side in 0..2 {
            let [top_row, left_index, right_index, ..] = child[side];
            let [left, right, ..] = &mut values[side];
            let [_, left_mask, right_mask, ..] = GROUP_MASKS;
            *left = self.decode_six_tile(LEFT_SIX_START, left_mask, left_index, tiles[side], *left);
            *right = self.decode_six_tile(
                RIGHT_SIX_START,
                right_mask,
                right_index,
                tiles[side],
                *right,
            );
            let blank = usize::from(blanks[side]);
            let top_row = self.three_tile_value(TOP_ROW_START, top_row, blank);
            let estimate = u64::from(top_row + *left + *right);
            if estimate >= depth {
                return None;
            }
            best = best.max(estimate);
            total += estimate;
        }
        for side in 0..2 {
            let [.., right_column, upper_index, bottom_index] = child[side];
            let [.., upper, bottom] = &mut values[side];
            let [.., upper_mask, bottom_mask] = GROUP_MASKS;
            *upper = self.decode_six_tile(
                UPPER_SIX_START,
                upper_mask,
                upper_index,
                tiles[side],
                *upper,
            );
            *bottom = self.decode_six_tile(
                BOTTOM_SIX_START,
                bottom_mask,
                bottom_index,
                tiles[side],
                *bottom,
            );
            let blank = usize::from(blanks[side]);
            let right_column = self.three_tile_value(RIGHT_COLUMN_START, right_column, blank);
            let estimate = u64::from(right_column + *upper + *bottom);
            if estimate >= depth {
                return None;
            }
            best = best.max(estimate);
            total += estimate;
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
