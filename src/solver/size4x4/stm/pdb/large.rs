//! The large pattern database (1.27 GB): a single 7-8 partition.
//!
//! Both tables hold, for each placement and blank cell, `(value >> 1) & 1` of the group's
//! additive distance and that of its full distance (counting every move), which bounds the
//! solution length on its own. The 7-tile table holds a `u32` per placement: bit `2b` is the
//! additive bit with the blank on cell `b`, bit `2b + 1` the full one. The 8-tile table holds a
//! `u16` per placement, indexed by the blank's rank `r` among the 8 cells without a group tile:
//! bit `r` is the additive bit, bit `8 + r` the full one. Builds in about ten minutes.

use std::{hint::select_unpredictable, num::NonZeroUsize};

use crate::solver::{
    config::PdbConfig,
    size4x4::stm::{
        pdb::{
            Pdb, Search, SortKey,
            tables::{
                Tables, blank_rank, build_cell_bit_table, build_rank_bit_table, decode_bits,
                in_group,
            },
        },
        utils::{
            Board, Move,
            index::{index_shift_table, move_shifts, placement_count, placement_index, tile_mask},
            prefetch,
            walk::{Encoding, WalkSpec, exact_values},
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

/// The [`xxh3`](xxhash_rust::xxh3) hash of the database, checked by [`Pdb::try_from_bytes`].
const HASH: u64 = 0x67d0_6ca4_066a_b09c;

/// The large pattern database (1.27 GB): a single 7-8 partition. Builds in about ten
/// minutes.
///
/// See [`Pdb`] for building it, and loading and saving its bytes.
pub struct Large {
    tables: Tables<LEN, HASH>,
}

impl Large {
    /// The 7-tile table's bits for placement `index`, from blank cell `blank`'s on.
    #[inline(always)]
    fn seven_tile_bits(&self, index: u32, blank: usize) -> u32 {
        self.tables.u32(TOP_SEVEN_START + 4 * index as usize) >> (2 * blank)
    }

    /// The 8-tile table's bits for placement `index`, from blank rank `rank`'s on.
    #[inline(always)]
    fn eight_tile_bits(&self, index: u32, rank: u32) -> u32 {
        u32::from(self.tables.u16(BOTTOM_EIGHT_START + 2 * index as usize)) >> rank
    }
}

impl Pdb for Large {
    fn new_with_thread_count(config: &PdbConfig, threads: NonZeroUsize) -> Self {
        let threads = threads.get();
        let tables = Tables::build(|bytes| {
            let (top_seven, bottom_eight) = bytes.split_at_mut(BOTTOM_EIGHT_START);
            build_cell_bit_table(TOP_SEVEN, top_seven, config, threads);
            build_rank_bit_table(BOTTOM_EIGHT, bottom_eight, config, threads);
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

impl Search for Large {
    /// Each group's index (top, bottom), and the blank's rank among the 8-tile group's free
    /// cells.
    type Indices = [[u32; 3]; 2];
    /// Each group's additive and full values (top, then bottom).
    type Values = [[u8; 4]; 2];

    fn root(&self, boards: &[Board; 2]) -> (Self::Indices, Self::Values, u64) {
        // The tables' walks, both orientations: (tiles, encoding, bit offset), top seven first.
        const WALKS: [(&[u8], Encoding, u32); 4] = [
            (TOP_SEVEN, Encoding::AdditiveBit, 0),
            (TOP_SEVEN, Encoding::FullBit, 1),
            (BOTTOM_EIGHT, Encoding::AdditiveBit, 0),
            (BOTTOM_EIGHT, Encoding::FullBit, 8),
        ];
        let walks = std::array::from_fn::<_, 8, _>(|walk| {
            let (tiles, encoding, _) = WALKS[walk % 4];
            WalkSpec {
                tiles,
                partition: 0,
                board: &boards[walk / 4],
                encoding,
            }
        });
        let exact = exact_values(
            &INDEX_SHIFTS,
            walks,
            |walk, state| {
                let bits = if walk % 4 < 2 {
                    self.seven_tile_bits(state.index, state.blank as usize)
                } else {
                    let rank = (state.free_cells & ((1 << state.blank) - 1)).count_ones();
                    self.eight_tile_bits(state.index, rank)
                };
                (bits >> WALKS[walk % 4].2) & 1
            },
            |walk, state| {
                self.tables.address(if walk % 4 < 2 {
                    TOP_SEVEN_START + 4 * state.index as usize
                } else {
                    BOTTOM_EIGHT_START + 2 * state.index as usize
                })
            },
        );
        let indices = boards.map(|board| {
            [
                placement_index(TOP_SEVEN, &board),
                placement_index(BOTTOM_EIGHT, &board),
                blank_rank(&board, BOTTOM_EIGHT_MASK),
            ]
        });
        let values = [0, 1].map(|side| std::array::from_fn(|value| exact[4 * side + value]));
        let mut bound = 0;
        for [top_additive, top_full, bottom_additive, bottom_full] in values {
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
            let shifts = move_shifts(
                &INDEX_SHIFTS,
                board,
                tile_move,
                side,
                moved,
                [BOTTOM_EIGHT_MASK],
            );
            let [delta, _] = shifts.indices;
            let [top, bottom, rank] = &mut indices[side];
            let top_moved = in_group(TOP_SEVEN_MASK, moved);
            *top = top.wrapping_add(select_unpredictable(top_moved, delta, 0));
            let bottom_moved = in_group(BOTTOM_EIGHT_MASK, moved);
            *bottom = bottom.wrapping_add(select_unpredictable(bottom_moved, delta, 0));
            *rank = rank.wrapping_add(shifts.ranks[0]);
            prefetch(self.tables.address(TOP_SEVEN_START + 4 * *top as usize));
            prefetch(
                self.tables
                    .address(BOTTOM_EIGHT_START + 2 * *bottom as usize),
            );
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
        for side in 0..2 {
            let [top_index, bottom_index, rank] = child[side];
            let [top_additive, top_full, bottom_additive, bottom_full] = &mut values[side];
            let moved = tiles[side];
            [*top_additive, *top_full] = decode_bits(
                self.seven_tile_bits(top_index, usize::from(blanks[side])),
                1,
                in_group(TOP_SEVEN_MASK, moved),
                [*top_additive, *top_full],
            );
            [*bottom_additive, *bottom_full] = decode_bits(
                self.eight_tile_bits(bottom_index, rank),
                8,
                in_group(BOTTOM_EIGHT_MASK, moved),
                [*bottom_additive, *bottom_full],
            );
            let estimate = (*top_additive + *bottom_additive)
                .max(*top_full)
                .max(*bottom_full);
            key.add(u64::from(estimate), depth)?;
        }
        Some((key.key(), values))
    }
}

impl AsRef<[u8]> for Large {
    fn as_ref(&self) -> &[u8] {
        self.tables.bytes()
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
