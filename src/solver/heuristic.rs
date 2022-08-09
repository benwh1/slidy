use num_traits::{PrimInt, Unsigned};

use crate::puzzle::sliding_puzzle::SlidingPuzzle;

pub trait Heuristic<Piece, Puzzle, T>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
    T: PrimInt + Unsigned,
{
    fn bound(&self, puzzle: &Puzzle) -> T;
}
