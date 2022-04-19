use super::{direction::Direction, display::puzzle_move::DisplayMove};
use std::{cmp::Ordering, ops::Add};
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MoveSum {
    Ok(Move),
    Invalid,
    Empty,
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

impl Add for Move {
    type Output = MoveSum;

    fn add(self, rhs: Self) -> Self::Output {
        if self.direction == rhs.direction {
            MoveSum::Ok(Move {
                direction: self.direction,
                amount: self.amount + rhs.amount,
            })
        } else if self.direction == rhs.direction.inverse() {
            match self.amount.cmp(&rhs.amount) {
                Ordering::Less => MoveSum::Ok(Move {
                    direction: rhs.direction,
                    amount: rhs.amount - self.amount,
                }),
                Ordering::Equal => MoveSum::Empty,
                Ordering::Greater => MoveSum::Ok(Move {
                    direction: self.direction,
                    amount: self.amount - rhs.amount,
                }),
            }
        } else {
            MoveSum::Invalid
        }
    }
}
