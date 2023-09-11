//! Defines the [`PositionMove`] type.

use std::cmp::Ordering;

use thiserror::Error;

use crate::{
    algorithm::{
        direction::Direction,
        r#move::{r#move::Move, try_into_move::TryIntoMove},
    },
    puzzle::sliding_puzzle::SlidingPuzzle,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Represents a move of the piece in the given position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PositionMove(pub usize, pub usize);

/// Error type for the implementation of [`TryIntoMove`] for [`PositionMove`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TryPositionMoveIntoMoveError {
    /// Returned when the piece can not be moved.
    #[error("InvalidMove: position ({0}, {1}) can not be moved")]
    InvalidMove(usize, usize),
}

impl<Puzzle: SlidingPuzzle> TryIntoMove<Puzzle> for PositionMove {
    type Error = TryPositionMoveIntoMoveError;

    fn try_into_move(&self, puzzle: &Puzzle) -> Result<Move, Self::Error> {
        let Self(x, y) = *self;

        if puzzle.can_move_position_xy((x, y)) {
            let (gx, gy) = puzzle.gap_position_xy();
            if x == gx {
                match y.cmp(&gy) {
                    Ordering::Less => Ok(Move::new(Direction::Down, (gy - y) as u32)),
                    Ordering::Greater => Ok(Move::new(Direction::Up, (y - gy) as u32)),
                    Ordering::Equal => Ok(Move::new(Direction::Up, 0)),
                }
            } else {
                // We must have y == gy here
                match x.cmp(&gx) {
                    Ordering::Less => Ok(Move::new(Direction::Right, (gx - x) as u32)),
                    Ordering::Greater => Ok(Move::new(Direction::Left, (x - gx) as u32)),
                    Ordering::Equal => Ok(Move::new(Direction::Right, 0)),
                }
            }
        } else {
            Err(Self::Error::InvalidMove(x, y))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{
        algorithm::r#move::{
            position_move::{PositionMove, TryPositionMoveIntoMoveError},
            r#move::Move,
            try_into_move::TryIntoMove,
        },
        puzzle::puzzle::Puzzle,
    };

    #[test]
    fn test_try_into_move() {
        let p = Puzzle::from_str("7 4 14 10/11 0 12 5/1 9 2 3/15 8 6 13").unwrap();
        assert_eq!(
            PositionMove(3, 1).try_into_move(&p),
            Ok(Move::from_str("L2").unwrap())
        );
        assert_eq!(
            PositionMove(1, 3).try_into_move(&p),
            Ok(Move::from_str("U2").unwrap())
        );
        assert_eq!(
            PositionMove(0, 1).try_into_move(&p),
            Ok(Move::from_str("R").unwrap())
        );
        assert_eq!(
            PositionMove(1, 0).try_into_move(&p),
            Ok(Move::from_str("D").unwrap())
        );
        assert_eq!(
            PositionMove(0, 0).try_into_move(&p),
            Err(TryPositionMoveIntoMoveError::InvalidMove(0, 0))
        );
    }
}
