//! The huge pattern database (17.0 GB): the maximum of two 9-6 partitions.
//!
//! Each 6-tile table holds a `u16` per placement and blank cell: the group's additive distance in
//! the low byte, its full distance (counting every move) in the high byte. Each 9-tile table
//! holds a `u16` per placement, indexed by the blank's rank `r` among the 7 cells without a group
//! tile: bit `r` is `(value >> 1) & 1` of the additive distance, bit `8 + r` that of the full
//! distance. A full distance bounds the solution length on its own. Builds in about 40 minutes on
//! 6 cores, using up to 25 GB.

use std::{hint::select_unpredictable, num::NonZeroUsize};

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::{
        pdb::{
            Pdb, Search, SortKey,
            tables::{
                Tables, blank_rank, build_distance_table, build_rank_bit_table, decode_bits,
                in_group,
            },
        },
        utils::{
            Board, Move, blank_position,
            build::Cost,
            index::{index_shift_table, move_shifts, placement_count, placement_index, tile_mask},
            prefetch,
            walk::{Encoding, WalkSpec, exact_values},
        },
    },
};

// The first partition: the bottom row and a half and the rest; the second: the top row and a
// half and the rest.
const LAST_SIX: &[u8] = &[10, 11, 12, 13, 14, 15];
const FIRST_NINE: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
const FIRST_SIX: &[u8] = &[1, 2, 3, 4, 5, 6];
const LAST_NINE: &[u8] = &[7, 8, 9, 10, 11, 12, 13, 14, 15];

/// The groups, in the order of [`Huge`]'s indices: each partition's 6-tile group, then its 9-tile
/// one.
const GROUPS: [&[u8]; 4] = [LAST_SIX, FIRST_NINE, FIRST_SIX, LAST_NINE];

/// The groups' tiles as masks, in [`GROUPS`] order.
const GROUP_MASKS: [u16; 4] = [
    tile_mask(LAST_SIX),
    tile_mask(FIRST_NINE),
    tile_mask(FIRST_SIX),
    tile_mask(LAST_NINE),
];

/// How moves shift the groups' indices.
const INDEX_SHIFTS: [[u64; 16]; 16] =
    index_shift_table([&[LAST_SIX, FIRST_NINE], &[FIRST_SIX, LAST_NINE]]);

/// The size of a 6-tile table: a `u16` per placement and blank cell.
const SIX_TILE_TABLE_LEN: usize = placement_count(6) * 32;

/// The size of a 9-tile table: a `u16` per placement.
const NINE_TILE_TABLE_LEN: usize = placement_count(9) * 2;

// Where each table starts.
const LAST_SIX_START: usize = 0;
const FIRST_SIX_START: usize = LAST_SIX_START + SIX_TILE_TABLE_LEN;
const FIRST_NINE_START: usize = FIRST_SIX_START + SIX_TILE_TABLE_LEN;
const LAST_NINE_START: usize = FIRST_NINE_START + NINE_TILE_TABLE_LEN;

/// Where each partition's 6-tile and 9-tile tables start.
const SIX_TILE_STARTS: [usize; 2] = [LAST_SIX_START, FIRST_SIX_START];
const NINE_TILE_STARTS: [usize; 2] = [FIRST_NINE_START, LAST_NINE_START];

/// The length of the database in bytes.
const LEN: usize = LAST_NINE_START + NINE_TILE_TABLE_LEN;

/// The [`xxh3`](xxhash_rust::xxh3) hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0x03a3_0ec1_0688_1f45;

/// The huge pattern database (17.0 GB): the maximum of two 9-6 partitions. Builds in about 40
/// minutes on 6 cores, using up to 25 GB.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Huge {
    tables: Tables<LEN, HASH>,
}

impl Huge {
    /// The additive and full values of placement `index` and blank cell `blank` in partition
    /// `partition`'s 6-tile table.
    #[inline(always)]
    fn six_tile_values(&self, partition: usize, index: u32, blank: usize) -> [u8; 2] {
        let offset = SIX_TILE_STARTS[partition] + 32 * index as usize + 2 * blank;
        [self.tables.byte(offset), self.tables.byte(offset + 1)]
    }

    /// The bits of placement `index` in partition `partition`'s 9-tile table, from blank rank
    /// `rank`'s on.
    #[inline(always)]
    fn nine_tile_bits(&self, partition: usize, index: u32, rank: u32) -> u32 {
        let offset = NINE_TILE_STARTS[partition] + 2 * index as usize;
        u32::from(self.tables.u16(offset)) >> rank
    }
}

