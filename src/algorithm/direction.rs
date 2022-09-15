use rand::{distributions::Standard, prelude::Distribution};
use std::fmt::{Display, Write};
use thiserror::Error;

/// The directions in which a piece can be moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    Up,
    Left,
    Down,
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

/// The error type for [`TryFrom<char>`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TryDirectionFromCharError {
    /// Found a character other than U, L, D, R.
    #[error("InvalidCharacter: character {0} must be 'U', 'L', 'D', or 'R'")]
    InvalidCharacter(char),
}

impl TryFrom<char> for Direction {
    type Error = TryDirectionFromCharError;

    /// Maps the characters U, L, D, R to directions.
    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'U' => Ok(Self::Up),
            'L' => Ok(Self::Left),
            'D' => Ok(Self::Down),
            'R' => Ok(Self::Right),
            _ => Err(TryDirectionFromCharError::InvalidCharacter(value)),
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
