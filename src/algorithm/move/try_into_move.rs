//! Defines the [`TryIntoMove`] trait.

use crate::{algorithm::r#move::r#move::Move, puzzle::sliding_puzzle::SlidingPuzzle};

/// Defines conversions to [`Move`].
pub trait TryIntoMove<Puzzle: SlidingPuzzle> {
    /// Error type for the conversion.
    type Error;

    /// Attempts to perform the conversion.
    fn try_into_move(&self, puzzle: &Puzzle) -> Result<Move, Self::Error>;
}
