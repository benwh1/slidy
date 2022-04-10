use super::{direction::Direction, display::puzzle_move::DisplayMove};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Move {
    pub direction: Direction,
    pub amount: u32,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MoveError {
    #[error("InvalidAmount: `amount` ({0}) must be greater than 0")]
    InvalidAmount(u32),
}

impl Move {
    pub fn new(direction: Direction, amount: u32) -> Result<Self, MoveError> {
        if amount > 0 {
            Ok(Self { direction, amount })
        } else {
            Err(MoveError::InvalidAmount(amount))
        }
    }

    pub fn inverse(&self) -> Self {
        Self {
            direction: self.direction.inverse(),
            amount: self.amount,
        }
    }

    pub fn display<T>(&self) -> DisplayMove<'_, T> {
        DisplayMove::<T>::new(self)
    }
}
