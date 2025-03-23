//! Defines the [`Move`] type.

use std::{cmp::Ordering, fmt::Display, num::ParseIntError, ops::Add, str::FromStr};

use thiserror::Error;

use crate::algorithm::{
    as_slice::AsAlgorithmSlice,
    direction::{Direction, ParseDirectionError},
    display::r#move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort, MoveDisplay as _},
    slice::AlgorithmSlice,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// A (possibly multi-tile) move of a puzzle. Contains a direction and an amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Move {
    pub(crate) direction: Direction,
    pub(crate) amount: u64,
}

/// Represents the sum of two moves.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MoveSum {
    /// The sum of two moves that are in the same or opposite directions is another move.
    Ok(Move),

    /// If two moves are not in the same or opposite directions, they can not be added.
    Invalid,
}

/// Error type for [`Move`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MoveError {
    /// Returned when the amount 0 is passed to [`Move::new_nonzero`].
    #[error("ZeroAmount: move amount must be greater than 0")]
    ZeroAmount,
}

impl Move {
    /// Creates a new [`Move`].
    #[must_use]
    pub fn new(direction: Direction, amount: u64) -> Self {
        Self { direction, amount }
    }

    /// Creates a new [`Move`] with the requirement that `amount` must be non-zero.
    pub fn new_nonzero(direction: Direction, amount: u64) -> Result<Self, MoveError> {
        if amount == 0 {
            Err(MoveError::ZeroAmount)
        } else {
            Ok(Self { direction, amount })
        }
    }

    /// Returns the direction of the move.
    #[must_use]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Returns the number of pieces moved.
    #[must_use]
    pub fn amount(&self) -> u64 {
        self.amount
    }

    /// Returns the inverse of a move. This is given by taking the inverse of the direction and
    /// leaving the amount unchanged.
    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            direction: self.direction.inverse(),
            amount: self.amount,
        }
    }

    /// Returns the transpose of a move. This is given by taking the transpose of the direction and
    /// leaving the amount unchanged.
    #[must_use]
    pub fn transpose(&self) -> Self {
        Self {
            direction: self.direction.transpose(),
            amount: self.amount,
        }
    }

    /// Helper function for creating a [`DisplayLongSpaced`] around `self`.
    #[must_use]
    pub fn display_long_spaced(&self) -> DisplayLongSpaced {
        DisplayLongSpaced::new(*self)
    }

    /// Helper function for creating a [`DisplayLongUnspaced`] around `self`.
    #[must_use]
    pub fn display_long_unspaced(&self) -> DisplayLongUnspaced {
        DisplayLongUnspaced::new(*self)
    }

    /// Helper function for creating a [`DisplayShort`] around `self`.
    #[must_use]
    pub fn display_short(&self) -> DisplayShort {
        DisplayShort::new(*self)
    }
}

impl Display for Move {
    /// Uses [`Move::display_short`] as the default formatting.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display_short().fmt(f)
    }
}

/// Error type for [`Move::from_str`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParseMoveError {
    /// Failed to parse the direction.
    #[error("ParseDirectionError: {0}")]
    ParseDirectionError(ParseDirectionError),

    /// Failed to parse the amount.
    #[error("ParseIntError: {0}")]
    ParseIntError(ParseIntError),

    /// The string is empty.
    #[error("Empty: input string is empty")]
    Empty,
}

impl FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();

        let direction = chars
            .next()
            .ok_or(Self::Err::Empty)?
            .try_into()
            .map_err(Self::Err::ParseDirectionError)?;

        let rest = chars.as_str();
        let amount = if rest.is_empty() {
            1
        } else {
            rest.parse().map_err(Self::Err::ParseIntError)?
        };

        Ok(Self::new(direction, amount))
    }
}

impl From<Direction> for Move {
    /// Creates a [`Move`] from a [`Direction`] with `amount = 1`.
    fn from(direction: Direction) -> Self {
        Self {
            direction,
            amount: 1,
        }
    }
}

impl Add for Move {
    type Output = MoveSum;

    fn add(self, rhs: Self) -> Self::Output {
        if self.direction == rhs.direction {
            MoveSum::Ok(Self {
                direction: self.direction,
                amount: self.amount + rhs.amount,
            })
        } else if self.direction == rhs.direction.inverse() {
            match self.amount.cmp(&rhs.amount) {
                Ordering::Less => MoveSum::Ok(Self {
                    direction: rhs.direction,
                    amount: rhs.amount - self.amount,
                }),
                Ordering::Equal | Ordering::Greater => MoveSum::Ok(Self {
                    direction: self.direction,
                    amount: self.amount - rhs.amount,
                }),
            }
        }
        // Even if the directions are not on the same axis, we can still add the moves if one of
        // them has amount == 0 (in which case the sum is just the other move).
        // Put the check for rhs.amount == 0 first so that if they are both 0, we return self
        // instead of rhs.
        else if rhs.amount == 0 {
            MoveSum::Ok(self)
        } else if self.amount == 0 {
            MoveSum::Ok(rhs)
        } else {
            MoveSum::Invalid
        }
    }
}

