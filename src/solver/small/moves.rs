//! The moves of the gap in a small `WxH` puzzle, and how they change the [`Dense`] index of a
//! state.

use crate::{algorithm::direction::Direction, solver::small::coord::Dense};

/// The directions to search, in order. Bit `d` of a set of moves is `DIRECTIONS[d]`, the inverse
/// of `DIRECTIONS[d]` is `DIRECTIONS[d ^ 2]`, and the moves along the same axis as `DIRECTIONS[d]`
/// are the bits of `0b0101 << (d & 1)`.
pub(super) const DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
];

/// The moves of the gap in a `WxH` puzzle with `N` cells.
pub(super) struct Moves<const W: usize, const H: usize, const N: usize>;

impl<const W: usize, const H: usize, const N: usize> Moves<W, H, N> {
    /// For each cell of the gap, the cells that it moves to in each direction (ignoring moves
    /// that aren't possible), and the set of possible moves.
    pub(super) const MOVES: [([u8; 4], u8); N] = {
        let mut out = [([0; 4], 0); N];
        let mut gap = 0;
        while gap < N {
            let (x, y) = (gap % W, gap / W);
            let possible = [y + 1 < H, x + 1 < W, y > 0, x > 0];
            let cells = [gap + W, gap + 1, gap.wrapping_sub(W), gap.wrapping_sub(1)];
            let mut d = 0;
            while d < 4 {
                if possible[d] {
                    out[gap].0[d] = cells[d] as u8;
                    out[gap].1 |= 1 << d;
                }
                d += 1;
            }
            gap += 1;
        }
        out
    };

    /// Returns the index of the state reached by moving in direction `DIRECTIONS[d]` from the
    /// state with the given pieces, gap, and index. The move must be possible.
    #[inline(always)]
    pub(super) fn child_index(pieces: u64, gap: u8, index: u64, d: u32) -> u64 {
        match d {
            0 => index.wrapping_add(Dense::<W, H, N>::delta_down(pieces, gap)),
            1 => index + 1,
            2 => index.wrapping_add(Dense::<W, H, N>::delta_up(pieces, gap)),
            _ => index - 1,
        }
    }
}
