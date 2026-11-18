//! Indexing the placements of a group of tiles, and updating the index after a move.
//!
//! A group's placement is the cells of its tiles, in the group's order, and its index is the
//! placement's Lehmer code: tile `i`'s digit is its cell minus the number of earlier tiles on
//! lower cells, and the digits are read in a mixed radix (`16`, `15`, ...).

use crate::solver::size4x4::stm::utils::{Board, Move};

/// Number of placements of `k` tiles.
#[must_use]
pub const fn placements(k: usize) -> usize {
    let mut n = 1;
    let mut i = 0;
    while i < k {
        n *= 16 - i;
        i += 1;
    }
    n
}

/// Mask of `tiles`.
#[must_use]
pub const fn mask(tiles: &[u8]) -> u16 {
    let mut m = 0;
    let mut i = 0;
    while i < tiles.len() {
        m |= 1 << tiles[i];
        i += 1;
    }
    m
}

/// Index of the placement with the group's tiles on `cells`.
#[must_use]
pub fn lehmer(cells: &[u8]) -> u32 {
    let mut used = 0u16;
    let mut index = 0;
    for (i, &cell) in cells.iter().enumerate() {
        let digit = u32::from(cell) - (used & ((1 << cell) - 1)).count_ones();
        index = index * (16 - i as u32) + digit;
        used |= 1 << cell;
    }
    index
}

/// The cells of placement `index` of `k` tiles.
#[must_use]
pub fn unlehmer(mut index: u32, k: usize) -> [u8; 16] {
    let mut digits = [0u8; 16];
    for i in (0..k).rev() {
        let radix = 16 - i as u32;
        digits[i] = (index % radix) as u8;
        index /= radix;
    }
    let mut cells = [0u8; 16];
    let mut used = 0u16;
    for i in 0..k {
        let mut free = !used;
        for _ in 0..digits[i] {
            free &= free - 1;
        }
        cells[i] = free.trailing_zeros() as u8;
        used |= 1 << cells[i];
    }
    cells
}

/// Index of the placement of `tiles` on `board`.
#[must_use]
pub fn placement(tiles: &[u8], board: &Board) -> u32 {
    let mut cells = [0u8; 16];
    for (i, &tile) in tiles.iter().enumerate() {
        cells[i] = board.iter().position(|&t| t == tile).unwrap() as u8;
    }
    lehmer(&cells[..tiles.len()])
}

/// How moves shift the indices of the groups of up to two partitions of the tiles: partition 0's
/// in the low 32 bits of each entry, partition 1's in the high 32 bits.
///
/// When tile `t` moves past the cells between its old and new cell (in label order), the index
/// of its group changes by `±(SHIFT[t][0] + SHIFT[t][u] for each tile u passed)`: added when the
/// tile moves to a higher cell. Tile `t`'s own digit changes by one for the move and one for
/// each tile passed that is not an earlier tile of its group, and each later tile of its group
/// passed changes its digit by one the opposite way.
#[must_use]
pub const fn shift_table(partitions: [&[&[u8]]; 2]) -> [[u64; 16]; 16] {
    let mut out = [[0u64; 16]; 16];
    let mut p = 0;
    while p < 2 {
        let mut g = 0;
        while g < partitions[p].len() {
            let tiles = partitions[p][g];
            let k = tiles.len();
            // Digit weights: the product of the later radices.
            let mut weight = [0u64; 16];
            let mut w = 1;
            let mut i = k;
            while i > 0 {
                i -= 1;
                weight[tiles[i] as usize] = w;
                w *= 16 - i as u64;
            }
            let mut i = 0;
            while i < k {
                let t = tiles[i] as usize;
                let mut u = 0;
                while u < 16 {
                    let mut j = 0;
                    while j < k && tiles[j] as usize != u {
                        j += 1;
                    }
                    let shift = if j == k {
                        // Not in the group (or the blank, for the move itself).
                        weight[t]
                    } else if j > i {
                        weight[t] + weight[u]
                    } else {
                        0
                    };
                    out[t][u] |= shift << (32 * p);
                    u += 1;
                }
                i += 1;
            }
            g += 1;
        }
        p += 1;
    }
    out
}

/// The index shifts (see [`shift_table`]) of both partitions for `m` on `board` (`side` of the
/// move), which moves `tile`: partition 0's in the low 32 bits, partition 1's in the high ones,
/// each to be added with wrapping.
#[inline(always)]
#[must_use]
pub fn shift(table: &[[u64; 16]; 16], board: &Board, m: &Move, side: usize, tile: u8) -> [u32; 2] {
    // SAFETY: tiles are below 16.
    let row = unsafe { table.get_unchecked(usize::from(tile)) };
    let lo = usize::from(m.lo[side]);
    let mut sum = row[0];
    for cell in lo + 1..lo + usize::from(m.gap[side]) {
        // SAFETY: the cells between two cells of the board are cells, holding tiles below 16.
        sum += unsafe { row.get_unchecked(usize::from(*board.get_unchecked(cell))) };
    }
    // The tile moves to the lower cell when the blank moves to the higher one.
    let negate = u32::from(m.sign[side]).wrapping_neg();
    let lane = |x: u64| (x as u32 ^ negate).wrapping_sub(negate);
    [lane(sum), lane(sum >> 32)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::size4x4::stm::utils::{MOVES, MIRROR, random_boards, reflect};

    #[test]
    fn lehmer_round_trips() {
        for k in [1, 3, 6, 9] {
            for index in [0, 1, placements(k) as u32 / 3, placements(k) as u32 - 1] {
                let cells = unlehmer(index, k);
                assert_eq!(lehmer(&cells[..k]), index);
            }
        }
        // Every tile home.
        let home: [u8; 8] = std::array::from_fn(|i| i as u8 + 8);
        let board: Board = std::array::from_fn(|i| i as u8);
        assert_eq!(placement(&home, &board), lehmer(&home));
    }

    /// The shifts match recomputing the indices, for every move of random boards.
    #[test]
    fn shift_matches_recompute() {
        const P0: &[&[u8]] = &[&[1, 2, 3, 4, 5, 6, 7], &[8, 9, 10, 11, 12, 13, 14, 15]];
        const P1: &[&[u8]] = &[&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[10, 11, 12, 13, 14, 15]];
        const TABLE: [[u64; 16]; 16] = shift_table([P0, P1]);
        for board in random_boards(200) {
            let boards = [board, reflect(&board)];
            let blank = board.iter().position(|&t| t == 0).unwrap();
            for m in &MOVES[blank][15].moves[..usize::from(MOVES[blank][15].n)] {
                for (side, board) in boards.iter().enumerate() {
                    let here = if side == 0 { blank } else { usize::from(MIRROR[blank]) };
                    let next = usize::from(m.next[side]);
                    let tile = board[next];
                    let mut moved = *board;
                    moved[here] = tile;
                    moved[next] = 0;
                    let delta = shift(&TABLE, board, m, side, tile);
                    for (p, groups) in [P0, P1].iter().enumerate() {
                        for tiles in groups.iter().filter(|g| g.contains(&tile)) {
                            let before = placement(tiles, board);
                            assert_eq!(before.wrapping_add(delta[p]), placement(tiles, &moved));
                        }
                    }
                }
            }
        }
    }
}
