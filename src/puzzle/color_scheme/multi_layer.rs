//! Defines the [`MultiLayerColorScheme`] trait, representing a color scheme with multiple
//! "layers" of color schemes.

use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{
    color_scheme::{ColorScheme, ColorSchemeError},
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
    fn layer(&self, size: Size, layer: u32) -> Option<Layer<Self>>
    where
        Self: Sized,
    {
        (layer < self.num_layers(size)).then_some(Layer {
            scheme: self,
            layer,
        })
    }
}

/// Represents a single layer of a [`MultiLayerColorScheme`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer<'a, S> {
    scheme: &'a S,
    layer: u32,
}

impl<S: MultiLayerColorScheme> ColorScheme for Layer<'_, S> {
    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        self.scheme.color(size, pos, self.layer)
    }
}
