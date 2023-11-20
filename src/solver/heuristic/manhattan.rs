//! Defines the [`ManhattanDistance`] heuristic, which is the sum of the manhattan distances of
//! each piece from it's solved position.

use itertools::Itertools;
use num_traits::{AsPrimitive, PrimInt, Unsigned, Zero};

use crate::{
    puzzle::{
        label::labels::{Checkerboard, Diagonals, Fringe, Label, RowGrids, Rows, Trivial},
        size::Size,
        sliding_puzzle::SlidingPuzzle,
        solved_state::SolvedState,
    },
    solver::heuristic::Heuristic,
};

/// Manhattan distance heuristic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManhattanDistance<'a, S: SolvedState>(pub &'a S);

impl<T: PrimInt + Unsigned + 'static> Heuristic<T> for ManhattanDistance<'_, Trivial>
where
    usize: AsPrimitive<T>,
{
    fn bound<P: SlidingPuzzle>(&self, puzzle: &P) -> T {
        let (w, h) = puzzle.size().into();
        let (gx, gy) = puzzle.gap_position_xy();
        (w + h - 2 - gx - gy).as_()
    }
}

/// Defines a function computing the shortest distance of a piece from a solved position.
pub trait Distance {
    /// True if the sum of `self.dist(pos, solved_pos, size)` over all non-gap tiles of a puzzle is
    /// guaranteed to be equal to the length of a solution of the puzzle mod 2
    const HAS_PARITY_CONSTRAINT: bool;

    /// Suppose the solved position of the piece in position `pos` is `solved_pos`, then this
    /// function returns the minimum Manhattan distance from `pos` to any position where the piece
    /// is considered solved (according to some [`SolvedState`]).
    fn dist(&self, pos: (usize, usize), solved_pos: (usize, usize), size: Size) -> usize;
}

impl Distance for ManhattanDistance<'_, RowGrids> {
    const HAS_PARITY_CONSTRAINT: bool = true;

    fn dist(&self, (x, y): (usize, usize), (sx, sy): (usize, usize), _size: Size) -> usize {
        x.abs_diff(sx) + y.abs_diff(sy)
    }
}

impl Distance for ManhattanDistance<'_, Rows> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (_, y): (usize, usize), (_, sy): (usize, usize), _size: Size) -> usize {
        y.abs_diff(sy)
    }
}

impl Distance for ManhattanDistance<'_, Fringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (usize, usize), (sx, sy): (usize, usize), _size: Size) -> usize {
        x.abs_diff(sx).min(y.abs_diff(sy))
    }
}

impl Distance for ManhattanDistance<'_, Diagonals> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (usize, usize), (sx, sy): (usize, usize), _size: Size) -> usize {
        (x + y).abs_diff(sx + sy)
    }
}

impl Distance for ManhattanDistance<'_, Checkerboard> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (usize, usize), (sx, sy): (usize, usize), _size: Size) -> usize {
        usize::from((x + y) % 2 != (sx + sy) % 2)
    }
}

impl<T: PrimInt + Unsigned + 'static, L: Label> Heuristic<T> for ManhattanDistance<'_, L>
where
    usize: AsPrimitive<T>,
    Self: Distance,
{
    fn bound<P: SlidingPuzzle>(&self, puzzle: &P) -> T {
        let (w, h) = puzzle.size().into();
        let md = (0..w)
            .cartesian_product(0..h)
            .map(|pos| {
                let piece = puzzle.piece_at_xy(pos);
                let solved_pos = puzzle.solved_pos_xy(piece);
                let md = self.dist(pos, solved_pos, puzzle.size());

                if piece == P::Piece::zero() {
                    0
                } else {
                    md
                }
            })
            .sum::<usize>();

        if Self::HAS_PARITY_CONSTRAINT {
            md.as_()
        } else {
            // Make sure the parity is correct (some positions will give an even bound for a position
            // that takes an odd number of moves, etc.)
            let (x, y) = puzzle.gap_position_xy();
            let (sx, sy) = puzzle.solved_pos_xy(P::Piece::zero());

            // Actual Manhattan distance, not `dist`
            let parity = (x.abs_diff(sx) + y.abs_diff(sy)) % 2;

            let adjusted_md = if md % 2 == parity { md } else { md + 1 };
            adjusted_md.as_()
        }
    }
}
