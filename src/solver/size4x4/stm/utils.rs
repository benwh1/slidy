//! Board geometry, pattern-database index weights and solution rendering.
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

use std::error::Error;

use crate::{
    algorithm::direction::Direction,
    puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle as _},
};

/// A 4x4 board: one tile per cell, 0 is the blank.
pub type Board = [u8; 16];

/// Blank-move directions come from slidy (`Up` = 0, `Left` = 1, `Down` = 2,
/// `Right` = 3), matching the columns of this table.
/// `NEIGHBORS[blank][dir]` is the internal position of the blank after moving
/// it in direction `dir` ([`Direction`] order: up, left, down, right),
/// or -1 when that move leaves the board.
pub const NEIGHBORS: [[i8; 4]; 16] = [
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

/// One usable move from a blank cell.
///
/// Everything about a move that depends only on the blank's cell and
/// the direction is precomputed, per orientation (`[direct, mirrored]`):
/// the neighbour, the lower of the two blank positions and the gap
/// between them (the cells the index scan walks), and the sign of the
/// shift.
#[derive(Clone, Copy)]
pub struct Move {
    pub(crate) dir: u8,
    pub(crate) next: [u8; 2],
    pub(crate) lo: [u8; 2],
    pub(crate) gap: [u8; 2],
    /// 1 when the blank moves to the higher cell (the shift is negated).
    pub(crate) sign: [u8; 2],
}

/// The usable moves from one blank cell.
#[derive(Clone, Copy)]
pub(crate) struct MoveList {
    pub(crate) n: u8,
    pub(crate) moves: [Move; 4],
}

/// `MOVES[blank][mask]`: the moves available from `blank` among the
/// directions allowed by `mask` (bit `d` for slidy direction `d`; the
/// duplicate-pruning automaton supplies the mask, which also excludes
/// the move that would undo the previous one). The moves are listed in
/// generation order: left, right, up, down.
pub(crate) const MOVES: [[MoveList; 16]; 16] = build_moves();

/// The order the successors are generated in.
const MOVE_DIRS: [u8; 4] = [
    Direction::Left as u8,
    Direction::Right as u8,
    Direction::Up as u8,
    Direction::Down as u8,
];

const fn build_moves() -> [[MoveList; 16]; 16] {
    let none = Move {
        dir: 0,
        next: [0; 2],
        lo: [0; 2],
        gap: [0; 2],
        sign: [0; 2],
    };
    let mut out = [[MoveList {
        n: 0,
        moves: [none; 4],
    }; 16]; 16];
    let mut blank = 0;
    while blank < 16 {
        let mut mask = 0;
        while mask < 16 {
            let list = &mut out[blank][mask];
            let mut k = 0;
            while k < 4 {
                let dir = MOVE_DIRS[k];
                let next = NEIGHBORS[blank][dir as usize];
                if mask & (1 << dir) != 0 && next >= 0 {
                    let mut m = Move { dir, ..none };
                    // The mirrored move joins the mirrored cells.
                    let ends = [
                        (blank, next as usize),
                        (MIRROR[blank] as usize, MIRROR[next as usize] as usize),
                    ];
                    let mut side = 0;
                    while side < 2 {
                        let (from, to) = ends[side];
                        m.next[side] = to as u8;
                        m.lo[side] = if from < to { from } else { to } as u8;
                        m.gap[side] = from.abs_diff(to) as u8;
                        m.sign[side] = (from < to) as u8;
                        side += 1;
                    }
                    list.moves[list.n as usize] = m;
                    list.n += 1;
                }
                k += 1;
            }
            mask += 1;
        }
        blank += 1;
    }
    out
}

/// Mirror of an internal position across the main diagonal.
pub(crate) const MIRROR: [u8; 16] = [0, 7, 8, 15, 14, 9, 6, 1, 2, 5, 10, 13, 12, 11, 4, 3];

/// Internal position of zero-based cell `i`.
const FROM_ZERO_BASED: [u8; 16] = [0, 1, 2, 3, 7, 6, 5, 4, 8, 9, 10, 11, 15, 14, 13, 12];

/// Permutations of `k` out of `n`: `n * (n-1) * … * (n-k+1)`, or 0 when
/// `k` is out of range. Determines the pattern-database sizes.
#[must_use]
pub(crate) const fn permutations(n: i32, k: i32) -> i32 {
    if k < 0 || n < k {
        return 0;
    }
    let mut total = 1;
    let mut i = n;
    while i != n - k {
        total *= i;
        i -= 1;
    }
    total
}

/// Reflect a board across the main diagonal. The mirrored search uses the
/// same tables on the mirrored position, and the heuristic takes the better
/// (larger) of both orientations.
#[must_use]
pub(crate) fn reflect(board: &Board) -> Board {
    std::array::from_fn(|i| MIRROR[board[MIRROR[i] as usize] as usize])
}

/// Internal position of the blank.
#[must_use]
pub(crate) fn blank_position(board: &Board) -> usize {
    board
        .iter()
        .position(|&tile| tile == 0)
        .expect("board always contains the blank")
}

/// Convert a user board (row-major, tiles 1–15, 0 blank) to internal order.
#[must_use]
pub(crate) fn to_internal(input: &Board) -> Board {
    std::array::from_fn(|i| {
        let tile = input[15 - FROM_ZERO_BASED[i] as usize];
        FROM_ZERO_BASED[(16 - tile as usize) % 16]
    })
}

/// Parse `1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0` into a row-major board.
/// Delegates to slidy's parser (same accepted language: digits separated by
/// whitespace, `/` or newlines; duplicates, out-of-range tiles and ragged
/// rows rejected) and additionally requires the 4x4 shape our solver needs.
pub fn parse_scramble(text: &str) -> Result<Board, Box<dyn Error>> {
    // Trim first: slidy counts a trailing newline as an (empty) extra row.
    let puzzle = text.trim().parse::<Puzzle>()?;
    if puzzle.size().width() != 4 || puzzle.size().height() != 4 {
        Err("expected a 4x4 position")?;
    }
    Ok(std::array::from_fn(|i| puzzle.piece_at(i as u64) as u8))
}

/// The moves of a blank trail (internal cells after each move, from `blank`), as the directions
/// the moved tiles go.
pub(crate) fn directions(
    blank: usize,
    trail: impl Iterator<Item = usize>,
) -> impl Iterator<Item = Direction> {
    // Row-major (row, column) of an internal cell, as `to_internal` maps it.
    let at = |internal: usize| {
        let cell = 15 - FROM_ZERO_BASED[internal] as i8;
        (cell / 4, cell % 4)
    };
    let mut from = at(blank);
    trail.map(move |cell| {
        let to = at(cell);
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
pub(crate) fn random_boards(count: usize) -> impl Iterator<Item = Board> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    std::iter::repeat_with(move || {
        let mut board: Board = std::array::from_fn(|i| i as u8);
        for i in (1..16).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            board.swap(i, (state % (i as u64 + 1)) as usize);
        }
        board
    })
    .take(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzle::small::Puzzle4x4;

    #[test]
    fn user_format_conversions() {
        // Solved user board maps to all-home internal (tile i at position i).
        let solved: Board = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0];
        let internal = to_internal(&solved);
        assert_eq!(
            internal,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
        assert!(Puzzle4x4::try_from(solved).unwrap().is_solved());
        assert!(
            !Puzzle4x4::try_from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 14, 0])
                .unwrap()
                .is_solved()
        );
        // Multi-step blank trail renders in order (serpentine internal 0→1 is
        // row-major right).
        let solution_string = |blank, trail: &[usize]| {
            directions(blank, trail.iter().copied())
                .map(|d| d.as_char())
                .collect::<String>()
        };
        assert_eq!(solution_string(0, &[1, 2, 3]), "RRR");
        assert_eq!(solution_string(0, &[7]), "D");
    }

    #[test]
    fn round_trips() {
        let pos: Board = [2, 11, 10, 9, 13, 8, 14, 12, 15, 1, 5, 4, 7, 3, 0, 6];
        let internal = to_internal(&pos);
        // Reflecting twice is the identity.
        assert_eq!(reflect(&reflect(&internal)), internal);
        // Swapping two non-blank tiles flips solvability.
        let home = to_internal(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0]);
        assert_eq!(home, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let solved_user: Board = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0];
        assert!(Puzzle4x4::try_from(solved_user).unwrap().is_solvable());
        let mut swapped = solved_user;
        swapped.swap(12, 13);
        assert!(!Puzzle4x4::try_from(swapped).unwrap().is_solvable());
        assert!(
            Puzzle4x4::try_from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0])
                .unwrap()
                .is_solved()
        );
        assert!(
            !Puzzle4x4::try_from([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 14, 0])
                .unwrap()
                .is_solved()
        );
    }
}
