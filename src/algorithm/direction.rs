//! Defines the [`Direction`] type.

use rand::{distributions::Standard, prelude::Distribution};
use std::{
    fmt::{Display, Write},
    str::FromStr,
};
use thiserror::Error;

/// The directions in which a piece can be moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    /// Moving a piece below the gap upwards.
    Up,
    /// Moving a piece right of the gap to the left.
    Left,
    /// Moving a piece above the gap downwards.
    Down,
    /// Moving a piece left of the gap to the right.
    Right,
}

impl Direction {
    /// The opposite directon. Swaps `Up` with `Down` and `Left` with `Right`.
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Left => Self::Right,
            Self::Down => Self::Up,
            Self::Right => Self::Left,
        }
    }

    /// Reflection in the main diagonal. Swaps `Up` with `Left` and `Down` with `Right`.
    #[must_use]
    pub fn transpose(&self) -> Self {
        match self {
            Self::Up => Self::Left,
            Self::Left => Self::Up,
            Self::Down => Self::Right,
            Self::Right => Self::Down,
        }
    }
}

impl Display for Direction {
    /// Formats the direction as an upper case character: U, L, D, R
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(match self {
            Self::Up => 'U',
            Self::Left => 'L',
            Self::Down => 'D',
            Self::Right => 'R',
        })
    }
}

/// Error type for [`TryFrom<char>`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParseDirectionError {
    /// Found a character other than U, L, D, R.
    #[error("InvalidCharacter: character {0} must be one of 'U', 'L', 'D', 'R'")]
    InvalidCharacter(char),

    /// The string is empty.
    #[error("Empty: string is empty")]
    Empty,
}

impl TryFrom<char> for Direction {
    type Error = ParseDirectionError;

    /// Maps the characters 'U', 'L', 'D', 'R' to directions.
    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'U' => Ok(Self::Up),
            'L' => Ok(Self::Left),
            'D' => Ok(Self::Down),
            'R' => Ok(Self::Right),
            _ => Err(Self::Error::InvalidCharacter(value)),
        }
    }
}

impl FromStr for Direction {
    type Err = ParseDirectionError;

    /// Maps the single-character strings "U", "L", "D", "R" to directions.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "U" => Ok(Self::Up),
            "L" => Ok(Self::Left),
            "D" => Ok(Self::Down),
            "R" => Ok(Self::Right),
            _ => Err(match s.chars().next() {
                Some(c) => Self::Err::InvalidCharacter(c),
                None => Self::Err::Empty,
            }),
        }
    }
}

impl Distribution<Direction> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..4) {
            0 => Direction::Up,
            1 => Direction::Left,
            2 => Direction::Down,
            3 => Direction::Right,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::algorithm::direction::{Direction, ParseDirectionError};

    #[test]
    fn test_from_str() {
        assert_eq!(Direction::from_str("U"), Ok(Direction::Up));
        assert_eq!(
            Direction::from_str("🥚"),
            Err(ParseDirectionError::InvalidCharacter('🥚'))
        );
        assert_eq!(Direction::from_str(""), Err(ParseDirectionError::Empty));
    }
}
