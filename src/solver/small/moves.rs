//! The moves of the gap in a small `WxH` puzzle, and how they change the [`Dense`] index of a
//! state.

use crate::solver::{directions::gap_moves, small::coord::Dense};

/// The moves of the gap in a `WxH` puzzle with `N` cells.
pub(super) struct Moves<const W: usize, const H: usize, const N: usize>;

impl<const W: usize, const H: usize, const N: usize> Moves<W, H, N> {
    /// For each cell of the gap, the cells that it moves to in each direction (0 for moves that
    /// aren't possible), and the set of possible moves (see [`gap_moves`]).
    pub(super) const MOVES: [([u8; 4], u8); N] = {
        let mut out = [([0; 4], 0); N];
        let mut gap = 0;
        while gap < N {
            out[gap] = gap_moves(W, H, gap);
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
