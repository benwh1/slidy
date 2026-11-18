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
        pdb::{Pdb, Search},
        utils::{
            Board, Move,
            build::{Cost, as_atomic, breadth_first_search, relax_minimums},
            index::{index_shift_table, index_shifts, placement_count, placement_index, tile_mask},
            prefetch,
            walk::{Encoding, WalkSpec, exact_values},
        },
    },
};

// The first partition: the top two rows, the bottom two but the far corner, and the corner.
const TOP_SEVEN: &[u8] = &[1, 2, 3, 4, 5, 6, 7];
const BOTTOM_SEVEN: &[u8] = &[8, 9, 10, 11, 13, 14, 15];
const CORNER: &[u8] = &[12];
// The second: the top row, and the left and right halves of the rest.
const TOP_ROW: &[u8] = &[1, 2, 3];
const LEFT_SIX: &[u8] = &[6, 7, 8, 9, 14, 15];
const RIGHT_SIX: &[u8] = &[4, 5, 10, 11, 12, 13];

/// The groups, in table order (and in the order of [`Medium`]'s indices).
const GROUPS: [&[u8]; 6] = [
    TOP_SEVEN,
    BOTTOM_SEVEN,
    CORNER,
    TOP_ROW,
    LEFT_SIX,
    RIGHT_SIX,
];

/// The groups' tiles as masks, in [`GROUPS`] order.
const GROUP_MASKS: [u16; 6] = [
    tile_mask(TOP_SEVEN),
    tile_mask(BOTTOM_SEVEN),
    tile_mask(CORNER),
    tile_mask(TOP_ROW),
    tile_mask(LEFT_SIX),
    tile_mask(RIGHT_SIX),
];

/// How moves shift the groups' indices.
const INDEX_SHIFTS: [[u64; 16]; 16] = index_shift_table([
    &[TOP_SEVEN, BOTTOM_SEVEN, CORNER],
    &[TOP_ROW, LEFT_SIX, RIGHT_SIX],
]);

/// The size of a 7-tile table: 2 bits per placement.
const SEVEN_TILE_TABLE_LEN: usize = placement_count(7) / 4;

/// The size of a 6-tile table: a `u32` per placement.
const SIX_TILE_TABLE_LEN: usize = placement_count(6) * 4;

/// The size of the 3-tile table: a byte per placement and blank cell.
const THREE_TILE_TABLE_LEN: usize = placement_count(3) * 16;

/// The size of the 1-tile table: a byte per placement and blank cell.
const ONE_TILE_TABLE_LEN: usize = placement_count(1) * 16;

// Where each table starts.
const TOP_SEVEN_START: usize = 0;
const BOTTOM_SEVEN_START: usize = TOP_SEVEN_START + SEVEN_TILE_TABLE_LEN;
const LEFT_SIX_START: usize = BOTTOM_SEVEN_START + SEVEN_TILE_TABLE_LEN;
const RIGHT_SIX_START: usize = LEFT_SIX_START + SIX_TILE_TABLE_LEN;
const TOP_ROW_START: usize = RIGHT_SIX_START + SIX_TILE_TABLE_LEN;
const CORNER_START: usize = TOP_ROW_START + THREE_TILE_TABLE_LEN;

/// The length of the database in bytes.
const LEN: usize = CORNER_START + ONE_TILE_TABLE_LEN;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0xa561_acd8_6ab3_ac0c;

/// The medium pattern database (75 MB): the maximum of a 7-7-1 partition and a 6-6-3 one.
/// Builds in a minute or two.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Medium {
    bytes: Box<[u8]>,
}

