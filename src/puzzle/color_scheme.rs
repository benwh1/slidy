//! Defines the [`ColorScheme`] trait and an implementation, as well as a recursive color scheme.

use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{
    coloring::Coloring,
    label::{label::Label, rect_partition::RectPartition},
};

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
pub struct Scheme {
    label: Box<dyn Label>,
    coloring: Box<dyn Coloring>,
}

impl Scheme {
    /// Create a new [`Scheme`] from a [`Label`] and a [`Coloring`].
    #[must_use]
    pub fn new(label: Box<dyn Label>, coloring: Box<dyn Coloring>) -> Self {
        Self { label, coloring }
    }
}

impl ColorScheme for Scheme {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.label.is_valid_size(width, height)
    }

    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        let label = self.label.position_label(width, height, x, y);
        let num_labels = self.label.num_labels(width, height);
        self.coloring.color(label, num_labels)
    }
}

/// Error type for [`RecursiveScheme`]
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecursiveSchemeError {
    /// Returned when the number of rectangles in the partition is not equal to the number of
    /// subschemes.
    #[error("IncompatiblePartitionAndSubschemes: partition has {num_rects} rects, but {num_subschemes} subschemes were given")]
    IncompatiblePartitionAndSubschemes {
        /// Number of rectangles in the given partition.
        num_rects: usize,
        /// Number of subschemes given.
        num_subschemes: usize,
    },

    /// Returned when a subscheme is not valid on the size of the rectangle that it would be used
    /// on.
    #[error("InvalidSubschemeSize: puzzle size {w}x{h} is not valid for subscheme at index {subscheme_idx}",
        w = rect_size.0,
        h = rect_size.1
    )]
    InvalidSubschemeSize {
        /// The index of the subscheme.
        subscheme_idx: usize,
        /// The size of the rectangle that the subscheme would be used on.
        rect_size: (u32, u32),
    },
}

/// A recursive color scheme consisting of a main [`Scheme`], a [`RectPartition`], and a list of
/// recursive schemes to be used as subschemes in the rectangles making up the partition.
///
/// This struct is only used to store the data making up a recursive scheme, so the implementation
/// of [`ColorScheme`] for this struct only uses the main scheme, with the subschemes not being
/// accessible. Use [`IndexedRecursiveScheme`] instead.
pub struct RecursiveScheme<'a, S: ColorScheme + ?Sized> {
    scheme: &'a S,
    partition: Option<RectPartition>,
    subschemes: Vec<Self>,
}

impl<'a, S: ColorScheme + ?Sized> RecursiveScheme<'a, S> {
    /// Create a new recursive scheme from a main [`Scheme`], a [`RectPartition`], and a list of
    /// subschemes.
    pub fn new(
        scheme: &'a S,
        partition: RectPartition,
        subschemes: Vec<Self>,
    ) -> Result<Self, RecursiveSchemeError> {
        if partition.num_rects() != subschemes.len() {
            Err(RecursiveSchemeError::IncompatiblePartitionAndSubschemes {
                num_rects: partition.num_rects(),
                num_subschemes: subschemes.len(),
            })
        } else if let Some(idx) = subschemes
            .iter()
            .zip(partition.rects.iter())
            .position(|(s, r)| !s.is_valid_size(r.width() as usize, r.height() as usize))
        {
            Err(RecursiveSchemeError::InvalidSubschemeSize {
                subscheme_idx: idx,
                rect_size: partition.rects[idx].size(),
            })
        } else {
            Ok(Self {
                scheme,
                partition: Some(partition),
                subschemes,
            })
        }
    }

    /// Create a new [`RecursiveScheme`] with no subschemes (a leaf node in the scheme tree).
    #[must_use]
    pub fn new_leaf(scheme: &'a S) -> Self {
        Self {
            scheme,
            partition: None,
            subschemes: Vec::new(),
        }
    }

