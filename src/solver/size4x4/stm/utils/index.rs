//! Indexing the placements of a group of tiles, and updating the index after a move.
//!
//! A group's placement is the cells of its tiles, in the group's order, and its index is the
//! placement's Lehmer code: tile `i`'s digit is its cell minus the number of earlier tiles on
//! lower cells, and the digits are read in a mixed radix (`16`, `15`, ...).

use std::hint::select_unpredictable;

use crate::solver::size4x4::stm::utils::{Board, Move};

/// The number of placements of `tile_count` tiles.
#[must_use]
pub(in crate::solver::size4x4::stm) const fn placement_count(tile_count: usize) -> usize {
    let mut count = 1;
    let mut placed = 0;
    while placed < tile_count {
        count *= 16 - placed;
        placed += 1;
    }
    count
}

/// The mask with bit `tile` set for each of `tiles`.
#[must_use]
pub(in crate::solver::size4x4::stm) const fn tile_mask(tiles: &[u8]) -> u16 {
    let mut mask = 0;
    let mut position = 0;
    while position < tiles.len() {
        mask |= 1 << tiles[position];
        position += 1;
    }
    mask
}

/// The index of the placement with the group's tiles on `cells`.
#[must_use]
pub(super) fn index_of_cells(cells: &[u8]) -> u32 {
    let mut used = 0u16;
    let mut index = 0;
    for (position, &cell) in cells.iter().enumerate() {
        let digit = u32::from(cell) - (used & ((1 << cell) - 1)).count_ones();
        index = index * (16 - position as u32) + digit;
        used |= 1 << cell;
    }
    index
}

/// `SET_BIT_POSITIONS[byte][n]`: the position of set bit `n` (from the lowest) of `byte`, or 8
/// when it has `n` or fewer set bits.
const SET_BIT_POSITIONS: [[u8; 8]; 256] = {
    let mut table = [[8; 8]; 256];
    let mut byte = 0;
    while byte < 256 {
        let mut found = 0;
        let mut bit = 0;
        while bit < 8 {
            if (byte >> bit) & 1 != 0 {
                table[byte][found] = bit as u8;
                found += 1;
            }
            bit += 1;
        }
        byte += 1;
    }
    table
};

/// The position of set bit `n` (from the lowest) of `mask`, which has more than `n`.
#[inline(always)]
fn nth_set_bit(mask: u16, n: u32) -> u32 {
    let low = usize::from(mask as u8);
    let low_count = low.count_ones();
    let in_low = u32::from(SET_BIT_POSITIONS[low][(n & 7) as usize]);
    let high = usize::from(mask >> 8);
    let in_high = 8 + u32::from(SET_BIT_POSITIONS[high][(n.wrapping_sub(low_count) & 7) as usize]);
    select_unpredictable(n < low_count, in_low, in_high)
}

/// The cells of placement `index` of `N` tiles.
#[inline(always)]
fn cells_of_index_with<const N: usize>(mut index: u32) -> [u8; N] {
    // With `N` known, the divisions by the radices are by constants.
    let mut digits = [0; N];
    for position in (0..N).rev() {
        let radix = 16 - position as u32;
        digits[position] = index % radix;
        index /= radix;
    }
    let mut cells = [0; N];
    let mut unused = u16::MAX;
    for (cell, digit) in cells.iter_mut().zip(digits) {
        let position = nth_set_bit(unused, digit);
        *cell = position as u8;
        unused &= !(1 << position);
    }
    cells
}

/// The cells of placement `index` of `tile_count` tiles (from 1 to 9), then zeros.
#[inline(always)]
#[must_use]
pub(super) fn cells_of_index(index: u32, tile_count: usize) -> [u8; 16] {
    fn padded<const N: usize>(cells: [u8; N]) -> [u8; 16] {
        let mut padded = [0; 16];
        padded[..N].copy_from_slice(&cells);
        padded
    }
    match tile_count {
        1 => padded(cells_of_index_with::<1>(index)),
        2 => padded(cells_of_index_with::<2>(index)),
        3 => padded(cells_of_index_with::<3>(index)),
        4 => padded(cells_of_index_with::<4>(index)),
        5 => padded(cells_of_index_with::<5>(index)),
        6 => padded(cells_of_index_with::<6>(index)),
        7 => padded(cells_of_index_with::<7>(index)),
        8 => padded(cells_of_index_with::<8>(index)),
        9 => padded(cells_of_index_with::<9>(index)),
        _ => panic!("groups have 1 to 9 tiles"),
    }
}

