//! Defines the [`MultiLayerColorScheme`] trait, representing a color scheme with multiple
//! "layers" of color schemes.

use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{
    color_scheme::{ColorScheme, ColorSchemeError, FixedSizeColorScheme},
    size::Size,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Error type for [`MultiLayerColorScheme`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MultiLayerColorSchemeError {
    /// Errors from [`ColorScheme`].
    #[error("ColorSchemeError: {0}")]
    ColorSchemeError(#[from] ColorSchemeError),

    /// Returned when the layer index is out of bounds.
    #[error("LayerOutOfBounds: layer {layer} is out of bounds")]
    LayerOutOfBounds {
        /// The provided layer index.
        layer: u32,
    },
}

/// Similar to [`ColorScheme`], but with multiple "layers" of color schemes.
pub trait MultiLayerColorScheme {
    /// Returns the number of layers in the color scheme.
    #[must_use]
    fn num_layers(&self, size: Size) -> u32;

    /// See [`MultiLayerColorScheme::try_color`].
    ///
    /// This function may not check whether `pos` is within the bounds of the puzzle, or whether
    /// `layer` is within bounds. If these conditions are not satisfied, the function may panic or
    /// return any other color.
    #[must_use]
    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba;

    /// Returns the color of the piece in position `pos` on a solved puzzle of the given size.
    fn try_color(
        &self,
        size: Size,
        pos: (u64, u64),
        layer: u32,
    ) -> Result<Rgba, MultiLayerColorSchemeError> {
        if !size.is_within_bounds(pos) {
            Err(ColorSchemeError::PositionOutOfBounds { size, pos })?
        } else if layer >= self.num_layers(size) {
            Err(MultiLayerColorSchemeError::LayerOutOfBounds { layer })
        } else {
            Ok(self.color(size, pos, layer))
        }
    }

    /// Returns the given [`Layer`] of the scheme, if it exists.
    fn layer(&self, size: Size, layer: u32) -> Option<Layer<&Self>>
    where
        Self: Sized,
    {
        (layer < self.num_layers(size)).then_some(Layer {
            scheme: self,
            layer,
        })
    }

    /// Restricts the [`MultiLayerColorScheme`] to a single size.
    #[must_use]
    fn fixed_size(self, size: Size) -> FixedSize<Self>
    where
        Self: Sized,
    {
        FixedSize { scheme: self, size }
    }

    /// Restricts the [`MultiLayerColorScheme`] to a single size, holding a reference to the inner
    /// scheme rather than taking ownership.
    #[must_use]
    fn fixed_size_ref(&self, size: Size) -> FixedSize<&Self>
    where
        Self: Sized,
    {
        FixedSize { scheme: self, size }
    }
}

impl<S: MultiLayerColorScheme> MultiLayerColorScheme for &S {
    fn num_layers(&self, size: Size) -> u32 {
        (**self).num_layers(size)
    }

    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba {
        (**self).color(size, pos, layer)
    }
}

impl<S: MultiLayerColorScheme> MultiLayerColorScheme for &mut S {
    fn num_layers(&self, size: Size) -> u32 {
        (**self).num_layers(size)
    }

    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba {
        (**self).color(size, pos, layer)
    }
}

impl<S: MultiLayerColorScheme + ?Sized> MultiLayerColorScheme for Box<S> {
    fn num_layers(&self, size: Size) -> u32 {
        (**self).num_layers(size)
    }

    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba {
        (**self).color(size, pos, layer)
    }
}

/// A [`MultiLayerColorScheme`] that is defined for a puzzle of a single size.
pub trait FixedSizeMultiLayerColorScheme {
    /// Returns the [`Size`] of the puzzle that this label is defined for.
    #[must_use]
    fn size(&self) -> Size;

    /// See [`MultiLayerColorScheme::num_layers`].
    #[must_use]
    fn num_layers(&self) -> u32;

    /// See [`MultiLayerColorScheme::color`].
    #[must_use]
    fn color(&self, pos: (u64, u64), layer: u32) -> Rgba;

    /// See [`MultiLayerColorScheme::try_color`].
    fn try_color(&self, pos: (u64, u64), layer: u32) -> Result<Rgba, MultiLayerColorSchemeError> {
        let size = self.size();

        if !size.is_within_bounds(pos) {
            Err(ColorSchemeError::PositionOutOfBounds { size, pos })?
        } else if layer >= self.num_layers() {
            Err(MultiLayerColorSchemeError::LayerOutOfBounds { layer })
        } else {
            Ok(self.color(pos, layer))
        }
    }

    /// See [`MultiLayerColorScheme::layer`].
    fn layer(&self, layer: u32) -> Option<Layer<&Self>>
    where
        Self: Sized,
    {
        (layer < self.num_layers()).then_some(Layer {
            scheme: self,
            layer,
        })
    }
}

impl<S: FixedSizeMultiLayerColorScheme> FixedSizeMultiLayerColorScheme for &S {
    fn size(&self) -> Size {
        (**self).size()
    }

    fn num_layers(&self) -> u32 {
        (**self).num_layers()
    }

    fn color(&self, pos: (u64, u64), layer: u32) -> Rgba {
        (**self).color(pos, layer)
    }
}

impl<S: FixedSizeMultiLayerColorScheme> FixedSizeMultiLayerColorScheme for &mut S {
    fn size(&self) -> Size {
        (**self).size()
    }

    fn num_layers(&self) -> u32 {
        (**self).num_layers()
    }

    fn color(&self, pos: (u64, u64), layer: u32) -> Rgba {
        (**self).color(pos, layer)
    }
}

impl<S: FixedSizeMultiLayerColorScheme + ?Sized> FixedSizeMultiLayerColorScheme for Box<S> {
    fn size(&self) -> Size {
        (**self).size()
    }

    fn num_layers(&self) -> u32 {
        (**self).num_layers()
    }

    fn color(&self, pos: (u64, u64), layer: u32) -> Rgba {
        (**self).color(pos, layer)
    }
}

/// A [`MultiLayerColorScheme`] restricted to a single size.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FixedSize<S: MultiLayerColorScheme> {
    scheme: S,
    size: Size,
}

impl<S: MultiLayerColorScheme> FixedSize<S> {
    /// Returns a reference to the inner [`MultiLayerColorScheme`].
    pub fn inner(&self) -> &S {
        &self.scheme
    }

    /// Extracts the inner [`MultiLayerColorScheme`], consuming `self`.
    pub fn into_inner(self) -> S {
        self.scheme
    }
}

impl<S: MultiLayerColorScheme> FixedSizeMultiLayerColorScheme for FixedSize<S> {
    fn size(&self) -> Size {
        self.size
    }

    fn num_layers(&self) -> u32 {
        self.scheme.num_layers(self.size)
    }

    fn color(&self, pos: (u64, u64), layer: u32) -> Rgba {
        self.scheme.color(self.size, pos, layer)
    }
}

/// Represents a single layer of a [`MultiLayerColorScheme`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer<S> {
    scheme: S,
    layer: u32,
}

impl<S: MultiLayerColorScheme> Layer<S> {
    /// Returns a reference to the inner [`MultiLayerColorScheme`].
    pub fn scheme(&self) -> &S {
        &self.scheme
    }

    /// Returns the index of the layer that this [`Layer`] represents.
    pub fn layer(&self) -> u32 {
        self.layer
    }
}

impl<S: MultiLayerColorScheme> ColorScheme for Layer<S> {
    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        self.scheme.color(size, pos, self.layer)
    }
}

impl<S: FixedSizeMultiLayerColorScheme> FixedSizeColorScheme for Layer<S> {
    fn size(&self) -> Size {
        self.scheme.size()
    }

    fn color(&self, pos: (u64, u64)) -> Rgba {
        self.scheme.color(pos, self.layer)
    }
}
