use itertools::Itertools;
use num_traits::{PrimInt, Unsigned};

use crate::puzzle::sliding_puzzle::SlidingPuzzle;

pub trait Heuristic<Piece, Puzzle, T>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
    T: PrimInt + Unsigned,
{
    #[must_use]
    fn bound(&self, puzzle: &Puzzle) -> T;
}

pub struct ManhattanDistance;

impl<Piece, Puzzle, T> Heuristic<Piece, Puzzle, T> for ManhattanDistance
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
    T: PrimInt + Unsigned + TryFrom<usize>,
{
    fn bound(&self, puzzle: &Puzzle) -> T {
        let (w, h) = puzzle.size();
        (0..w)
            .cartesian_product(0..h)
            .map(|(x, y)| {
                let piece = puzzle.piece_at_xy_unchecked(x, y);
                let (a, b) = puzzle.solved_pos_xy_unchecked(piece);

                if piece == Piece::zero() {
                    0
                } else {
                    x.abs_diff(a) + y.abs_diff(b)
                }
            })
            .sum::<usize>()
            .try_into()
            .unwrap_or_else(|_| T::zero())
    }
}
