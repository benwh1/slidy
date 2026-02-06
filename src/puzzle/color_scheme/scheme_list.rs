//! Defines the [`SchemeList`] struct, which is a [`MultiLayerColorScheme`] that consists of a list
//! of [`ColorScheme`]s.

use std::marker::PhantomData;

use palette::rgb::Rgba;
use thiserror::Error;

use crate::puzzle::{
    color_scheme::{multi_layer::MultiLayerColorScheme, ColorScheme},
    size::Size,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A list of [`ColorScheme`]s, which can be used as a [`MultiLayerColorScheme`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(try_from = "SchemeListUnvalidated<S, List>")
)]
pub struct SchemeList<S: ColorScheme, List: AsRef<[S]>> {
    schemes: List,
    phantom_s: PhantomData<S>,
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
struct SchemeListUnvalidated<S: ColorScheme, List: AsRef<[S]>> {
    schemes: List,
    phantom_s: PhantomData<S>,
}

impl<S: ColorScheme, List: AsRef<[S]>> TryFrom<SchemeListUnvalidated<S, List>>
    for SchemeList<S, List>
{
    type Error = SchemeListError;

    fn try_from(value: SchemeListUnvalidated<S, List>) -> Result<Self, Self::Error> {
        let SchemeListUnvalidated { schemes, .. } = value;

        Self::new(schemes)
    }
}

/// Error type for [`SchemeList`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SchemeListError {
    /// Returned from [`SchemeList::new`] if the list of schemes is empty.
    #[error("Empty: list of schemes must be non-empty")]
    Empty,
}

impl<S: ColorScheme, List: AsRef<[S]>> SchemeList<S, List> {
    /// Create a new [`SchemeList`] containing the given list of color schemes.
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
}

impl<S: ColorScheme, List: AsRef<[S]>> MultiLayerColorScheme for SchemeList<S, List> {
    fn num_layers(&self, _size: Size) -> u32 {
        self.schemes().len() as u32
    }

    fn color(&self, size: Size, pos: (u64, u64), layer: u32) -> Rgba {
        self.schemes()[layer as usize].color(size, pos)
    }
}

#[allow(clippy::fallible_impl_from)]
impl<S: ColorScheme> From<S> for SchemeList<S, Vec<S>> {
    fn from(scheme: S) -> Self {
        Self::new(vec![scheme]).unwrap()
    }
}
