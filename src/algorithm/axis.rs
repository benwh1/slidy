//! Defines the [`Axis`] type.

use rand::distr::{Distribution, StandardUniform};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::algorithm::direction::Direction;

/// The axes along which moves can be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Axis {
    /// The up/down axis
    Vertical,
    /// The left/right axis
    Horizontal,
}

impl Axis {
    /// Reflection in the main diagonal. Swaps the two axes.
    #[must_use]
    pub fn transpose(&self) -> Self {
        match self {
            Self::Vertical => Self::Horizontal,
            Self::Horizontal => Self::Vertical,
        }
    }
}

impl From<Direction> for Axis {
    fn from(value: Direction) -> Self {
        match value {
            Direction::Up | Direction::Down => Self::Vertical,
            Direction::Left | Direction::Right => Self::Horizontal,
        }
    }
}

impl Distribution<Axis> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Axis {
        match rng.random_range(0..2) {
            0 => Axis::Vertical,
            1 => Axis::Horizontal,
            _ => unreachable!(),
        }
    }
}
