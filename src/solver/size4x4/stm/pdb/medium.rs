//! The medium pattern database (75 MB): the maximum of a 7-7-1 partition and a 6-6-3 one.
//!
//! Each 7-tile group's table holds 2 bits per placement: `(value >> 1) & 3` of its additive
//! distance minimised over the blank's cells and relaxed so that a move changes it by 1 or 3. The
//! 1- and 3-tile groups' tables hold their additive distance as a byte per placement and blank
//! cell. Each 6-tile group's table holds a `u32` per placement: for each blank cell `b`, bit `2b`
//! is `(value >> 1) & 1` of the additive distance and bit `2b + 1` that of the full distance
//! (counting every move), which bounds the solution length on its own. Builds in a minute or two.

use std::{hint::select_unpredictable, num::NonZeroUsize};

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::{
        pdb::{
            Pdb, Search, SortKey,
            tables::{
                Tables, build_cell_bit_table, build_distance_table, decode_bits, decode_code,
                in_group, pack_codes,
            },
        },
        utils::{
            Board, Move, blank_position,
            build::{Cost, relaxed_minimums},
            index::{index_shift_table, move_shifts, placement_count, placement_index, tile_mask},
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

/// The [`xxh3`](xxhash_rust::xxh3) hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0xa561_acd8_6ab3_ac0c;

/// The medium pattern database (75 MB): the maximum of a 7-7-1 partition and a 6-6-3 one.
/// Builds in a minute or two.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Medium {
    tables: Tables<LEN, HASH>,
}

impl Medium {
    /// The value of placement `index` and blank cell `blank` in the 1- or 3-tile table at
    /// `start`.
    #[inline(always)]
    fn byte_value(&self, start: usize, index: u32, blank: usize) -> u8 {
        self.tables.byte(start + 16 * index as usize + blank)
    }

    /// The bits of placement `index` in the 6-tile table at `start`, from blank cell `blank`'s
    /// on.
    #[inline(always)]
    fn six_tile_bits(&self, start: usize, index: u32, blank: usize) -> u32 {
        self.tables.u32(start + 4 * index as usize) >> (2 * blank)
    }
}

impl Pdb for Medium {
    fn new_with_thread_count(config: &PdbConfig, threads: NonZeroUsize) -> Self {
        let threads = threads.get();
        let tables = Tables::build(|bytes| {
            for (tiles, start) in [
                (TOP_SEVEN, TOP_SEVEN_START),
                (BOTTOM_SEVEN, BOTTOM_SEVEN_START),
            ] {
                let table = &mut bytes[start..][..SEVEN_TILE_TABLE_LEN];
                pack_codes(&relaxed_minimums(tiles, config, threads), table);
            }
            for (tiles, start) in [(LEFT_SIX, LEFT_SIX_START), (RIGHT_SIX, RIGHT_SIX_START)] {
                let table = &mut bytes[start..][..SIX_TILE_TABLE_LEN];
                build_cell_bit_table(tiles, table, config, threads);
            }
            for (tiles, start, len) in [
                (TOP_ROW, TOP_ROW_START, THREE_TILE_TABLE_LEN),
                (CORNER, CORNER_START, ONE_TILE_TABLE_LEN),
            ] {
                let table = &mut bytes[start..][..len];
                build_distance_table(tiles, &[Cost::Additive], table, config, threads);
            }
        });
        Self { tables }
    }

    unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        Tables::try_from_bytes(bytes).map(|tables| Self { tables })
    }

    unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        Self {
            tables: Tables::from_bytes_unchecked(bytes),
        }
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
                    self.tables.code(start, state.index)
                } else {
                    (self.six_tile_bits(start, state.index, state.blank as usize) >> bit) & 1
                }
            },
            |walk, state| {
                let (.., encoding, start, _) = WALKS[walk % 6];
                self.tables
                    .address(if encoding == Encoding::RelaxedMinimum {
                        start + state.index as usize / 4
                    } else {
                        start + 4 * state.index as usize
                    })
            },
        );
        let indices = boards.map(|board| GROUPS.map(|tiles| placement_index(tiles, &board)));
        let values = [0, 1].map(|side| std::array::from_fn(|value| exact[6 * side + value]));
        let mut bound = 0;
        for (side, board) in boards.iter().enumerate() {
            let blank = blank_position(board);
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
            let deltas =
                move_shifts(&INDEX_SHIFTS, &boards[side], tile_move, side, moved, []).indices;
            for (group, index) in indices[side].iter_mut().enumerate() {
                let delta =
                    select_unpredictable(in_group(GROUP_MASKS[group], moved), deltas[group / 3], 0);
                *index = index.wrapping_add(delta);
            }
            let [top, bottom, corner, top_row, left, right] =
                indices[side].map(|index| index as usize);
            let blank = usize::from(tile_move.blank_after[side]);
            prefetch(self.tables.address(TOP_SEVEN_START + top / 4));
            prefetch(self.tables.address(BOTTOM_SEVEN_START + bottom / 4));
            prefetch(self.tables.address(CORNER_START + 16 * corner + blank));
            prefetch(self.tables.address(TOP_ROW_START + 16 * top_row + blank));
            prefetch(self.tables.address(LEFT_SIX_START + 4 * left));
            prefetch(self.tables.address(RIGHT_SIX_START + 4 * right));
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
        let mut key = SortKey::default();
        // One partition in both orientations, then the other, stopping at the first that prunes.
        for side in 0..2 {
            let [top_index, bottom_index, corner_index, ..] = child[side];
            let [top, bottom, ..] = &mut values[side];
            let (moved, blank) = (tiles[side], usize::from(blanks[side]));
            let top_code = self.tables.code(TOP_SEVEN_START, top_index);
            *top = decode_code(top_code, in_group(top_mask, moved), *top);
            let bottom_code = self.tables.code(BOTTOM_SEVEN_START, bottom_index);
            *bottom = decode_code(bottom_code, in_group(bottom_mask, moved), *bottom);
            let corner = self.byte_value(CORNER_START, corner_index, blank);
            key.add(u64::from(corner + *top + *bottom), depth)?;
        }
        for side in 0..2 {
            let [.., top_row_index, left_index, right_index] = child[side];
            let [_, _, left_additive, left_full, right_additive, right_full] = &mut values[side];
            let (moved, blank) = (tiles[side], usize::from(blanks[side]));
            [*left_additive, *left_full] = decode_bits(
                self.six_tile_bits(LEFT_SIX_START, left_index, blank),
                1,
                in_group(left_mask, moved),
                [*left_additive, *left_full],
            );
            [*right_additive, *right_full] = decode_bits(
                self.six_tile_bits(RIGHT_SIX_START, right_index, blank),
                1,
                in_group(right_mask, moved),
                [*right_additive, *right_full],
            );
            let top_row = self.byte_value(TOP_ROW_START, top_row_index, blank);
            let estimate = (top_row + *left_additive + *right_additive)
                .max(*left_full)
                .max(*right_full);
            key.add(u64::from(estimate), depth)?;
        }
        Some((key.key(), values))
    }
}

impl AsRef<[u8]> for Medium {
    fn as_ref(&self) -> &[u8] {
        self.tables.bytes()
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
