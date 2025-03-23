use blanket::blanket;
use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{
    color_scheme::{ColorScheme, ColorSchemeError},
    size::Size,
};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Error type for [`MultiLayerColorScheme`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[blanket(derive(Ref, Rc, Arc, Mut))]
pub trait MultiLayerColorScheme {
    #[must_use]
    fn is_valid_size(&self, size: Size) -> bool;

    #[must_use]
    fn num_layers(&self, size: Size) -> u32;

    #[must_use]
    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba;

    fn try_color(
        &self,
        size: Size,
        pos: (u64, u64),
        layer: u32,
    ) -> Result<Rgba, MultiLayerColorSchemeError> {
        if !self.is_valid_size(size) {
            Err(ColorSchemeError::InvalidSize(size))?
        } else if !size.is_within_bounds(pos) {
            Err(ColorSchemeError::PositionOutOfBounds { size, pos })?
        } else if layer >= self.num_layers(size) {
            Err(MultiLayerColorSchemeError::LayerOutOfBounds { layer })
        } else {
            Ok(self.color(size, pos, layer))
        }
    }
}

pub struct Layer<S> {
    pub(super) scheme: S,
    pub(super) layer: u32,
}

impl<S: MultiLayerColorScheme> ColorScheme for Layer<S> {
    fn is_valid_size(&self, size: Size) -> bool {
        self.scheme.is_valid_size(size)
    }

    fn color(&self, size: Size, pos: (u64, u64)) -> Rgba {
        self.scheme.color(size, pos, self.layer)
    }
}