/// `((value + 1) >> 1) & 1`: the bit a 6-tile table holds for `value + 1`. A move changes a
/// value by exactly 1, so a child's bit matches this exactly when its value went up.
#[inline(always)]
const fn bit_if_increased(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

impl Medium {
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

    /// The value of placement `index` and blank cell `blank` in the 1- or 3-tile table at
    /// `start`.
    #[inline(always)]
    fn byte_value(&self, start: usize, index: u32, blank: usize) -> u8 {
        self.byte(start + 16 * index as usize + blank)
    }

    /// The code of placement `index` in the 7-tile table at `start`.
    #[inline(always)]
    fn seven_tile_code(&self, start: usize, index: u32) -> u32 {
        let index = index as usize;
        u32::from(self.byte(start + index / 4) >> (2 * (index % 4))) & 3
    }

    /// A child's value in the 7-tile table at `start` of the tiles in `group_mask`, from its
    /// index and the moved tile, decoded against its parent's value.
    #[inline(always)]
    fn decode_seven_tile(
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
            .seven_tile_code(start, index)
            .wrapping_sub(u32::midpoint(parent, 5))
            & 3;
        let child = (parent + 2 * step).wrapping_sub(3);
        select_unpredictable((group_mask >> moved) & 1 != 0, child, parent) as u8
    }

    /// The bits of placement `index` in the 6-tile table at `start`, from blank cell `blank`'s
    /// on.
    #[inline(always)]
    fn six_tile_bits(&self, start: usize, index: u32, blank: usize) -> u32 {
        let offset = start + 4 * index as usize;
        debug_assert!(offset + 4 <= LEN);
        // SAFETY: as in `byte`.
        let word = unsafe {
            self.bytes
                .as_ptr()
                .add(offset)
                .cast::<u32>()
                .read_unaligned()
        };
        word >> (2 * blank)
    }

    /// A child's additive and full values in the 6-tile table at `start` of the tiles in
    /// `group_mask`, from its index, the blank's cell and the moved tile, decoded against its
    /// parent's.
    #[inline(always)]
    fn decode_six_tile(
        &self,
        start: usize,
        group_mask: u16,
        index: u32,
        blank: usize,
        moved: u8,
        [additive, full]: [u8; 2],
    ) -> [u8; 2] {
        // Each value goes up when its bit matches, else down; the additive one only changes
        // when the moved tile is in the group.
        let (additive, full) = (u32::from(additive), u32::from(full));
        let expected = bit_if_increased(additive) | (bit_if_increased(full) << 1);
        let went_down = self.six_tile_bits(start, index, blank) ^ expected;
        let child_additive = (additive + 1).wrapping_sub((went_down << 1) & 2);
        let child_full = (full + 1).wrapping_sub(went_down & 2);
        [
            select_unpredictable((group_mask >> moved) & 1 != 0, child_additive, additive) as u8,
            child_full as u8,
        ]
    }
}

/// Builds the 7-tile table of `tiles` into `out`.
fn build_seven_tile_table(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
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

/// Builds the 6-tile table of `tiles` into `out`.
fn build_six_tile_table(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = as_atomic(out);
    for (cost, cost_bit) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        breadth_first_search(tiles, cost, config, |index, blank, _, distance| {
            // The bytes start zeroed, so only set bits need writing.
            if (distance >> 1) & 1 != 0 {
                let bit = 2 * blank as usize + cost_bit;
                out[4 * index as usize + bit / 8].fetch_or(1 << (bit % 8), Relaxed);
            }
        });
    }
}

/// Builds the table of `tiles` with a byte per placement and blank cell into `out`.
fn build_byte_table(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
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

impl Pdb for Medium {
    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        let top_seven = &mut bytes[TOP_SEVEN_START..BOTTOM_SEVEN_START];
        build_seven_tile_table(TOP_SEVEN, top_seven, config);
        let bottom_seven = &mut bytes[BOTTOM_SEVEN_START..LEFT_SIX_START];
        build_seven_tile_table(BOTTOM_SEVEN, bottom_seven, config);
        build_six_tile_table(
            LEFT_SIX,
            &mut bytes[LEFT_SIX_START..RIGHT_SIX_START],
            config,
        );
        build_six_tile_table(
            RIGHT_SIX,
            &mut bytes[RIGHT_SIX_START..TOP_ROW_START],
            config,
        );
        build_byte_table(TOP_ROW, &mut bytes[TOP_ROW_START..CORNER_START], config);
        build_byte_table(CORNER, &mut bytes[CORNER_START..], config);
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

impl Search for Medium {
    /// Each group's index: top seven, bottom seven, corner, top row, left six, right six.
    type Indices = [[u32; 6]; 2];
    /// The 7-tile groups' values (top, bottom), then the 6-tile groups' additive and full values
    /// (left, right).
    type Values = [[u8; 6]; 2];

    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64) {
        // The tables' walks, both orientations: (tiles, partition, encoding, table start, bit
        // offset), in the order of the values.
        const WALKS: [(&[u8], usize, Encoding, usize, u32); 6] = [
            (TOP_SEVEN, 0, Encoding::RelaxedMinimum, TOP_SEVEN_START, 0),
            (
                BOTTOM_SEVEN,
                0,
                Encoding::RelaxedMinimum,
                BOTTOM_SEVEN_START,
                0,
            ),
            (LEFT_SIX, 1, Encoding::AdditiveBit, LEFT_SIX_START, 0),
            (LEFT_SIX, 1, Encoding::FullBit, LEFT_SIX_START, 1),
            (RIGHT_SIX, 1, Encoding::AdditiveBit, RIGHT_SIX_START, 0),
            (RIGHT_SIX, 1, Encoding::FullBit, RIGHT_SIX_START, 1),
        ];
        let walks = std::array::from_fn::<_, 12, _>(|walk| {
            let (tiles, partition, encoding, ..) = WALKS[walk % 6];
            WalkSpec {
                tiles,
                partition,
                board: &boards[walk / 6],
                encoding,
            }
        });
        let exact = exact_values(
            &INDEX_SHIFTS,
            walks,
            |walk, state| {
                let (.., encoding, start, bit) = WALKS[walk % 6];
                if encoding == Encoding::RelaxedMinimum {
                    self.seven_tile_code(start, state.index)
                } else {
                    (self.six_tile_bits(start, state.index, state.blank as usize) >> bit) & 1
                }
            },
            |walk, state| {
                let (.., encoding, start, _) = WALKS[walk % 6];
                self.address_of(if encoding == Encoding::RelaxedMinimum {
                    start + state.index as usize / 4
                } else {
                    start + 4 * state.index as usize
                })
            },
        );
        let mut indices = [[0; 6]; 2];
        let mut values = [[0; 6]; 2];
        let mut bound = 0;
        for (side, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&tile| tile == 0).unwrap();
            indices[side] = GROUPS.map(|tiles| placement_index(tiles, board));
            values[side] = std::array::from_fn(|walk| exact[6 * side + walk]);
            let [_, _, corner_index, top_row_index, ..] = indices[side];
            let corner = self.byte_value(CORNER_START, corner_index, blank);
            let top_row = self.byte_value(TOP_ROW_START, top_row_index, blank);
            let [
                top,
                bottom,
                left_additive,
                left_full,
                right_additive,
                right_full,
            ] = values[side];
            bound = bound
                .max(top + bottom + corner)
                .max(top_row + left_additive + right_additive)
                .max(left_full)
                .max(right_full);
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
            let [top, bottom, corner, top_row, left, right] =
                indices[side].map(|index| index as usize);
            let blank = usize::from(tile_move.blank_after[side]);
            prefetch(self.address_of(TOP_SEVEN_START + top / 4));
            prefetch(self.address_of(BOTTOM_SEVEN_START + bottom / 4));
            prefetch(self.address_of(CORNER_START + 16 * corner + blank));
            prefetch(self.address_of(TOP_ROW_START + 16 * top_row + blank));
            prefetch(self.address_of(LEFT_SIX_START + 4 * left));
            prefetch(self.address_of(RIGHT_SIX_START + 4 * right));
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
        let [top_mask, bottom_mask, _, _, left_mask, right_mask] = GROUP_MASKS;
        let mut values = *parent;
        let (mut best, mut total) = (0, 0);
        // One partition in both orientations, then the other, stopping at the first that prunes.
        for side in 0..2 {
            let [top_index, bottom_index, corner_index, ..] = child[side];
            let [top, bottom, ..] = &mut values[side];
            let moved = tiles[side];
            *top = self.decode_seven_tile(TOP_SEVEN_START, top_mask, top_index, moved, *top);
            *bottom = self.decode_seven_tile(
                BOTTOM_SEVEN_START,
                bottom_mask,
                bottom_index,
                moved,
                *bottom,
            );
            let blank = usize::from(blanks[side]);
            let corner = self.byte_value(CORNER_START, corner_index, blank);
            let estimate = u64::from(corner + *top + *bottom);
            if estimate >= depth {
                return None;
            }
            best = best.max(estimate);
            total += estimate;
        }
        for side in 0..2 {
            let [.., top_row_index, left_index, right_index] = child[side];
            let [_, _, left_additive, left_full, right_additive, right_full] = &mut values[side];
            let (moved, blank) = (tiles[side], usize::from(blanks[side]));
            [*left_additive, *left_full] = self.decode_six_tile(
                LEFT_SIX_START,
                left_mask,
                left_index,
                blank,
                moved,
                [*left_additive, *left_full],
            );
            [*right_additive, *right_full] = self.decode_six_tile(
                RIGHT_SIX_START,
                right_mask,
                right_index,
                blank,
                moved,
                [*right_additive, *right_full],
            );
            let top_row = self.byte_value(TOP_ROW_START, top_row_index, blank);
            let estimate = u64::from(
                (top_row + *left_additive + *right_additive)
                    .max(*left_full)
                    .max(*right_full),
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