    /// Returns the number of the layers in the scheme tree. The height is equal to 1 plus the
    /// maximum of the heights of the subschemes, or just 1 if there are no subschemes.
    #[must_use]
    pub fn height(&self) -> u32 {
        1 + self
            .subschemes
            .iter()
            .map(|s| s.height())
            .max()
            .unwrap_or_default()
    }

    /// Returns the color of a position `(x, y)` in a `width x height` puzzle, using layer `layer`
    /// of the color scheme tree. Layer 0 is the main scheme, layer 1 is the first subscheme, etc.
    #[must_use]
    pub fn color_at_layer(
        &self,
        layer: u32,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> Option<Rgba> {
        if layer == 0 {
            Some(self.scheme.color(width, height, x, y))
        } else if let Some(partition) = &self.partition {
            let (pos, rect) = partition
                .rects
                .iter()
                .enumerate()
                .find(|(_, rect)| rect.contains(x as u32, y as u32))
                .unwrap();
            let subscheme = &self.subschemes[pos];

            // Map the coordinates to the subscheme (so top left of the rect goes to (0, 0), etc.)
            let (width, height) = (rect.width() as usize, rect.height() as usize);
            let (left, top) = rect.top_left();
            let (x, y) = (x - left as usize, y - top as usize);

            subscheme.color_at_layer(layer - 1, width, height, x, y)
        } else {
            None
        }
    }
}

impl<'a, S: ColorScheme + ?Sized> ColorScheme for RecursiveScheme<'a, S> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        self.scheme.is_valid_size(width, height)
    }

    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        self.scheme.color(width, height, x, y)
    }
}

/// A [`RecursiveScheme`] together with an index, representing which layer of the color scheme tree
/// is currently active.
pub struct IndexedRecursiveScheme<'a, S: ColorScheme + ?Sized> {
    scheme: &'a RecursiveScheme<'a, S>,
    index: u32,
}

impl<'a, S: ColorScheme + ?Sized> IndexedRecursiveScheme<'a, S> {
    /// Create a new [`IndexedRecursiveScheme`]. The default index is 0.
    #[must_use]
    pub fn new(scheme: &'a RecursiveScheme<'a, S>) -> Self {
        Self { scheme, index: 0 }
    }

    /// Go up one layer in the scheme tree (subtract 1 from the index), unless already at the top.
    pub fn ascend(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    /// Go down one layer in the scheme tree (add 1 to the index), unless already at the bottom.
    pub fn descend(&mut self) {
        if self.index + 1 < self.scheme.height() {
            self.index += 1;
        }
    }

    /// Returns the color of the position `(x, y)` on a `width x height` puzzle (one layer below
    /// the active scheme).
    #[must_use]
    pub fn subscheme_color(&self, width: usize, height: usize, x: usize, y: usize) -> Option<Rgba> {
        self.scheme
            .color_at_layer(self.index + 1, width, height, x, y)
    }
}

impl<'a, S: ColorScheme + ?Sized> ColorScheme for IndexedRecursiveScheme<'a, S> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        let partition_valid = if let Some(p) = &self.scheme.partition {
            p.is_valid_size(width, height)
        } else {
            true
        };
        let scheme_valid = self.scheme.scheme.is_valid_size(width, height);

        partition_valid && scheme_valid
    }

    fn color(&self, width: usize, height: usize, x: usize, y: usize) -> Rgba {
        self.scheme
            .color_at_layer(self.index, width, height, x, y)
            .unwrap()
    }
}

impl<'a> From<&'a Scheme> for RecursiveScheme<'a, Scheme> {
    fn from(scheme: &'a Scheme) -> Self {
        Self::new_leaf(scheme)
    }
}

impl<'a> From<&'a RecursiveScheme<'a, Scheme>> for IndexedRecursiveScheme<'a, Scheme> {
    fn from(scheme: &'a RecursiveScheme<'a, Scheme>) -> Self {
        Self::new(scheme)
    }
}
