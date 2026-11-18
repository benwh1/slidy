//! Board geometry and moves, solution rendering, and helpers for the pattern databases:
//! indexing placements ([`index`]), building tables ([`build`]) and decoding the root's values
//! ([`walk`]).
//!
//! Boards are stored in the solver's *internal* (serpentine) order:
//!
//! ```text
//! 00 01 02 03
//! 07 06 05 04
//! 08 09 10 11
//! 15 14 13 12
//! ```
//!
//! The user-facing format is row-major with tiles 1–15 and 0 as the blank:
//! `1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0`. `to_internal` converts it;
//! solutions come back as the moved tiles' directions (`directions`).

pub(super) mod build;
pub(super) mod index;
pub(super) mod walk;

use crate::algorithm::direction::Direction;

/// A 4x4 board: one tile per cell, 0 is the blank.
pub(super) type Board = [u8; 16];

/// Blank-move directions come from slidy (`Up` = 0, `Left` = 1, `Down` = 2,
/// `Right` = 3), matching the columns of this table.
/// `NEIGHBOR_IN_DIRECTION[blank][direction]` is the internal position of the blank after moving
/// it in direction `direction` ([`Direction`] order: up, left, down, right),
/// or -1 when that move leaves the board.
const NEIGHBOR_IN_DIRECTION: [[i8; 4]; 16] = [
    [-1, -1, 7, 1],
    [-1, 0, 6, 2],
    [-1, 1, 5, 3],
    [-1, 2, 4, -1],
    [3, 5, 11, -1],
    [2, 6, 10, 4],
    [1, 7, 9, 5],
    [0, -1, 8, 6],
    [7, -1, 15, 9],
    [6, 8, 14, 10],
    [5, 9, 13, 11],
    [4, 10, 12, -1],
    [11, 13, -1, -1],
    [10, 14, -1, 12],
    [9, 15, -1, 13],
    [8, -1, -1, 14],
];

/// `NEIGHBOR_MASKS[cell]`: the cells next to `cell`, as a mask.
const NEIGHBOR_MASKS: [u16; 16] = {
    let mut out = [0; 16];
    let mut cell = 0;
    while cell < 16 {
        let mut direction = 0;
        while direction < 4 {
            if NEIGHBOR_IN_DIRECTION[cell][direction] >= 0 {
                out[cell] |= 1 << NEIGHBOR_IN_DIRECTION[cell][direction];
            }
            direction += 1;
        }
        cell += 1;
    }
    out
};

/// Hints the cache line at `address` into the cache ahead of a read. The tables are far larger
/// than any cache, so nearly every lookup misses without this.
#[inline(always)]
pub(super) fn prefetch(address: *const u8) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: SSE is enabled on all x86_64 targets, and a prefetch never faults.
    unsafe {
        std::arch::x86_64::_mm_prefetch::<{ std::arch::x86_64::_MM_HINT_T0 }>(address.cast());
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = address;
}

/// One usable move from a blank cell.
///
/// Everything about a move that depends only on the blank's cell and the direction is
/// precomputed, per orientation (`[direct, mirrored]`).
#[derive(Clone, Copy)]
// `pub` (in this private module) because the sealed `Search` trait, which `Pdb` exposes as a
// supertrait, takes it; nothing outside the 4x4 STM solver can name it.
pub struct Move {
    /// The direction the blank moves ([`Direction`] order: up 0, left 1, down 2, right 3).
    pub(super) direction: u8,
    /// The blank's cell after the move.
    pub(super) blank_after: [u8; 2],
    /// The lower of the blank's cells before and after the move.
    pub(super) lower_cell: [u8; 2],
    /// How far apart the blank's cells before and after the move are, in label order: the cells
    /// strictly between them are the ones the moved tile passes.
    pub(super) cell_gap: [u8; 2],
    /// Whether the blank moves to the higher cell (so the moved tile to the lower one).
    pub(super) blank_to_higher_cell: [bool; 2],
}

/// The usable moves from one blank cell.
#[derive(Clone, Copy)]
pub(super) struct MoveList {
    /// How many moves there are.
    pub(super) count: u8,
    pub(super) moves: [Move; 4],
}

/// `MOVES[blank][allowed]`: the moves available from `blank` among the
/// directions allowed by `allowed` (bit `d` for slidy direction `d`; the
/// duplicate-pruning automaton supplies the mask, which also excludes
/// the move that would undo the previous one). The moves are listed in
/// generation order: left, right, up, down.
pub(super) const MOVES: [[MoveList; 16]; 16] = build_moves();

/// The order the successors are generated in.
const GENERATION_ORDER: [u8; 4] = [
    Direction::Left as u8,
    Direction::Right as u8,
    Direction::Up as u8,
    Direction::Down as u8,
];