impl AsAlgorithmSlice<'_> for Move {
    fn as_slice(&self) -> AlgorithmSlice<'_> {
        AlgorithmSlice {
            first: Some(*self),
            middle: &[],
            last: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let a = Move::new(Direction::Up, 2);
        assert_eq!(
            a,
            Move {
                direction: Direction::Up,
                amount: 2
            }
        );
    }

    #[test]
    fn test_new_2() {
        let a = Move::new(Direction::Up, 0);
        assert_eq!(
            a,
            Move {
                direction: Direction::Up,
                amount: 0
            }
        );
    }

    #[test]
    fn test_new_nonzero() {
        let a = Move::new_nonzero(Direction::Up, 2);
        assert_eq!(
            a,
            Ok(Move {
                direction: Direction::Up,
                amount: 2
            })
        );
    }

    #[test]
    fn test_new_nonzero_2() {
        let a = Move::new_nonzero(Direction::Up, 0);
        assert_eq!(a, Err(MoveError::ZeroAmount));
    }

    #[test]
    fn test_inverse() {
        let a = Move::new(Direction::Up, 3);
        let b = Move::new(Direction::Down, 3);
        assert_eq!(a.inverse(), b);
    }

    #[test]
    fn test_transpose() {
        let a = Move::new(Direction::Up, 3);
        let b = Move::new(Direction::Left, 3);
        assert_eq!(a.transpose(), b);
    }

    mod from_direction {
        use super::*;

        #[test]
        fn test_from_direction() {
            assert_eq!(Move::from(Direction::Up), Move::new(Direction::Up, 1));
        }
    }

    mod add {
        use super::*;

        #[test]
        fn test_add() {
            let m1 = Move::new(Direction::Up, 3);
            let m2 = Move::new(Direction::Up, 4);
            assert_eq!(
                m1 + m2,
                MoveSum::Ok(Move {
                    direction: Direction::Up,
                    amount: 7
                })
            );
        }

        #[test]
        fn test_add_2() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 3,
            };
            let m2 = Move {
                direction: Direction::Down,
                amount: 4,
            };
            assert_eq!(
                m1 + m2,
                MoveSum::Ok(Move {
                    direction: Direction::Down,
                    amount: 1
                })
            );
        }

        #[test]
        fn test_add_3() {
            let m1 = Move {
                direction: Direction::Left,
                amount: 5,
            };
            let m2 = Move {
                direction: Direction::Right,
                amount: 5,
            };
            assert_eq!(
                m1 + m2,
                MoveSum::Ok(Move {
                    direction: Direction::Left,
                    amount: 0
                })
            );
        }

        #[test]
        fn test_add_4() {
            let m1 = Move {
                direction: Direction::Left,
                amount: 2,
            };
            let m2 = Move {
                direction: Direction::Up,
                amount: 1,
            };
            assert_eq!(m1 + m2, MoveSum::Invalid);
        }

        #[test]
        fn test_add_5() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 0,
            };
            let m2 = Move {
                direction: Direction::Down,
                amount: 2,
            };
            assert_eq!(m1 + m2, MoveSum::Ok(m2));
            assert_eq!(m2 + m1, MoveSum::Ok(m2));
        }

        #[test]
        fn test_add_6() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 0,
            };
            let m2 = Move {
                direction: Direction::Left,
                amount: 2,
            };
            assert_eq!(m1 + m2, MoveSum::Ok(m2));
            assert_eq!(m2 + m1, MoveSum::Ok(m2));
        }

        #[test]
        fn test_add_7() {
            let m1 = Move {
                direction: Direction::Up,
                amount: 0,
            };
            let m2 = Move {
                direction: Direction::Left,
                amount: 0,
            };
            assert_eq!(m1 + m2, MoveSum::Ok(m1));
            assert_eq!(m2 + m1, MoveSum::Ok(m2));
        }
    }
}

#[cfg(all(feature = "nightly", test))]
mod benchmarks {
    extern crate test;

    use test::Bencher;

    use super::*;

    #[bench]
    fn bench_display_long_spaced(b: &mut Bencher) {
        let m = Move::new(Direction::Up, 10);
        b.iter(|| {
            for _ in 0..100 {
                DisplayLongSpaced::new(m).to_string();
            }
        });
    }

    #[bench]
    fn bench_display_long_unspaced(b: &mut Bencher) {
        let m = Move::new(Direction::Up, 10);
        b.iter(|| {
            for _ in 0..100 {
                DisplayLongUnspaced::new(m).to_string();
            }
        });
    }

    #[bench]
    fn bench_display_short(b: &mut Bencher) {
        let m = Move::new(Direction::Up, 10);
        b.iter(|| {
            for _ in 0..100 {
                DisplayShort::new(m).to_string();
            }
        });
    }
}
