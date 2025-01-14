//! Defines the [`ManhattanDistance`] heuristic, which is the sum of the manhattan distances of
//! each piece from it's solved position.

use std::cmp::Ordering;

use itertools::Itertools as _;
use num_traits::{AsPrimitive, PrimInt, Unsigned, Zero as _};

use crate::{
    puzzle::{
        label::label::{
            Checkerboard, Diagonals, Fringe, Label, RowGrids, Rows, SplitFringe, SplitSquareFringe,
            SquareFringe, Trivial,
        },
        size::Size,
        sliding_puzzle::SlidingPuzzle,
        solved_state::SolvedState,
    },
    solver::heuristic::Heuristic,
};

/// Manhattan distance heuristic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManhattanDistance<'a, S: SolvedState>(pub &'a S);

impl<P: SlidingPuzzle, T: PrimInt + Unsigned + 'static> Heuristic<P, T>
    for ManhattanDistance<'_, Trivial>
where
    u64: AsPrimitive<T>,
{
    fn bound(&self, puzzle: &P) -> T {
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
    fn dist(&self, pos: (u64, u64), solved_pos: (u64, u64), size: Size) -> u64;
}

impl Distance for ManhattanDistance<'_, RowGrids> {
    const HAS_PARITY_CONSTRAINT: bool = true;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        x.abs_diff(sx) + y.abs_diff(sy)
    }
}

impl Distance for ManhattanDistance<'_, Rows> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (_, y): (u64, u64), (_, sy): (u64, u64), _size: Size) -> u64 {
        y.abs_diff(sy)
    }
}

impl Distance for ManhattanDistance<'_, Fringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), size: Size) -> u64 {
        let fringe = Fringe.position_label(size, (sx, sy));
        fringe.saturating_sub(x)
            + fringe.saturating_sub(y)
            + x.saturating_sub(fringe).min(y.saturating_sub(fringe))
    }
}

impl Distance for ManhattanDistance<'_, SquareFringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), size: Size) -> u64 {
        let (w, h) = size.into();
        match w.cmp(&h) {
            Ordering::Less => {
                if sy < h.saturating_sub(w) {
                    // Solved position is above the square part. Distance is the vertical distance
                    // from the current row to the target row.
                    y.abs_diff(sy)
                } else {
                    // Solved position is within the square part. Distance is the number of rows
                    // we need to move down to reach the square part (or 0 if we are already in the
                    // square part), plus the distance within the square part.
                    let size_diff = h - w;
                    let vertical_distance = size_diff.saturating_sub(y);
                    let square_distance = ManhattanDistance(&Fringe).dist(
                        (x, y.saturating_sub(size_diff)),
                        (sx, sy - size_diff),
                        size.shrink_to_square(),
                    );

                    vertical_distance + square_distance
                }
            }
            Ordering::Equal => ManhattanDistance(&Fringe).dist((x, y), (sx, sy), size),
            Ordering::Greater => {
                // Same as above, but for the horizontal direction.
                if sx < w.saturating_sub(h) {
                    x.abs_diff(sx)
                } else {
                    let size_diff = w - h;
                    let horizontal_distance = size_diff.saturating_sub(x);
                    let square_distance = ManhattanDistance(&Fringe).dist(
                        (x.saturating_sub(size_diff), y),
                        (sx - size_diff, sy),
                        size.shrink_to_square(),
                    );

                    horizontal_distance + square_distance
                }
            }
        }
    }
}

impl Distance for ManhattanDistance<'_, SplitFringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        if sx < sy {
            x.abs_diff(sx) + (sx + 1).saturating_sub(y)
        } else {
            y.abs_diff(sy) + sy.saturating_sub(x)
        }
    }
}

impl Distance for ManhattanDistance<'_, SplitSquareFringe> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), size: Size) -> u64 {
        // Same as for `SplitFringe` above, but using `SplitFringe` for the square part.
        let (w, h) = size.into();
        match w.cmp(&h) {
            Ordering::Less => {
                if sy < h.saturating_sub(w) {
                    y.abs_diff(sy)
                } else {
                    let size_diff = h - w;
                    let vertical_distance = size_diff.saturating_sub(y);
                    let square_distance = ManhattanDistance(&SplitFringe).dist(
                        (x, y.saturating_sub(size_diff)),
                        (sx, sy - size_diff),
                        size.shrink_to_square(),
                    );

                    vertical_distance + square_distance
                }
            }
            Ordering::Equal => ManhattanDistance(&SplitFringe).dist((x, y), (sx, sy), size),
            Ordering::Greater => {
                if sx < w.saturating_sub(h) {
                    x.abs_diff(sx)
                } else {
                    let size_diff = w - h;
                    let horizontal_distance = size_diff.saturating_sub(x);
                    let square_distance = ManhattanDistance(&SplitFringe).dist(
                        (x.saturating_sub(size_diff), y),
                        (sx - size_diff, sy),
                        size.shrink_to_square(),
                    );

                    horizontal_distance + square_distance
                }
            }
        }
    }
}

