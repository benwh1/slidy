#![allow(missing_docs)]

use crate::{algorithm::r#move::r#move::Move, puzzle::sliding_puzzle::SlidingPuzzle};

pub trait TryIntoMove<Puzzle: SlidingPuzzle> {
    type Error;

    fn try_into_move(&self, puzzle: &Puzzle) -> Result<Move, Self::Error>;
}
