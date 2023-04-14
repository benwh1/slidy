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

/// Represents a move of the piece in the given position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PositionMove(pub usize, pub usize);

/// Error type for the implementation of [`TryIntoMove`] for [`PositionMove`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
