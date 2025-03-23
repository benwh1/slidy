//! Defines the [`SchemeList`] struct, which is a [`MultiLayerColorScheme`] that consists of a list
//! of [`ColorScheme`]s.

use std::marker::PhantomData;

use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{
    color_scheme::{
        multi_layer::{Layer, MultiLayerColorScheme},
        ColorScheme,
    },
    size::Size,
};

/// A list of [`ColorScheme`]s and an index, indicating which color scheme is currently "active".
/// The implementation of [`ColorScheme`] for this type uses the active scheme.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SchemeList<S: ColorScheme, List: AsRef<[S]>> {
    schemes: List,
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
                phantom_s: PhantomData,
            })
        }
    }

    /// Returns a reference to the list of schemes.
    pub fn schemes(&self) -> &[S] {
        self.schemes.as_ref()
    }

    /// Returns the number of schemes in the list.
    #[must_use]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.schemes.as_ref().len()
    }

    /// Returns the given [`Layer`] of the scheme.
    pub fn layer(&self, layer: u32) -> Layer<&Self> {
        Layer {
            scheme: self,
            layer,
        }
    }
}

impl<S: ColorScheme, List: AsRef<[S]>> MultiLayerColorScheme for SchemeList<S, List> {
    fn is_valid_size(&self, size: Size) -> bool {
        self.schemes
            .as_ref()
            .iter()
            .all(|scheme| scheme.is_valid_size(size))
    }

    fn num_layers(&self, _size: Size) -> u32 {
        self.len() as u32
    }

    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba {
        self.layer(layer).color(size, pos)
    }
}
