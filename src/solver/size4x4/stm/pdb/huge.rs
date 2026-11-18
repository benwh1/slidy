//! The huge pattern database (17.0 GB): the maximum of two 9-6 partitions.
//!
//! Each 6-tile table holds a `u16` per placement and blank cell: the group's additive distance in
//! the low byte, its full distance (counting every move) in the high byte. Each 9-tile table
//! holds a `u16` per placement, indexed by the blank's rank `r` among the 7 cells without a group
//! tile: bit `r` is `(value >> 1) & 1` of the additive distance, bit `8 + r` that of the full
//! distance. A full distance bounds the solution length on its own. Builds in about an hour,
//! using up to 25 GB.

use std::{hint::select_unpredictable, sync::atomic::Ordering::Relaxed};

use xxhash_rust::xxh3;

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::{
        pdb::{Pdb, Search},
        utils::{
            Board, Move,
            build::{Cost, as_atomic, breadth_first_search},
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

/// The groups, in the order of [`Huge`]'s indices.
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

/// The length of the database in bytes.
const LEN: usize = LAST_NINE_START + NINE_TILE_TABLE_LEN;

/// The [`xxh3`] hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0x03a3_0ec1_0688_1f45;

/// The huge pattern database (17.0 GB): the maximum of two 9-6 partitions. Builds in about 40
/// minutes on 6 cores, using up to 25 GB.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Huge {
    bytes: Box<[u8]>,
}

/// `((value + 1) >> 1) & 1`: the bit a 9-tile table holds for `value + 1`. A move changes a value
/// by exactly 1, so a child's bit matches this exactly when its value went up.
#[inline(always)]
const fn bit_if_increased(value: u32) -> u32 {
    ((value + 1) >> 1) & 1
}

/// The blank's rank among the cells of `board` without a tile of `group_mask`.
fn blank_rank(board: &Board, group_mask: u16) -> u8 {
    let blank = board.iter().position(|&tile| tile == 0).unwrap();
    board[..blank]
        .iter()
        .filter(|&&tile| (group_mask >> tile) & 1 == 0)
        .count() as u8
}

impl Huge {
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

    /// The additive and full values of placement `index` and blank cell `blank` in the 6-tile
    /// table at `start`.
    #[inline(always)]
    fn six_tile_values(&self, start: usize, index: u32, blank: usize) -> [u8; 2] {
        let offset = start + 32 * index as usize + 2 * blank;
        [self.byte(offset), self.byte(offset + 1)]
    }

    /// The bits of placement `index` in the 9-tile table at `start`, from blank rank `rank`'s on.
    #[inline(always)]
    fn nine_tile_bits(&self, start: usize, index: u32, rank: u32) -> u32 {
        let offset = start + 2 * index as usize;
        debug_assert!(offset + 2 <= LEN);
        // SAFETY: as in `byte`.
        let half = unsafe {
            self.bytes
                .as_ptr()
                .add(offset)
                .cast::<u16>()
                .read_unaligned()
        };
        u32::from(half) >> rank
    }

    /// A child's additive and full values in the 9-tile table at `start` of the tiles in
    /// `group_mask`, from its index, the blank's rank and the moved tile, decoded against its
    /// parent's.
    #[inline(always)]
    fn decode_nine_tile(
        &self,
        start: usize,
        group_mask: u16,
        index: u32,
        rank: u32,
        moved: u8,
        [additive, full]: [u8; 2],
    ) -> [u8; 2] {
        // Each value goes up when its bit matches, else down; the additive one only changes
        // when the moved tile is in the group.
        let (additive, full) = (u32::from(additive), u32::from(full));
        let expected = bit_if_increased(additive) | (bit_if_increased(full) << 8);
        let went_down = self.nine_tile_bits(start, index, rank) ^ expected;
        let child_additive = (additive + 1).wrapping_sub((went_down << 1) & 2);
        let child_full = (full + 1).wrapping_sub((went_down >> 7) & 2);
        [
            select_unpredictable((group_mask >> moved) & 1 != 0, child_additive, additive) as u8,
            child_full as u8,
        ]
    }
}

/// Builds the 6-tile table of `tiles` into `out`.
fn build_six_tile_table(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = as_atomic(out);
    for (cost, cost_byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        breadth_first_search(tiles, cost, config, |index, blank, _, distance| {
            out[32 * index as usize + 2 * blank as usize + cost_byte].store(distance, Relaxed);
        });
    }
}

/// Builds the 9-tile table of `tiles` into `out`.
fn build_nine_tile_table(tiles: &[u8], out: &mut [u8], config: &PdbConfig) {
    let out = as_atomic(out);
    for (cost, cost_byte) in [(Cost::Additive, 0), (Cost::Full, 1)] {
        breadth_first_search(tiles, cost, config, |index, _, rank, distance| {
            // The bytes start zeroed, so only set bits need writing.
            if (distance >> 1) & 1 != 0 {
                out[2 * index as usize + cost_byte].fetch_or(1 << rank, Relaxed);
            }
        });
    }
}

impl Pdb for Huge {
    fn new(config: &PdbConfig) -> Self {
        let mut bytes = vec![0; LEN].into_boxed_slice();
        build_six_tile_table(
            LAST_SIX,
            &mut bytes[LAST_SIX_START..FIRST_SIX_START],
            config,
        );
        build_six_tile_table(
            FIRST_SIX,
            &mut bytes[FIRST_SIX_START..FIRST_NINE_START],
            config,
        );
        build_nine_tile_table(
            FIRST_NINE,
            &mut bytes[FIRST_NINE_START..LAST_NINE_START],
            config,
        );
        build_nine_tile_table(LAST_NINE, &mut bytes[LAST_NINE_START..], config);
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
        // The 9-tile tables' walks, both orientations: (tiles, partition, encoding), and where
        // each reads its bits (table start, bit offset).
        const WALKS: [(&[u8], usize, Encoding, usize, u32); 4] = [
            (FIRST_NINE, 0, Encoding::AdditiveBit, FIRST_NINE_START, 0),
            (FIRST_NINE, 0, Encoding::FullBit, FIRST_NINE_START, 8),
            (LAST_NINE, 1, Encoding::AdditiveBit, LAST_NINE_START, 0),
            (LAST_NINE, 1, Encoding::FullBit, LAST_NINE_START, 8),
        ];
        let walks = std::array::from_fn::<_, 8, _>(|walk| {
            let (tiles, partition, encoding, ..) = WALKS[walk % 4];
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
                let (.., start, bit) = WALKS[walk % 4];
                let rank = (state.free_cells & ((1 << state.blank) - 1)).count_ones();
                (self.nine_tile_bits(start, state.index, rank) >> bit) & 1
            },
            |walk, state| self.address_of(WALKS[walk % 4].3 + 2 * state.index as usize),
        );
        let [_, first_nine_mask, _, last_nine_mask] = GROUP_MASKS;
        let mut indices = Indices {
            groups: [[0; 4]; 2],
            ranks: [[0; 2]; 2],
        };
        let mut values = [[0; 4]; 2];
        let mut bound = 0;
        for (side, board) in boards.iter().enumerate() {
            let blank = board.iter().position(|&tile| tile == 0).unwrap();
            let [last_six, first_nine, first_six, last_nine] =
                GROUPS.map(|tiles| placement_index(tiles, board));
            indices.groups[side] = [last_six, first_nine, first_six, last_nine];
            indices.ranks[side] = [
                blank_rank(board, first_nine_mask),
                blank_rank(board, last_nine_mask),
            ];
            values[side] = std::array::from_fn(|walk| exact[4 * side + walk]);
            let [
                first_nine_additive,
                first_nine_full,
                last_nine_additive,
                last_nine_full,
            ] = values[side];
            let [last_six_additive, last_six_full] =
                self.six_tile_values(LAST_SIX_START, last_six, blank);
            let [first_six_additive, first_six_full] =
                self.six_tile_values(FIRST_SIX_START, first_six, blank);
            bound = bound
                .max(last_six_additive + first_nine_additive)
                .max(last_six_full)
                .max(first_nine_full)
                .max(first_six_additive + last_nine_additive)
                .max(first_six_full)
                .max(last_nine_full);
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
            let (moved, board) = (tiles[side], &boards[side]);
            let shifts = move_shifts(
                &INDEX_SHIFTS,
                board,
                tile_move,
                side,
                moved,
                [first_nine_mask, last_nine_mask],
            );
            let groups = &mut indices.groups[side];
            for (group, mask) in GROUP_MASKS.into_iter().enumerate() {
                let in_group = (mask >> moved) & 1 != 0;
                let delta = select_unpredictable(in_group, shifts.indices[group / 2], 0);
                groups[group] = groups[group].wrapping_add(delta);
            }
            for (rank, change) in indices.ranks[side].iter_mut().zip(shifts.ranks) {
                *rank = rank.wrapping_add(change as u8);
            }
            let [last_six, first_nine, first_six, last_nine] =
                indices.groups[side].map(|index| index as usize);
            let blank = usize::from(tile_move.blank_after[side]);
            prefetch(self.address_of(LAST_SIX_START + 32 * last_six + 2 * blank));
            prefetch(self.address_of(FIRST_NINE_START + 2 * first_nine));
            prefetch(self.address_of(FIRST_SIX_START + 32 * first_six + 2 * blank));
            prefetch(self.address_of(LAST_NINE_START + 2 * last_nine));
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
        let (mut best, mut total) = (0, 0);
        // One partition in both orientations, then the other, stopping at the first that prunes.
        for side in 0..2 {
            let [last_six_index, first_nine_index, ..] = child.groups[side];
            let first_nine_rank = u32::from(child.ranks[side][0]);
            let [first_nine_additive, first_nine_full, ..] = &mut values[side];
            [*first_nine_additive, *first_nine_full] = self.decode_nine_tile(
                FIRST_NINE_START,
                first_nine_mask,
                first_nine_index,
                first_nine_rank,
                tiles[side],
                [*first_nine_additive, *first_nine_full],
            );
            let blank = usize::from(blanks[side]);
            let [last_six_additive, last_six_full] =
                self.six_tile_values(LAST_SIX_START, last_six_index, blank);
            let estimate = u64::from(
                (last_six_additive + *first_nine_additive)
                    .max(last_six_full)
                    .max(*first_nine_full),
            );
            if estimate >= depth {
                return None;
            }
            best = best.max(estimate);
            total += estimate;
        }
        for side in 0..2 {
            let [_, _, first_six_index, last_nine_index] = child.groups[side];
            let last_nine_rank = u32::from(child.ranks[side][1]);
            let [.., last_nine_additive, last_nine_full] = &mut values[side];
            [*last_nine_additive, *last_nine_full] = self.decode_nine_tile(
                LAST_NINE_START,
                last_nine_mask,
                last_nine_index,
                last_nine_rank,
                tiles[side],
                [*last_nine_additive, *last_nine_full],
            );
            let blank = usize::from(blanks[side]);
            let [first_six_additive, first_six_full] =
                self.six_tile_values(FIRST_SIX_START, first_six_index, blank);
            let estimate = u64::from(
                (first_six_additive + *last_nine_additive)
                    .max(first_six_full)
                    .max(*last_nine_full),
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

impl AsRef<[u8]> for Huge {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
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