const fn build_moves() -> [[MoveList; 16]; 16] {
    let none = Move {
        direction: 0,
        blank_after: [0; 2],
        lower_cell: [0; 2],
        cell_gap: [0; 2],
        blank_to_higher_cell: [false; 2],
    };
    let mut table = [[MoveList {
        count: 0,
        moves: [none; 4],
    }; 16]; 16];
    let mut blank = 0;
    while blank < 16 {
        let mut allowed = 0;
        while allowed < 16 {
            let list = &mut table[blank][allowed];
            let mut order_index = 0;
            while order_index < 4 {
                let direction = GENERATION_ORDER[order_index];
                let neighbor = NEIGHBOR_IN_DIRECTION[blank][direction as usize];
                if allowed & (1 << direction) != 0 && neighbor >= 0 {
                    let mut tile_move = Move { direction, ..none };
                    // The mirrored move joins the mirrored cells.
                    let cells = [
                        (blank, neighbor as usize),
                        (
                            MIRRORED_CELL[blank] as usize,
                            MIRRORED_CELL[neighbor as usize] as usize,
                        ),
                    ];
                    let mut side = 0;
                    while side < 2 {
                        let (from, to) = cells[side];
                        tile_move.blank_after[side] = to as u8;
                        tile_move.lower_cell[side] = if from < to { from } else { to } as u8;
                        tile_move.cell_gap[side] = from.abs_diff(to) as u8;
                        tile_move.blank_to_higher_cell[side] = from < to;
                        side += 1;
                    }
                    list.moves[list.count as usize] = tile_move;
                    list.count += 1;
                }
                order_index += 1;
            }
            allowed += 1;
        }
        blank += 1;
    }
    table
}

/// Mirror of an internal position across the main diagonal.
pub(super) const MIRRORED_CELL: [u8; 16] = [0, 7, 8, 15, 14, 9, 6, 1, 2, 5, 10, 13, 12, 11, 4, 3];

/// The internal cell of each row-major cell.
const ROW_MAJOR_TO_INTERNAL: [u8; 16] = [0, 1, 2, 3, 7, 6, 5, 4, 8, 9, 10, 11, 15, 14, 13, 12];

/// Reflect a board across the main diagonal. The mirrored search uses the
/// same tables on the mirrored position, and the heuristic takes the better
/// (larger) of both orientations.
#[must_use]
pub(super) fn reflect(board: &Board) -> Board {
    std::array::from_fn(|cell| MIRRORED_CELL[board[MIRRORED_CELL[cell] as usize] as usize])
}

/// Internal position of the blank.
#[must_use]
pub(super) fn blank_position(board: &Board) -> usize {
    board
        .iter()
        .position(|&tile| tile == 0)
        .expect("board always contains the blank")
}

/// Convert a user board (row-major, tiles 1–15, 0 blank) to internal order.
#[must_use]
pub(super) fn to_internal(input: &Board) -> Board {
    std::array::from_fn(|cell| {
        let tile = input[15 - ROW_MAJOR_TO_INTERNAL[cell] as usize];
        ROW_MAJOR_TO_INTERNAL[(16 - tile as usize) % 16]
    })
}

/// The moves of a blank trail (internal cells after each move, from `blank`), as the directions
/// the moved tiles go.
pub(super) fn directions(
    blank: usize,
    trail: impl Iterator<Item = usize>,
) -> impl Iterator<Item = Direction> {
    // Row-major (row, column) of an internal cell, as `to_internal` maps it.
    let row_and_column = |internal: usize| {
        let cell = 15 - ROW_MAJOR_TO_INTERNAL[internal] as i8;
        (cell / 4, cell % 4)
    };
    let mut from = row_and_column(blank);
    trail.map(move |cell| {
        let to = row_and_column(cell);
        // The blank moves (to - from); the tile goes the opposite way.
        let tile_move = match (to.0 - from.0, to.1 - from.1) {
            (-1, 0) => Direction::Down,
            (1, 0) => Direction::Up,
            (0, -1) => Direction::Right,
            (0, 1) => Direction::Left,
            _ => panic!("solution moves to neighboring squares"),
        };
        from = to;
        tile_move
    })
}

/// `count` random internal boards (a fixed xorshift sequence), for tests.
#[cfg(test)]
fn random_boards(count: usize) -> impl Iterator<Item = Board> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    std::iter::repeat_with(move || {
        let mut board: Board = std::array::from_fn(|cell| cell as u8);
        for last in (1..16).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            board.swap(last, (state % (last as u64 + 1)) as usize);
        }
        board
    })
    .take(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_format_conversions() {
        // The solved user board maps to every tile home (tile `t` on cell `t`).
        let solved: Board = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0];
        let home: Board = std::array::from_fn(|cell| cell as u8);
        assert_eq!(to_internal(&solved), home);
        assert_eq!(reflect(&home), home);
        // A blank trail renders in order (internal 0 to 1 is the blank moving left in the user's
        // board, so a tile moving right).
        let moves = |blank, trail: &[usize]| {
            directions(blank, trail.iter().copied())
                .map(|direction| direction.as_char())
                .collect::<String>()
        };
        assert_eq!(moves(0, &[1, 2, 3]), "RRR");
        assert_eq!(moves(0, &[7]), "D");
    }

    #[test]
    fn reflecting_twice_is_the_identity() {
        for board in random_boards(10) {
            assert_eq!(reflect(&reflect(&board)), board);
        }
    }
}
