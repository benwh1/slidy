//! Defines the [`Heuristic`] trait and the [`ManhattanDistance`] heuristic.

use itertools::Itertools;
use num_traits::{AsPrimitive, PrimInt, Unsigned, Zero};

use crate::puzzle::{
    label::labels::{RowGrids, Rows},
    sliding_puzzle::SlidingPuzzle,
    solved_state::SolvedState,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Provides a function returning a lower bound on the number of moves needed to solve a puzzle.
pub trait Heuristic<S: SolvedState, T: PrimInt + Unsigned> {
    /// Returns a lower bound on the number of moves needed to solve `puzzle`.
    #[must_use]
    fn bound<P: SlidingPuzzle>(&self, puzzle: &P) -> T;
}

/// Manhattan distance heuristic.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ManhattanDistance;

impl<T: PrimInt + Unsigned + 'static> Heuristic<RowGrids, T> for ManhattanDistance
where
    usize: AsPrimitive<T>,
{
    fn bound<P: SlidingPuzzle>(&self, puzzle: &P) -> T {
        let (w, h) = puzzle.size().into();
        (0..w)
            .cartesian_product(0..h)
            .map(|(x, y)| {
                let piece = puzzle.piece_at_xy((x, y));
                let (a, b) = puzzle.solved_pos_xy(piece);

                if piece == P::Piece::zero() {
                    0
                } else {
                    x.abs_diff(a) + y.abs_diff(b)
                }
            })
            .sum::<usize>()
            .as_()
    }
}

impl<T: PrimInt + Unsigned + 'static> Heuristic<Rows, T> for ManhattanDistance
where
    usize: AsPrimitive<T>,
{
    fn bound<P: SlidingPuzzle>(&self, puzzle: &P) -> T {
        let (w, h) = puzzle.size().into();
        let md = (0..w)
            .cartesian_product(0..h)
            .map(|(x, y)| {
                let piece = puzzle.piece_at_xy((x, y));
                let (_, b) = puzzle.solved_pos_xy(piece);

                if piece == P::Piece::zero() {
                    0
                } else {
                    y.abs_diff(b)
                }
            })
            .sum::<usize>();

        // Make sure the parity is correct (some positions will give an even bound for a position
        // that takes an odd number of moves, etc.)
        let (gx, gy) = puzzle.gap_position_xy();
        let (sx, sy) = puzzle.solved_pos_xy(P::Piece::zero());

        let adjusted_md = if md % 2 == (gx.abs_diff(sx) + gy.abs_diff(sy)) % 2 {
            md
        } else {
            md + 1
        };

        adjusted_md.as_()
    }
}
