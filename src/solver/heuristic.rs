//! Defines the [`Heuristic`] trait and the [`ManhattanDistance`] heuristic.

use itertools::Itertools;
use num_traits::{AsPrimitive, PrimInt, Unsigned, Zero};

use crate::puzzle::sliding_puzzle::SlidingPuzzle;

/// Provides a function returning a lower bound on the number of moves needed to solve a puzzle.
pub trait Heuristic<Puzzle, T>
where
    Puzzle: SlidingPuzzle,
    T: PrimInt + Unsigned,
{
    /// Returns a lower bound on the number of moves needed to solve `puzzle`.
    #[must_use]
    fn bound(&self, puzzle: &Puzzle) -> T;
}

/// Manhattan distance heuristic.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManhattanDistance;

impl<Puzzle, T> Heuristic<Puzzle, T> for ManhattanDistance
where
    Puzzle: SlidingPuzzle,
    T: PrimInt + Unsigned + 'static,
    usize: AsPrimitive<T>,
{
    fn bound(&self, puzzle: &Puzzle) -> T {
        let (w, h) = puzzle.size().into();
        (0..w)
            .cartesian_product(0..h)
            .map(|(x, y)| {
                let piece = puzzle.piece_at_xy((x, y));
                let (a, b) = puzzle.solved_pos_xy(piece);

                if piece == Puzzle::Piece::zero() {
                    0
                } else {
                    x.abs_diff(a) + y.abs_diff(b)
                }
            })
            .sum::<usize>()
            .as_()
    }
}
