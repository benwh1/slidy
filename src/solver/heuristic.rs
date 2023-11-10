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

macro_rules! impl_manhattan {
    ($label:ty, $dist:expr, $parity_fix:literal $(,)?) => {
        impl<T: PrimInt + Unsigned + 'static> Heuristic<$label, T> for ManhattanDistance
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
                    let pos = puzzle.gap_position_xy();
                    let solved_pos = puzzle.solved_pos_xy(P::Piece::zero());
                    let parity = dist(pos, solved_pos) % 2;

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
