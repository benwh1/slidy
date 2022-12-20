//! Defines the [`ColorScheme`] trait and an implementation, as well as a recursive color scheme.

pub mod tiled;

use blanket::blanket;
use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{coloring::Coloring, label::label::Label};

/// Error type for [`ColorScheme`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ColorSchemeError {
    /// Returned when the given puzzle size is incompatible with the label.
    #[error("InvalidSize: {width}x{height} is not a valid size")]
    InvalidSize {
        /// Width of the puzzle.
        width: usize,
        /// Height of the puzzle.
        height: usize,
    },

    /// Returned when the `(x, y)` position is outside the bounds of the puzzle.
    #[error(
        "PositionOutOfBounds: position ({x}, {y}) is out of bounds on a {width}x{height} puzzle."
    )]
    PositionOutOfBounds {
        /// Width of the puzzle.
        width: usize,
        /// Height of the puzzle.
        height: usize,
        /// x coordinate of the position.
        x: usize,
        /// y coordinate of the position.
        y: usize,
    },
}

/// Provides a function mapping `(x, y)` coordinate on a puzzle to a color.
#[blanket(derive(Ref, Rc, Arc, Mut, Box))]
pub trait ColorScheme {
    /// Checks if this `ColorScheme` can be used with a given puzzle size.
    #[must_use]
    fn is_valid_size(&self, width: usize, height: usize) -> bool;

    /// See [`Self::color`].
    ///
    /// This function may not check whether `width x height` is a valid puzzle size for the color
    /// scheme, or whether `(x, y)` is within the bounds of the puzzle. If these conditions are not
    /// satisfied, the function may panic or return any other color.
    #[must_use]
    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba;

    /// Returns the color of `(x, y)` on a `width x height` puzzle.
    fn try_color(
        &self,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> Result<Rgba, ColorSchemeError> {
        if !self.is_valid_size(width, height) {
            Err(ColorSchemeError::InvalidSize { width, height })
        } else if x >= width || y >= height {
            Err(ColorSchemeError::PositionOutOfBounds {
                width,
                height,
                x,
                y,
            })
        } else {
            Ok(self.color(width, height, x, y))
        }
    }
}

/// A color scheme formed by composing a [`Label`] and a [`Coloring`].
pub struct Scheme<L: Label, C: Coloring> {
    label: L,
    coloring: C,
}

impl<L: Label, C: Coloring> Scheme<L, C> {
    /// Create a new [`Scheme`] from a [`Label`] and a [`Coloring`].
    #[must_use]
    pub fn new(label: L, coloring: C) -> Self {
        Self { label, coloring }
    }
}

impl<L: Label, C: Coloring> ColorScheme for Scheme<L, C> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.label.is_valid_size(width, height)
    }

    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        let label = self.label.position_label(width, height, x, y);
        let num_labels = self.label.num_labels(width, height);
        self.coloring.color(label, num_labels)
    }
}

/// A list of [`ColorScheme`]s and an index, indicating which color scheme is currently "active".
/// The implementation of [`ColorScheme`] for this type uses the active scheme.
pub struct SchemeList<'a, S: ColorScheme> {
    schemes: &'a [S],
    index: usize,
}

/// Error type for [`SchemeList`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SchemeListError {
    /// Returned from [`SchemeList::new`] if the list of schemes is empty.
    #[error("Empty: list of schemes must be non-empty")]
    Empty,
}

impl<'a, S: ColorScheme> SchemeList<'a, S> {
    /// Create a new [`SchemeList`] containing the given list of color schemes. The default index
    /// is 0.
    pub fn new(schemes: &'a [S]) -> Result<Self, SchemeListError> {
        if schemes.is_empty() {
            Err(SchemeListError::Empty)
        } else {
            Ok(Self { schemes, index: 0 })
        }
    }

    /// Increments the index by 1. Returns true if the index changed, or false if the last scheme
    /// was already active.
    pub fn increment_index(&mut self) -> bool {
        if self.index < self.schemes.len() - 1 {
            self.index += 1;
            true
        } else {
            false
        }
    }

    /// Decrements the index by 1. Returns true if the index changed, or false if the first scheme
    /// was already active.
    pub fn decrement_index(&mut self) -> bool {
        if self.index > 0 {
            self.index -= 1;
            true
        } else {
            false
        }
    }

    /// Returns a reference to the scheme that is currently active.
    #[must_use]
    pub fn current_scheme(&self) -> &'a S {
        &self.schemes[self.index]
    }

    /// Returns a reference to the scheme after the one that is currently active, or `None` if the
    /// active scheme is the last one.
    #[must_use]
    pub fn subscheme(&self) -> Option<&'a S> {
        if self.index + 1 < self.schemes.len() {
            Some(&self.schemes[self.index + 1])
        } else {
            None
        }
    }
}

impl<'a, S: ColorScheme> ColorScheme for SchemeList<'a, S> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.schemes[self.index].is_valid_size(width, height)
    }

    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        self.schemes[self.index].color(width, height, x, y)
    }
}
