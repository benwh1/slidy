//! Defines the [`AsAlgorithmSlice`] trait.

use crate::algorithm::{algorithm::Algorithm, slice::AlgorithmSlice};

/// Conversion to [`AlgorithmSlice`].
pub trait AsAlgorithmSlice<'a> {
    /// Converts `self` to an [`AlgorithmSlice`].
    fn as_slice(&'a self) -> AlgorithmSlice<'a>;
}

impl<'a> AsAlgorithmSlice<'a> for Algorithm {
    fn as_slice(&'a self) -> AlgorithmSlice<'a> {
        self.as_slice()
    }
}

impl<'a> AsAlgorithmSlice<'a> for AlgorithmSlice<'a> {
    fn as_slice(&'a self) -> AlgorithmSlice<'a> {
        let Self {
            first,
            middle,
            last,
        } = self;
        Self {
            first: *first,
            middle,
            last: *last,
        }
    }
}
