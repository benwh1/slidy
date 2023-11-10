//! Defines the [`ManhattanDistance`] heuristic, which is the sum of the manhattan distances of
//! each piece from it's solved position.

use itertools::Itertools;
use num_traits::{AsPrimitive, PrimInt, Unsigned, Zero};

use crate::{
    puzzle::{
        label::labels::{Checkerboard, Diagonals, Fringe, RowGrids, Rows, Trivial},
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

macro_rules! impl_manhattan {
    ($label:ty, $dist:expr, $parity_fix:literal $(,)?) => {
        impl<T: PrimInt + Unsigned + 'static> Heuristic<T> for ManhattanDistance<'_, $label>
        where
            usize: AsPrimitive<T>,
        {
            fn bound<P: SlidingPuzzle>(&self, puzzle: &P) -> T {
                let dist: fn((usize, usize), (usize, usize)) -> usize = $dist;

                let (w, h) = puzzle.size().into();
                let md = (0..w)
                    .cartesian_product(0..h)
                    .map(|pos| {
                        let piece = puzzle.piece_at_xy(pos);
                        let solved_pos = puzzle.solved_pos_xy(piece);
                        let md = dist(pos, solved_pos);

                        if piece == P::Piece::zero() {
                            0
                        } else {
                            md
                        }
                    })
                    .sum::<usize>();

                if $parity_fix {
                    // Make sure the parity is correct (some positions will give an even bound for a position
                    // that takes an odd number of moves, etc.)
                    let (x, y) = puzzle.gap_position_xy();
                    let (sx, sy) = puzzle.solved_pos_xy(P::Piece::zero());

                    // Actual Manhattan distance, not `dist`
                    let parity = (x.abs_diff(sx) + y.abs_diff(sy)) % 2;

                    let adjusted_md = if md % 2 == parity { md } else { md + 1 };
                    adjusted_md.as_()
                } else {
                    md.as_()
                }
            }
        }
    };
}

impl_manhattan!(
    RowGrids,
    |(x, y), (sx, sy)| x.abs_diff(sx) + y.abs_diff(sy),
    false,
);

impl_manhattan!(Rows, |(_, y), (_, sy)| y.abs_diff(sy), true);

impl_manhattan!(
    Fringe,
    |(x, y), (sx, sy)| x.abs_diff(sx).min(y.abs_diff(sy)),
    true
);

impl_manhattan!(
    Diagonals,
    |(x, y), (sx, sy)| (x + y).abs_diff(sx + sy),
    true
);

impl_manhattan!(
    Checkerboard,
    |(x, y), (sx, sy)| usize::from((x + y) % 2 != (sx + sy) % 2),
    true
);
