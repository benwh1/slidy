//! Defines a struct representing the size of a [`SlidingPuzzle`].
//!
//! [`SlidingPuzzle`]: ../sliding_puzzle.html

use std::{
    fmt::{Display, Write},
    num::ParseIntError,
    str::FromStr,
};

use num_traits::AsPrimitive;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// The size of a [`SlidingPuzzle`].
///
/// [`SlidingPuzzle`]: ../sliding_puzzle.html
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Size(usize, usize);

impl Default for Size {
    fn default() -> Self {
        Self(4, 4)
    }
}

/// Error type for [`Size::new`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SizeError {
    /// Returned from [`Size::new`] when the width or height is less than 2.
    #[error("InvalidSize: width ({0}) and height ({1}) must be greater than or equal to 2")]
    InvalidSize(usize, usize),
}

impl Size {
    /// Creates a new [`Size`] with the given `width` and `height`.
    pub fn new(width: usize, height: usize) -> Result<Self, SizeError> {
        if width >= 2 && height >= 2 {
            Ok(Self(width, height))
        } else {
            Err(SizeError::InvalidSize(width, height))
        }
    }

    /// The width of the [`Size`].
    #[must_use]
    pub fn width(&self) -> usize {
        self.0
    }

    /// The height of the [`Size`].
    #[must_use]
    pub fn height(&self) -> usize {
        self.1
    }

    /// The product of the width and height.
    #[must_use]
    pub fn area(&self) -> usize {
        self.width() * self.height()
    }

    /// The number of pieces in a puzzle of this size. Equals `self.area() - 1`.
    #[must_use]
    pub fn num_pieces(&self) -> usize {
        self.area() - 1
    }

    /// Checks whether a position `(x, y)` is within bounds on a puzzle of this size.
    #[must_use]
    pub fn is_within_bounds(&self, (x, y): (usize, usize)) -> bool {
        x < self.width() && y < self.height()
    }

    /// The size of the transposed puzzle (width and height swapped).
    #[must_use]
    pub fn transpose(&self) -> Self {
        Self(self.1, self.0)
    }

    /// The square of size equal to the minimum of the width and height of `self`.
    #[must_use]
    pub fn shrink_to_square(&self) -> Self {
        let s = self.0.min(self.1);
        Self(s, s)
    }

    /// The square of size equal to the maximum of the width and height of `self`.self) -> Self {
    #[must_use]
    pub fn expand_to_square(&self) -> Self {
        let s = self.0.max(self.1);
        Self(s, s)
    }

    /// Returns true if the width and height are equal, and false otherwise.
    #[must_use]
    pub fn is_square(&self) -> bool {
        self.width() == self.height()
    }

    /// The number of solvable states of a puzzle of size `self`.
    #[must_use]
    pub fn num_states(&self) -> u128 {
        (1..=self.area().as_()).product::<u128>() / 2
    }
}

impl From<Size> for (usize, usize) {
    fn from(value: Size) -> Self {
        (value.width(), value.height())
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())?;
        f.write_char('x')?;
        f.write_str(&self.1.to_string())
    }
}

/// Error type for [`Size::from_str`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParseSizeError {
    /// The [`Size`] was not of the form `N` or `WxH`.
    #[error("ParseError: failed to parse size string")]
    ParseError,

    /// The width could not be parsed as an integer.
    #[error("ParseWidthError: {0}")]
    ParseWidthError(ParseIntError),

    /// The height could not be parsed as an integer.
    #[error("ParseWidthError: {0}")]
    ParseHeightError(ParseIntError),

    /// The width and height were both parsed as integers, but one of them was less than 2.
    #[error("SizeError: {0}")]
    SizeError(SizeError),
}

impl FromStr for Size {
    type Err = ParseSizeError;