/// The index of the placement of `tiles` on `board`.
#[must_use]
pub(in crate::solver::size4x4::stm) fn placement_index(tiles: &[u8], board: &Board) -> u32 {
    let mut cells = [0u8; 16];
    for (cell, &tile) in cells.iter_mut().zip(tiles) {
        *cell = board
            .iter()
            .position(|&cell_tile| cell_tile == tile)
            .unwrap() as u8;
    }
    index_of_cells(&cells[..tiles.len()])
}

/// How moves shift the indices of the groups of up to two partitions of the tiles: partition 0's
/// in the low 32 bits of each entry, partition 1's in the high 32 bits.
///
/// When tile `t` moves past the cells between its old and new cell (in label order), the index
/// of its group changes by `±(table[t][0] + table[t][u] for each tile u passed)`: added when the
/// tile moves to a higher cell. Tile `t`'s own digit changes by one for the move and one for
/// each tile passed that is not an earlier tile of its group, and each later tile of its group
/// passed changes its digit by one the opposite way.
#[must_use]
pub(in crate::solver::size4x4::stm) const fn index_shift_table(
    partitions: [&[&[u8]]; 2],
) -> [[u64; 16]; 16] {
    let mut table = [[0u64; 16]; 16];
    let mut partition = 0;
    while partition < 2 {
        let mut group = 0;
        while group < partitions[partition].len() {
            let tiles = partitions[partition][group];
            let tile_count = tiles.len();
            // Digit weights: the product of the later radices.
            let mut weight = [0u64; 16];
            let mut product = 1;
            let mut position = tile_count;
            while position > 0 {
                position -= 1;
                weight[tiles[position] as usize] = product;
                product *= 16 - position as u64;
            }
            let mut moved_position = 0;
            while moved_position < tile_count {
                let tile = tiles[moved_position] as usize;
                let mut passed = 0;
                while passed < 16 {
                    // The passed tile's position in the group (`tile_count` if not in it).
                    let mut passed_position = 0;
                    while passed_position < tile_count && tiles[passed_position] as usize != passed
                    {
                        passed_position += 1;
                    }
                    let shift = if passed_position == tile_count {
                        // Not in the group (or the blank, for the move itself).
                        weight[tile]
                    } else if passed_position > moved_position {
                        weight[tile] + weight[passed]
                    } else {
                        0
                    };
                    table[tile][passed] |= shift << (32 * partition);
                    passed += 1;
                }
                moved_position += 1;
            }
            group += 1;
        }
        partition += 1;
    }
    table
}

/// The index shifts (see [`index_shift_table`]) of both partitions for `tile_move` on `board`
/// (`side` of the move), which moves `tile`: partition 0's first, each to be added with wrapping.
/// `table` is the partitions' [`index_shift_table`].
#[inline(always)]
#[must_use]
pub(in crate::solver::size4x4::stm) fn index_shifts(
    table: &[[u64; 16]; 16],
    board: &Board,
    tile_move: &Move,
    side: usize,
    tile: u8,
) -> [u32; 2] {
    // SAFETY: tiles are below 16.
    let row = unsafe { table.get_unchecked(usize::from(tile)) };
    let lower = usize::from(tile_move.lower_cell[side]);
    let mut sum = row[0];
    for cell in lower + 1..lower + usize::from(tile_move.cell_gap[side]) {
        // SAFETY: the cells between two cells of the board are cells, holding tiles below 16.
        sum += unsafe { row.get_unchecked(usize::from(*board.get_unchecked(cell))) };
    }
    // The tile moves to the lower cell when the blank moves to the higher one.
    let negate = u32::from(tile_move.blank_to_higher_cell[side]).wrapping_neg();
    let signed = |lane: u64| (lane as u32 ^ negate).wrapping_sub(negate);
    [signed(sum), signed(sum >> 32)]
}

