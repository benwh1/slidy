//! Defines the [`Direction`] type.

use std::{
    fmt::{Display, Write as _},
    str::FromStr,
};

use rand::{
    distr::{Distribution, StandardUniform},
    Rng, RngExt as _,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::algorithm::axis::Axis;

/// The directions in which a piece can be moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Direction {
    /// Moving a piece below the gap upward.
    Up = 0,
    /// Moving a piece right of the gap to the left.
    Left = 1,
    /// Moving a piece above the gap downward.
    Down = 2,
    /// Moving a piece left of the gap to the right.
    Right = 3,
}

impl Direction {
    /// The opposite direction. Swaps `Up` with `Down` and `Left` with `Right`.
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

    /// Reflection through the vertical axis. Swaps `Left` with `Right`.
    #[must_use]
    pub fn reflect_left_right(&self) -> Self {
        match self {
            Self::Up => Self::Up,
            Self::Left => Self::Right,
            Self::Down => Self::Down,
            Self::Right => Self::Left,
        }
    }

    /// Reflection through the horizontal axis. Swaps `Up` with `Down`.
    #[must_use]
    pub fn reflect_up_down(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Left => Self::Left,
            Self::Down => Self::Up,
            Self::Right => Self::Right,
        }
    }

    /// Returns the [`Axis`] along which this [`Direction`] moves.
    #[must_use]
    pub fn axis(&self) -> Axis {
        match self {
            Self::Up | Self::Down => Axis::Vertical,
            Self::Left | Self::Right => Axis::Horizontal,
        }
    }

    /// The standard name of the direction as a character.
    ///
    /// Returns 'U', 'L', 'D', 'R' for `Up`, `Left`, `Down`, `Right` respectively.
    #[must_use]
    pub fn as_char(&self) -> char {
        match self {
            Self::Up => 'U',
            Self::Left => 'L',
            Self::Down => 'D',
            Self::Right => 'R',
        }
    }
}

impl Display for Direction {
    /// Formats the direction as an upper case character: U, L, D, R.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(self.as_char())
    }
}

/// Error type for [`TryFrom<char>`] and [`Direction::from_str`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ParseDirectionError {
    /// Found a character other than U, L, D, R.
    #[error("InvalidCharacter: character {0} must be one of 'U', 'L', 'D', 'R'")]
    InvalidCharacter(char),

    /// The string is empty.
    #[error("Empty: string is empty")]
    Empty,

    /// The string contains more than one character.
    #[error("MultipleCharacters: string must contain exactly one character")]
    MultipleCharacters,
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
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (None, _) => Err(Self::Err::Empty),
            (Some(c), None) => c.try_into(),
            (Some(_), Some(_)) => Err(Self::Err::MultipleCharacters),
        }
    }
}

impl Distribution<Direction> for StandardUniform {
    fn sample<R>(&self, rng: &mut R) -> Direction
    where
        R: Rng + ?Sized,
    {
        match rng.random_range(0..4) {
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
    use std::str::FromStr as _;

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

    #[test]
    fn test_from_str_all_directions() {
        for d in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            assert_eq!(Direction::from_str(&d.to_string()), Ok(d));
        }
        assert_eq!(
            Direction::from_str("u"),
            Err(ParseDirectionError::InvalidCharacter('u'))
        );
    }

    #[test]
    fn test_from_str_multiple_characters() {
        assert_eq!(
            Direction::from_str("UD"),
            Err(ParseDirectionError::MultipleCharacters)
        );
        assert_eq!(
            Direction::from_str("U "),
            Err(ParseDirectionError::MultipleCharacters)
        );
        assert_eq!(
            Direction::from_str("xU"),
            Err(ParseDirectionError::MultipleCharacters)
        );
    }
}