impl Distance for ManhattanDistance<'_, Diagonals> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        (x + y).abs_diff(sx + sy)
    }
}

impl Distance for ManhattanDistance<'_, Checkerboard> {
    const HAS_PARITY_CONSTRAINT: bool = false;

    fn dist(&self, (x, y): (u64, u64), (sx, sy): (u64, u64), _size: Size) -> u64 {
        u64::from((x + y) % 2 != (sx + sy) % 2)
    }
}

impl<P: SlidingPuzzle, T: PrimInt + Unsigned + 'static, L: Label> Heuristic<P, T>
    for ManhattanDistance<'_, L>
where
    u64: AsPrimitive<T>,
    Self: Distance,
{
    fn bound(&self, puzzle: &P) -> T {
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
            .sum::<u64>();

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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_manhattan_distance {
        ($label:ty, $($w:literal x $h:literal, $solved_pos:literal : $dists:expr),+ $(,)?) => {
            paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::size::Size;

                    use super::*;

                    $(#[test]
                    fn [< test_ $label:snake _ $w x $h _ $solved_pos >] () {
                        let size = Size::new($w, $h).unwrap();
                        let md = ManhattanDistance(&$label);

                        #[allow(clippy::identity_op)]
                        let solved_pos_xy = ($solved_pos % $w, $solved_pos / $w);

                        let dists = (0..size.area())
                            .map(|i| md.dist((i % $w, i / $w), solved_pos_xy, size))
                            .collect::<Vec<_>>();
                        assert_eq!(dists, $dists);
                    })*
                }
            }
        };
    }

    test_manhattan_distance!(
        RowGrids,
        4 x 4, 2 : [
            2, 1, 0, 1,
            3, 2, 1, 2,
            4, 3, 2, 3,
            5, 4, 3, 4,
        ],
    );

    test_manhattan_distance!(
        Rows,
        4 x 4, 5 : [
            1, 1, 1, 1,
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
        ],
    );

    test_manhattan_distance!(
        Fringe,
        4 x 4, 9 : [
            2, 1, 1, 1,
            1, 0, 0, 0,
            1, 0, 1, 1,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            4, 3, 2, 2, 2, 2,
            3, 2, 1, 1, 1, 1,
            2, 1, 0, 0, 0, 0,
            2, 1, 0, 1, 1, 1,
        ],
    );

    test_manhattan_distance!(
        SquareFringe,
        4 x 4, 9 : [
            2, 1, 1, 1,
            1, 0, 0, 0,
            1, 0, 1, 1,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            6, 5, 4, 3, 2, 2,
            5, 4, 3, 2, 1, 1,
            4, 3, 2, 1, 0, 0,
            4, 3, 2, 1, 0, 1,
        ],
        4 x 6, 5 : [
            1, 1, 1, 1,
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 4, 4, 4,
        ],
    );

    test_manhattan_distance!(
        SplitFringe,
        4 x 4, 5 : [
            2, 1, 1, 1,
            1, 0, 0, 0,
            2, 1, 1, 1,
            3, 2, 2, 2,
        ],
        4 x 4, 9 : [
            3, 2, 3, 4,
            2, 1, 2, 3,
            1, 0, 1, 2,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            4, 3, 2, 2, 2, 2,
            3, 2, 1, 1, 1, 1,
            2, 1, 0, 0, 0, 0,
            3, 2, 1, 1, 1, 1,
        ],
        6 x 4, 20 : [
            5, 4, 3, 4, 5, 6,
            4, 3, 2, 3, 4, 5,
            3, 2, 1, 2, 3, 4,
            2, 1, 0, 1, 2, 3,
        ],
    );

    test_manhattan_distance!(
        SplitSquareFringe,
        4 x 4, 9 : [
            3, 2, 3, 4,
            2, 1, 2, 3,
            1, 0, 1, 2,
            1, 0, 1, 2,
        ],
        6 x 4, 17 : [
            6, 5, 4, 3, 2, 2,
            5, 4, 3, 2, 1, 1,
            4, 3, 2, 1, 0, 0,
            5, 4, 3, 2, 1, 1,
        ],
        4 x 6, 5 : [
            1, 1, 1, 1,
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 4, 4, 4,
        ],
    );
}