/// The blank's rank among the cells without a tile of `group` (how many of them are below its
/// own cell) after `tile_move` on `board` (`side` of the move), which moves `tile`, from its rank
/// `rank` before the move.
#[inline(always)]
#[must_use]
pub(in crate::solver::size4x4::stm) fn shifted_blank_rank(
    rank: u32,
    board: &Board,
    tile_move: &Move,
    side: usize,
    group: u16,
    tile: u8,
) -> u32 {
    // The cells that change sides of the blank: those it passes (in label order) without a
    // group tile, and the moved tile's own unless it is in the group.
    let lower = usize::from(tile_move.lower_cell[side]);
    let mut passed = u32::from((group >> tile) & 1 == 0);
    for cell in lower + 1..lower + usize::from(tile_move.cell_gap[side]) {
        // SAFETY: the cells between two cells of the board are cells, holding tiles below 16.
        let other = unsafe { *board.get_unchecked(cell) };
        passed += u32::from((group >> other) & 1 == 0);
    }
    // The blank moves to the higher cell when the tile moves to the lower one.
    select_unpredictable(
        tile_move.blank_to_higher_cell[side],
        rank + passed,
        rank.wrapping_sub(passed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::size4x4::stm::utils::{MIRRORED_CELL, MOVES, random_boards, reflect};

    #[test]
    fn nth_set_bit_matches_clearing_lower_bits() {
        for mask in 1..=u16::MAX {
            let mut remaining = mask;
            for n in 0..mask.count_ones() {
                assert_eq!(nth_set_bit(mask, n), remaining.trailing_zeros());
                remaining &= remaining - 1;
            }
        }
    }

    #[test]
    fn indices_round_trip() {
        for tile_count in [1, 3, 6, 9] {
            let last = placement_count(tile_count) as u32 - 1;
            for index in [0, 1, last / 3, last] {
                let cells = cells_of_index(index, tile_count);
                assert_eq!(index_of_cells(&cells[..tile_count]), index);
            }
        }
        // Every tile home.
        let home: [u8; 8] = std::array::from_fn(|position| position as u8 + 8);
        let board: Board = std::array::from_fn(|cell| cell as u8);
        assert_eq!(placement_index(&home, &board), index_of_cells(&home));
    }

    /// The shifts match recomputing the indices, for every move of random boards.
    #[test]
    fn shifts_match_recomputing() {
        const SEVEN_EIGHT: &[&[u8]] = &[&[1, 2, 3, 4, 5, 6, 7], &[8, 9, 10, 11, 12, 13, 14, 15]];
        const NINE_SIX: &[&[u8]] = &[&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[10, 11, 12, 13, 14, 15]];
        const INDEX_SHIFTS: [[u64; 16]; 16] = index_shift_table([SEVEN_EIGHT, NINE_SIX]);
        for board in random_boards(200) {
            let boards = [board, reflect(&board)];
            let blank = board.iter().position(|&tile| tile == 0).unwrap();
            let moves = &MOVES[blank][15];
            for tile_move in &moves.moves[..usize::from(moves.count)] {
                for (side, board) in boards.iter().enumerate() {
                    let blank_cell = if side == 0 {
                        blank
                    } else {
                        usize::from(MIRRORED_CELL[blank])
                    };
                    let blank_after = usize::from(tile_move.blank_after[side]);
                    let tile = board[blank_after];
                    let mut moved = *board;
                    moved[blank_cell] = tile;
                    moved[blank_after] = 0;
                    let delta = index_shifts(&INDEX_SHIFTS, board, tile_move, side, tile);
                    for (partition, groups) in [SEVEN_EIGHT, NINE_SIX].iter().enumerate() {
                        for tiles in groups.iter().filter(|group| group.contains(&tile)) {
                            let before = placement_index(tiles, board);
                            let after = placement_index(tiles, &moved);
                            assert_eq!(before.wrapping_add(delta[partition]), after);
                        }
                    }
                }
            }
        }
    }

    /// The rank shifts match recounting, for every move of random boards.
    #[test]
    fn shifted_ranks_match_recounting() {
        let group_mask = tile_mask(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let rank = |board: &Board| {
            let blank = board.iter().position(|&tile| tile == 0).unwrap();
            board[..blank]
                .iter()
                .filter(|&&tile| (group_mask >> tile) & 1 == 0)
                .count() as u32
        };
        for board in random_boards(200) {
            let boards = [board, reflect(&board)];
            let blank = board.iter().position(|&tile| tile == 0).unwrap();
            let moves = &MOVES[blank][15];
            for tile_move in &moves.moves[..usize::from(moves.count)] {
                for (side, board) in boards.iter().enumerate() {
                    let blank_cell = if side == 0 {
                        blank
                    } else {
                        usize::from(MIRRORED_CELL[blank])
                    };
                    let blank_after = usize::from(tile_move.blank_after[side]);
                    let tile = board[blank_after];
                    let mut moved = *board;
                    moved[blank_cell] = tile;
                    moved[blank_after] = 0;
                    let shifted =
                        shifted_blank_rank(rank(board), board, tile_move, side, group_mask, tile);
                    assert_eq!(shifted, rank(&moved));
                }
            }
        }
    }
}
