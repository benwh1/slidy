//! Defines the [`ColorScheme`] trait and an implementation, as well as a recursive color scheme.

pub mod tiled;

use std::marker::PhantomData;

use blanket::blanket;
use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{coloring::Coloring, label::label::Label, size::Size};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Error type for [`ColorScheme`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColorSchemeError {
    /// Returned when the given puzzle size is incompatible with the label.
    #[error("InvalidSize: {0} is not a valid size")]
    InvalidSize(Size),

    /// Returned when the `(x, y)` position is outside the bounds of the puzzle.
    #[error("PositionOutOfBounds: position {pos:?} is out of bounds on a {size} puzzle.")]
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
    /// Checks if this `ColorScheme` can be used with a given puzzle size.
    #[must_use]
    fn is_valid_size(&self, size: Size) -> bool;

    /// See [`ColorScheme::try_color`].
    ///
    /// This function may not check whether `size` is a valid puzzle size for the color scheme, or
    /// whether `pos` is within the bounds of the puzzle. If these conditions are not satisfied,
    /// the function may panic or return any other color.
    #[must_use]
    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba;

    /// Returns the color of the piece in position `pos` on a solved puzzle of the given size.
    fn try_color(&self, size: Size, pos: (u64, u64)) -> Result<Rgba, ColorSchemeError> {
        if !self.is_valid_size(size) {
            Err(ColorSchemeError::InvalidSize(size))
        } else if !size.is_within_bounds(pos) {
            Err(ColorSchemeError::PositionOutOfBounds { size, pos })
        } else {
            Ok(self.color(size, pos))
        }
    }
}

impl<T: ColorScheme + ?Sized> ColorScheme for Box<T> {
    fn is_valid_size(&self, size: Size) -> bool {
        (**self).is_valid_size(size)
    }

    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        (**self).color(size, pos)
    }
}

/// A color scheme formed by composing a [`Label`] and a [`Coloring`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    fn is_valid_size(&self, size: Size) -> bool {
        self.label.is_valid_size(size)
    }

    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        let label = self.label.position_label(size, pos);
        let num_labels = self.label.num_labels(size);
        self.coloring.color(label, num_labels)
    }
}

/// A list of [`ColorScheme`]s and an index, indicating which color scheme is currently "active".
/// The implementation of [`ColorScheme`] for this type uses the active scheme.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SchemeList<S: ColorScheme, List: AsRef<[S]>> {
    schemes: List,
    index: usize,
    phantom_s: PhantomData<S>,
}

/// Error type for [`SchemeList`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SchemeListError {
    /// Returned from [`SchemeList::new`] if the list of schemes is empty.
    #[error("Empty: list of schemes must be non-empty")]
    Empty,
}

impl<S: ColorScheme, List: AsRef<[S]>> SchemeList<S, List> {
    /// Create a new [`SchemeList`] containing the given list of color schemes. The default index
    /// is 0.
    pub fn new(schemes: List) -> Result<Self, SchemeListError> {
        if schemes.as_ref().is_empty() {
            Err(SchemeListError::Empty)
        } else {
            Ok(Self {
                schemes,
                index: 0,
                phantom_s: PhantomData,
            })
        }
    }

    /// Returns a reference to the list of schemes.
    pub fn schemes(&self) -> &[S] {
        self.schemes.as_ref()
    }

    /// Increments the index by 1. Returns true if the index changed, or false if the last scheme
    /// was already active.
    pub fn increment_index(&mut self) -> bool {
        if self.index < self.schemes.as_ref().len() - 1 {
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

    /// Returns the index of the currently active scheme.
    #[must_use]
    pub fn current_scheme_index(&self) -> usize {
        self.index
    }

    /// Returns the number of schemes in the list.
    #[must_use]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.schemes.as_ref().len()
    }

    /// Returns a reference to the scheme that is currently active.
    #[must_use]
    pub fn current_scheme(&self) -> &S {
        &self.schemes.as_ref()[self.index]
    }

    /// Returns a reference to the scheme after the one that is currently active, or `None` if the
    /// active scheme is the last one.
    #[must_use]
    pub fn subscheme(&self) -> Option<&S> {
        if self.index + 1 < self.schemes.as_ref().len() {
            Some(&self.schemes.as_ref()[self.index + 1])
        } else {
            None
        }
    }
}

impl<S: ColorScheme, List: AsRef<[S]>> ColorScheme for SchemeList<S, List> {
    fn is_valid_size(&self, size: Size) -> bool {
        self.schemes.as_ref()[self.index].is_valid_size(size)
    }

    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        self.schemes.as_ref()[self.index].color(size, pos)
    }
}

/// A [`ColorScheme`] that always outputs black. This is just to make using [`Renderer`] more
/// convenient (because most of the time, we probably want black text and black borders).
///
/// [`Renderer`]: ../renderer.html
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Black;

impl ColorScheme for Black {
    fn is_valid_size(&self, _size: Size) -> bool {
        true
    }

    fn color(&self, _size: Size, _pos: (u64, u64)) -> Rgba {
        Rgba::new(0.0, 0.0, 0.0, 1.0)
    }
}
