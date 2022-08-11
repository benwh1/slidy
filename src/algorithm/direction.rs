use rand::{distributions::Standard, prelude::Distribution};
use std::fmt::Display;
use thiserror::Error;

use crate::algorithm::puzzle_move::Move;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    Up,
    Left,
    Down,
    Right,
}

impl Direction {
    pub fn inverse(&self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Left => Self::Right,
            Self::Down => Self::Up,
            Self::Right => Self::Left,
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Up => "U",
                Self::Left => "L",
                Self::Down => "D",
                Self::Right => "R",
            }
        )
    }
}

impl Into<Move> for Direction {
    fn into(self) -> Move {
        Move {
            direction: self,
            amount: 1,
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TryDirectionFromCharError {
    #[error("InvalidCharacter: character {0} must be 'U', 'L', 'D', or 'R'")]
    InvalidCharacter(char),
}

impl TryFrom<char> for Direction {
    type Error = TryDirectionFromCharError;

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
