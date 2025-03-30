//! Defines the [`ColorScheme`] trait.

pub mod balanced_split;
pub mod multi_layer;
pub mod scheme_list;
pub mod tiled;

use blanket::blanket;
use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{coloring::Coloring, label::label::Label, size::Size};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Error type for [`ColorScheme`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColorSchemeError {
    /// Returned when the `(x, y)` position is outside the bounds of the puzzle.
    #[error("PositionOutOfBounds: position {pos:?} is out of bounds on a {size} puzzle")]
    PositionOutOfBounds {
        /// Size of the puzzle.
        size: Size,
        /// Piece position.
        pos: (u64, u64),
    },
}

/// Provides a function mapping `(x, y)` coordinate on a puzzle to a color.
#[blanket(derive(Ref, Rc, Arc, Mut))]
pub trait ColorScheme {
    /// See [`ColorScheme::try_color`].
    ///
    /// This function may not check whether `pos` is within the bounds of the puzzle. If this
    /// condition is not satisfied, the function may panic or return any other color.
    #[must_use]
    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba;

    /// Returns the color of the piece in position `pos` on a solved puzzle of the given size.
    fn try_color(&self, size: Size, pos: (u64, u64)) -> Result<Rgba, ColorSchemeError> {
        if size.is_within_bounds(pos) {
            Ok(self.color(size, pos))
        } else {
            Err(ColorSchemeError::PositionOutOfBounds { size, pos })
        }
    }
}

/// A [`ColorScheme`] that is defined for a puzzle of a single size.
#[blanket(derive(Ref, Rc, Arc, Mut))]
pub trait FixedSizeColorScheme {
    /// Returns the [`Size`] of the puzzle that this label is defined for.
    #[must_use]
    fn size(&self) -> Size;

    /// See [`ColorScheme::color`].
    #[must_use]
    fn color(&self, pos: (u64, u64)) -> Rgba;

    /// See [`ColorScheme::try_color`].
    fn try_color(&self, pos: (u64, u64)) -> Result<Rgba, ColorSchemeError> {
        let size = self.size();

        if size.is_within_bounds(pos) {
            Ok(self.color(pos))
        } else {
            Err(ColorSchemeError::PositionOutOfBounds { size, pos })
        }
    }
}

impl<T: ColorScheme + ?Sized> ColorScheme for Box<T> {
    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        (**self).color(size, pos)
    }
}

impl<T: FixedSizeColorScheme + ?Sized> FixedSizeColorScheme for Box<T> {
    fn size(&self) -> Size {
        (**self).size()
    }

    fn color(&self, pos: (u64, u64)) -> Rgba {
        (**self).color(pos)
    }
}

/// A [`ColorScheme`] restricted to a single size.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FixedSize<C: ColorScheme> {
    scheme: C,
    size: Size,
}

impl<C: ColorScheme> FixedSizeColorScheme for FixedSize<C> {
    fn size(&self) -> Size {
        self.size
    }

    fn color(&self, pos: (u64, u64)) -> Rgba {
        self.scheme.color(self.size, pos)
    }
}

/// A color scheme formed by composing a [`Label`] and a [`Coloring`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    /// The [`Label`] of the [`Scheme`].
    #[must_use]
    pub fn label(&self) -> &L {
        &self.label
    }

    /// The [`Coloring`] of the [`Scheme`].
    #[must_use]
    pub fn coloring(&self) -> &C {
        &self.coloring
    }
}

impl<L: Label, C: Coloring> ColorScheme for Scheme<L, C> {
    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        let label = self.label.position_label(size, pos);
        let num_labels = self.label.num_labels(size);
        self.coloring.color(label, num_labels)
    }
}

/// A [`ColorScheme`] that always outputs black. This is just to make using [`Renderer`] more
/// convenient (because most of the time, we probably want black text and black borders).
///
/// [`Renderer`]: ../renderer.html
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Black;

impl ColorScheme for Black {
    fn color(&self, _size: Size, _pos: (u64, u64)) -> Rgba {
        Rgba::new(0.0, 0.0, 0.0, 1.0)
    }
}