    /// Parses a string into a [`Size`]. Acceptable formats are
    /// - `N` for some integer string `N`, representing a size where width and height are equal,
    /// - `WxH` for some integer strings `W` and `H`, representing the width and height.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(s) = s.parse::<usize>() {
            Self::new(s, s).map_err(ParseSizeError::SizeError)
        } else {
            let (w, h) = s.split_once('x').ok_or(ParseSizeError::ParseError)?;
            let (w, h) = (
                w.trim()
                    .parse::<usize>()
                    .map_err(ParseSizeError::ParseWidthError)?,
                h.trim()
                    .parse::<usize>()
                    .map_err(ParseSizeError::ParseHeightError)?,
            );
            Self::new(w, h).map_err(ParseSizeError::SizeError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        assert_eq!(Size::new(2, 2), Ok(Size(2, 2)));
        assert_eq!(Size::new(2, 3), Ok(Size(2, 3)));
        assert_eq!(Size::new(3, 2), Ok(Size(3, 2)));
        assert_eq!(Size::new(3, 3), Ok(Size(3, 3)));
        assert_eq!(Size::new(1, 1), Err(SizeError::InvalidSize(1, 1)));
        assert_eq!(Size::new(1, 2), Err(SizeError::InvalidSize(1, 2)));
        assert_eq!(Size::new(2, 1), Err(SizeError::InvalidSize(2, 1)));
        assert_eq!(Size::new(0, 0), Err(SizeError::InvalidSize(0, 0)));
        assert_eq!(Size::new(0, 1), Err(SizeError::InvalidSize(0, 1)));
        assert_eq!(Size::new(1, 0), Err(SizeError::InvalidSize(1, 0)));
    }

    #[test]
    fn test_width() {
        assert_eq!(Size::new(2, 3).unwrap().width(), 2);
    }

    #[test]
    fn test_height() {
        assert_eq!(Size::new(2, 3).unwrap().height(), 3);
    }

    #[test]
    fn test_area() {
        assert_eq!(Size::new(2, 3).unwrap().area(), 6);
    }

    #[test]
    fn test_num_pieces() {
        assert_eq!(Size::new(2, 3).unwrap().num_pieces(), 5);
    }

    #[test]
    fn test_is_within_bounds() {
        assert_eq!(Size::new(2, 3).unwrap().is_within_bounds((0, 0)), true);
        assert_eq!(Size::new(2, 3).unwrap().is_within_bounds((1, 2)), true);
        assert_eq!(Size::new(2, 3).unwrap().is_within_bounds((2, 3)), false);
        assert_eq!(Size::new(2, 3).unwrap().is_within_bounds((3, 2)), false);
    }

    #[test]
    fn test_transpose() {
        assert_eq!(Size::new(2, 3).unwrap().transpose(), Size(3, 2));
    }

    #[test]
    fn test_shrink_to_square() {
        assert_eq!(Size::new(2, 5).unwrap().shrink_to_square(), Size(2, 2));
        assert_eq!(Size::new(5, 2).unwrap().shrink_to_square(), Size(2, 2));
        assert_eq!(Size::new(5, 5).unwrap().shrink_to_square(), Size(5, 5));
    }

    #[test]
    fn test_expand_to_square() {
        assert_eq!(Size::new(2, 5).unwrap().expand_to_square(), Size(5, 5));
        assert_eq!(Size::new(5, 2).unwrap().expand_to_square(), Size(5, 5));
        assert_eq!(Size::new(5, 5).unwrap().expand_to_square(), Size(5, 5));
    }

    #[test]
    fn test_is_square() {
        assert_eq!(Size::new(2, 5).unwrap().is_square(), false);
        assert_eq!(Size::new(5, 2).unwrap().is_square(), false);
        assert_eq!(Size::new(5, 5).unwrap().is_square(), true);
    }

    #[test]
    fn test_num_states() {
        assert_eq!(Size::new(2, 2).unwrap().num_states(), 12);
        assert_eq!(Size::new(2, 3).unwrap().num_states(), 360);
        assert_eq!(Size::new(3, 2).unwrap().num_states(), 360);
        assert_eq!(Size::new(3, 3).unwrap().num_states(), 181440);
        assert_eq!(Size::new(4, 4).unwrap().num_states(), 10461394944000);
        assert_eq!(
            Size::new(5, 5).unwrap().num_states(),
            7755605021665492992000000
        );
        assert_eq!(
            Size::new(17, 2).unwrap().num_states(),
            147616399519802070423809304821760000000
        );
    }

    #[test]
    fn test_into_usize_usize() {
        let (w, h) = Size(2, 3).into();
        assert_eq!((w, h), (2, 3));
    }

    #[test]
    fn test_display() {
        assert_eq!(Size(2, 3).to_string(), "2x3");
    }

    #[test]
    fn test_parse() {
        assert_eq!("2x3".parse::<Size>(), Ok(Size(2, 3)));
        assert_eq!(" 2x3 ".parse::<Size>(), Ok(Size(2, 3)));
        assert_eq!("2 x 3".parse::<Size>(), Ok(Size(2, 3)));
        assert_eq!(" 2 x3 ".parse::<Size>(), Ok(Size(2, 3)));
        assert_eq!("2".parse::<Size>(), Ok(Size(2, 2)));
        assert_eq!(" 2 ".parse::<Size>(), Ok(Size(2, 2)));
    }
}
