//! The small pattern database (5.9 MB): the maximum of two 6-6-3 partitions.
//!
//! Each 3-tile group's table holds its additive distance as a byte per placement and blank cell.
//! Each 6-tile group's table holds 2 bits per placement: `(value >> 1) & 3` of its additive
//! distance minimised over the blank's cells and relaxed so that a move changes it by 1 or 3.
//! Builds in under a second (about 0.8 s on an i5-9500's 6 cores).

use std::{hint::select_unpredictable, num::NonZeroUsize};

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::{
        pdb::{
            Pdb, Search, SortKey,
            tables::{Tables, build_distance_table, decode_code, in_group, pack_codes},
        },
        utils::{
            Board, Move, blank_position,
            build::{Cost, mirrored_relaxed_minimums, relaxed_minimums},
            index::{index_shift_table, move_shifts, placement_count, placement_index, tile_mask},
            prefetch,
            walk::{Encoding, WalkSpec, exact_values},
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

/// The groups, in table order (and in the order of [`Small`]'s indices): each partition's 3-tile
/// group, then its 6-tile groups.
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

/// The 6-tile groups' positions in [`GROUPS`], in the order of [`Small`]'s values.
const SIX_TILE_GROUPS: [usize; 4] = [1, 2, 4, 5];

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

/// Where each group's table starts, in [`GROUPS`] order.
const STARTS: [usize; 6] = [
    TOP_ROW_START,
    LEFT_SIX_START,
    RIGHT_SIX_START,
    RIGHT_COLUMN_START,
    UPPER_SIX_START,
    BOTTOM_SIX_START,
];

/// The length of the database in bytes.
const LEN: usize = BOTTOM_SIX_START + SIX_TILE_TABLE_LEN;

/// The [`xxh3`](xxhash_rust::xxh3) hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0x8dd1_306a_33a8_d57d;

/// The small pattern database (5.9 MB): the maximum of two 6-6-3 partitions. Builds in under a
/// second.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Small {
    tables: Tables<LEN, HASH>,
}

impl Pdb for Small {
    fn new_with_thread_count(config: &PdbConfig, threads: NonZeroUsize) -> Self {
        let threads = threads.get();
        let tables = Tables::build(|bytes| {
            let additive = &[Cost::Additive];
            build_distance_table(
                TOP_ROW,
                additive,
                &mut bytes[TOP_ROW_START..][..THREE_TILE_TABLE_LEN],
                config,
                threads,
            );
            // RIGHT_SIX is LEFT_SIX reflected left to right.
            let (left, right) = mirrored_relaxed_minimums(LEFT_SIX, RIGHT_SIX, config, threads);
            pack_codes(&left, &mut bytes[LEFT_SIX_START..][..SIX_TILE_TABLE_LEN]);
            pack_codes(&right, &mut bytes[RIGHT_SIX_START..][..SIX_TILE_TABLE_LEN]);
            build_distance_table(
                RIGHT_COLUMN,
                additive,
                &mut bytes[RIGHT_COLUMN_START..][..THREE_TILE_TABLE_LEN],
                config,
                threads,
            );
            for (tiles, start) in [(UPPER_SIX, UPPER_SIX_START), (BOTTOM_SIX, BOTTOM_SIX_START)] {
                pack_codes(
                    &relaxed_minimums(tiles, config, threads),
                    &mut bytes[start..][..SIX_TILE_TABLE_LEN],
                );
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

impl Search for Small {
    /// Each group's index: top row, left six, right six, right column, upper six, bottom six.
    type Indices = [[u32; 6]; 2];
    /// The 6-tile groups' values: left, right, upper, bottom.
    type Values = [[u8; 4]; 2];

    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64) {
        // The 6-tile tables' walks, both orientations.
        let walks = std::array::from_fn::<_, 8, _>(|walk| {
            let group = SIX_TILE_GROUPS[walk % 4];
            WalkSpec {
                tiles: GROUPS[group],
                partition: group / 3,
                board: &boards[walk / 4],
                encoding: Encoding::RelaxedMinimum,
            }
        });
        let start = |walk: usize| STARTS[SIX_TILE_GROUPS[walk % 4]];
        let exact = exact_values(
            &INDEX_SHIFTS,
            walks,
            |walk, state| self.tables.code(start(walk), state.index),
            |walk, state| self.tables.address(start(walk) + state.index as usize / 4),
        );
        let indices = boards.map(|board| GROUPS.map(|tiles| placement_index(tiles, &board)));
        let values = [0, 1].map(|side| std::array::from_fn(|value| exact[4 * side + value]));
        let mut bound = 0;
        for (side, board) in boards.iter().enumerate() {
            let blank = blank_position(board);
            for partition in 0..2 {
                let group = 3 * partition;
                let three = self
                    .tables
                    .byte(STARTS[group] + 16 * indices[side][group] as usize + blank);
                bound = bound
                    .max(three + values[side][2 * partition] + values[side][2 * partition + 1]);
            }
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
            let [top_row, left, right, right_column, upper, bottom] =
                indices[side].map(|index| index as usize);
            let blank = usize::from(tile_move.blank_after[side]);
            prefetch(self.tables.address(TOP_ROW_START + 16 * top_row + blank));
            prefetch(self.tables.address(LEFT_SIX_START + left / 4));
            prefetch(self.tables.address(RIGHT_SIX_START + right / 4));
            prefetch(
                self.tables
                    .address(RIGHT_COLUMN_START + 16 * right_column + blank),
            );
            prefetch(self.tables.address(UPPER_SIX_START + upper / 4));
            prefetch(self.tables.address(BOTTOM_SIX_START + bottom / 4));
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
        let mut key = SortKey::default();
        let decode = |group: usize, index: u32, moved: u8, value: u8| {
            let code = self.tables.code(STARTS[group], index);
            decode_code(code, in_group(GROUP_MASKS[group], moved), value)
        };
        // One partition in both orientations, then the other, stopping at the first that prunes.
        for side in 0..2 {
            let [top_row, left_index, right_index, ..] = child[side];
            let [left, right, ..] = &mut values[side];
            *left = decode(1, left_index, tiles[side], *left);
            *right = decode(2, right_index, tiles[side], *right);
            let blank = usize::from(blanks[side]);
            let top_row = self
                .tables
                .byte(TOP_ROW_START + 16 * top_row as usize + blank);
            key.add(u64::from(top_row + *left + *right), depth)?;
        }
        for side in 0..2 {
            let [.., right_column, upper_index, bottom_index] = child[side];
            let [.., upper, bottom] = &mut values[side];
            *upper = decode(4, upper_index, tiles[side], *upper);
            *bottom = decode(5, bottom_index, tiles[side], *bottom);
            let blank = usize::from(blanks[side]);
            let right_column = self
                .tables
                .byte(RIGHT_COLUMN_START + 16 * right_column as usize + blank);
            key.add(u64::from(right_column + *upper + *bottom), depth)?;
        }
        Some((key.key(), values))
    }
}

impl AsRef<[u8]> for Small {
    fn as_ref(&self) -> &[u8] {
        self.tables.bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb() {
        super::super::tests::check(Small::new(&PdbConfig::default()), HASH);
    }

    #[test]
    fn test_pdb_single_threaded() {
        let pdb = Small::new_with_thread_count(&PdbConfig::default(), NonZeroUsize::MIN);
        assert_eq!(xxhash_rust::xxh3::xxh3_64(pdb.as_ref()), HASH);
    }
}
