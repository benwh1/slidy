//! Defines the [`AsAlgorithmSlice`] trait.

use crate::algorithm::{algorithm::Algorithm, slice::AlgorithmSlice};

/// Conversion to [`AlgorithmSlice`].
pub trait AsAlgorithmSlice<'a> {
    /// Converts `self` to an [`AlgorithmSlice`].
    fn as_slice(&'a self) -> AlgorithmSlice<'a>;
}

impl<'a> AsAlgorithmSlice<'a> for Algorithm {
    /// Returns an [`AlgorithmSlice`] containing the entire algorithm.
    fn as_slice(&'a self) -> AlgorithmSlice<'a> {
        AlgorithmSlice {
            first: None,
            middle: &self.moves,
            last: None,
        }
    }
}

impl<'a> AsAlgorithmSlice<'a> for AlgorithmSlice<'a> {
    fn as_slice(&'a self) -> AlgorithmSlice<'a> {
        *self
    }
}