impl Pdb for Huge {
    fn new_with_thread_count(config: &PdbConfig, threads: NonZeroUsize) -> Self {
        let threads = threads.get();
        let tables = Tables::build(|bytes| {
            let both = &[Cost::Additive, Cost::Full];
            for (tiles, start) in [(LAST_SIX, LAST_SIX_START), (FIRST_SIX, FIRST_SIX_START)] {
                let table = &mut bytes[start..][..SIX_TILE_TABLE_LEN];
                build_distance_table(tiles, both, table, config, threads);
            }
            for (tiles, start) in [(FIRST_NINE, FIRST_NINE_START), (LAST_NINE, LAST_NINE_START)] {
                let table = &mut bytes[start..][..NINE_TILE_TABLE_LEN];
                build_rank_bit_table(tiles, table, config, threads);
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

/// A node's indices in both orientations, kept small: the search stores them for every
/// successor.
#[derive(Clone, Copy)]
pub struct Indices {
    /// Each group's index (last six, first nine, first six, last nine).
    groups: [[u32; 4]; 2],
    /// The blank's rank among the free cells of the first and the last nine.
    ranks: [[u8; 2]; 2],
}

impl Search for Huge {
    type Indices = Indices;
    /// The 9-tile groups' additive and full values (first nine, then last nine).
    type Values = [[u8; 4]; 2];

    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64) {
        // The 9-tile tables' walks, both orientations: (tiles, partition, encoding, bit offset).
        const WALKS: [(&[u8], usize, Encoding, u32); 4] = [
            (FIRST_NINE, 0, Encoding::AdditiveBit, 0),
            (FIRST_NINE, 0, Encoding::FullBit, 8),
            (LAST_NINE, 1, Encoding::AdditiveBit, 0),
            (LAST_NINE, 1, Encoding::FullBit, 8),
        ];
        let walks = std::array::from_fn::<_, 8, _>(|walk| {
            let (tiles, partition, encoding, _) = WALKS[walk % 4];
            WalkSpec {
                tiles,
                partition,
                board: &boards[walk / 4],
                encoding,
            }
        });
        let exact = exact_values(
            &INDEX_SHIFTS,
            walks,
            |walk, state| {
                let (_, partition, _, bit) = WALKS[walk % 4];
                let rank = (state.free_cells & ((1 << state.blank) - 1)).count_ones();
                (self.nine_tile_bits(partition, state.index, rank) >> bit) & 1
            },
            |walk, state| {
                let partition = WALKS[walk % 4].1;
                self.tables
                    .address(NINE_TILE_STARTS[partition] + 2 * state.index as usize)
            },
        );
        let [_, first_nine_mask, _, last_nine_mask] = GROUP_MASKS;
        let indices = Indices {
            groups: boards.map(|board| GROUPS.map(|tiles| placement_index(tiles, &board))),
            ranks: boards.map(|board| {
                [first_nine_mask, last_nine_mask].map(|mask| blank_rank(&board, mask) as u8)
            }),
        };
        let values = [0, 1].map(|side| std::array::from_fn(|value| exact[4 * side + value]));
        let mut bound = 0;
        for (side, board) in boards.iter().enumerate() {
            let blank = blank_position(board);
            for partition in 0..2 {
                let six_index = indices.groups[side][2 * partition];
                let [six_additive, six_full] = self.six_tile_values(partition, six_index, blank);
                let [additive, full] =
                    [values[side][2 * partition], values[side][2 * partition + 1]];
                bound = bound.max(six_additive + additive).max(six_full).max(full);
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
        let [_, first_nine_mask, _, last_nine_mask] = GROUP_MASKS;
        let mut indices = *parent;
        for side in 0..2 {
            let moved = tiles[side];
            let shifts = move_shifts(
                &INDEX_SHIFTS,
                &boards[side],
                tile_move,
                side,
                moved,
                [first_nine_mask, last_nine_mask],
            );
            for (group, index) in indices.groups[side].iter_mut().enumerate() {
                let delta = select_unpredictable(
                    in_group(GROUP_MASKS[group], moved),
                    shifts.indices[group / 2],
                    0,
                );
                *index = index.wrapping_add(delta);
            }
            for (rank, change) in indices.ranks[side].iter_mut().zip(shifts.ranks) {
                *rank = rank.wrapping_add(change as u8);
            }
            let [last_six, first_nine, first_six, last_nine] =
                indices.groups[side].map(|index| index as usize);
            let blank = usize::from(tile_move.blank_after[side]);
            prefetch(
                self.tables
                    .address(LAST_SIX_START + 32 * last_six + 2 * blank),
            );
            prefetch(self.tables.address(FIRST_NINE_START + 2 * first_nine));
            prefetch(
                self.tables
                    .address(FIRST_SIX_START + 32 * first_six + 2 * blank),
            );
            prefetch(self.tables.address(LAST_NINE_START + 2 * last_nine));
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
        let [_, first_nine_mask, _, last_nine_mask] = GROUP_MASKS;
        let mut values = *parent;
        let mut key = SortKey::default();
        // One partition in both orientations, then the other, stopping at the first that prunes.
        for side in 0..2 {
            let [last_six_index, first_nine_index, ..] = child.groups[side];
            let rank = u32::from(child.ranks[side][0]);
            let [additive, full, ..] = &mut values[side];
            [*additive, *full] = decode_bits(
                self.nine_tile_bits(0, first_nine_index, rank),
                8,
                in_group(first_nine_mask, tiles[side]),
                [*additive, *full],
            );
            let [six_additive, six_full] =
                self.six_tile_values(0, last_six_index, usize::from(blanks[side]));
            key.add(
                u64::from((six_additive + *additive).max(six_full).max(*full)),
                depth,
            )?;
        }
        for side in 0..2 {
            let [_, _, first_six_index, last_nine_index] = child.groups[side];
            let rank = u32::from(child.ranks[side][1]);
            let [.., additive, full] = &mut values[side];
            [*additive, *full] = decode_bits(
                self.nine_tile_bits(1, last_nine_index, rank),
                8,
                in_group(last_nine_mask, tiles[side]),
                [*additive, *full],
            );
            let [six_additive, six_full] =
                self.six_tile_values(1, first_six_index, usize::from(blanks[side]));
            key.add(
                u64::from((six_additive + *additive).max(six_full).max(*full)),
                depth,
            )?;
        }
        Some((key.key(), values))
    }
}

impl AsRef<[u8]> for Huge {
    fn as_ref(&self) -> &[u8] {
        self.tables.bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "builds two 9-tile tables: about 40 minutes and 25 GB"]
    fn test_pdb() {
        super::super::tests::check(Huge::new(&PdbConfig::default()), HASH);
    }
}
