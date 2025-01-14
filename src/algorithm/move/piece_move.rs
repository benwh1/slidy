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

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Represents a move of the piece with the given number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PieceMove<Piece>(pub Piece)
where
    Piece: PrimInt;

/// Error type for the implementation of [`TryIntoMove`] for [`PieceMove`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use crate::{
        algorithm::r#move::{
            piece_move::{PieceMove, TryPieceMoveIntoMoveError},
            r#move::Move,
            try_into_move::TryIntoMove as _,
        },
        puzzle::puzzle::Puzzle,
    };

    #[test]
    fn test_try_into_move() {
        let p = Puzzle::from_str("7 4 14 10/11 0 12 5/1 9 2 3/15 8 6 13").unwrap();
        assert_eq!(
            PieceMove(5).try_into_move(&p),
            Ok(Move::from_str("L2").unwrap())
        );
        assert_eq!(
            PieceMove(8).try_into_move(&p),
            Ok(Move::from_str("U2").unwrap())
        );
        assert_eq!(
            PieceMove(11).try_into_move(&p),
            Ok(Move::from_str("R").unwrap())
        );
        assert_eq!(
            PieceMove(4).try_into_move(&p),
            Ok(Move::from_str("D").unwrap())
        );
        assert_eq!(
            PieceMove(7).try_into_move(&p),
            Err(TryPieceMoveIntoMoveError::InvalidMove(7))
        );
    }
}
