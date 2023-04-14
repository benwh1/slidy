//! Defines the [`PieceMove`] type.

use num_traits::PrimInt;
use thiserror::Error;

use crate::{
    algorithm::r#move::{
        position_move::{PositionMove, TryPositionMoveIntoMoveError},
        r#move::Move,
        try_into_move::TryIntoMove,
    },
    puzzle::sliding_puzzle::SlidingPuzzle,
};

/// Represents a move of the piece with the given number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PieceMove<Piece>(pub Piece)
where
    Piece: PrimInt;

/// Error type for the implementation of [`TryIntoMove`] for [`PieceMove`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TryPieceMoveIntoMoveError<Piece: PrimInt> {
    /// Returned when the piece can not be moved.
    #[error("InvalidMove: piece {0} can not be moved")]
    InvalidMove(Piece),
}

impl<Piece: PrimInt, Puzzle: SlidingPuzzle<Piece = Piece>> TryIntoMove<Puzzle>
    for PieceMove<Piece>
{
    type Error = TryPieceMoveIntoMoveError<Piece>;

    fn try_into_move(&self, puzzle: &Puzzle) -> Result<Move, Self::Error> {
        let (x, y) = puzzle.piece_position_xy(self.0);

        PositionMove(x, y).try_into_move(puzzle).map_err(
            |TryPositionMoveIntoMoveError::InvalidMove(_, _)| Self::Error::InvalidMove(self.0),
        )
    }
}
