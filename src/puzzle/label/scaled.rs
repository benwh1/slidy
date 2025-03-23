//! Defines the [`Scaled`] label modifier.

use thiserror::Error;

use crate::puzzle::{label::label::Label, size::Size};

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Scales a [`Label`] up by a horizontal factor and a vertical factor.
///
/// For example, consider the [`crate::puzzle::label::label::RowGrids`] label on a 6x4 puzzle, with
/// 36 distinct labels. If we scale it up by a factor of 3 horizontally, and a factor of 2
/// vertically, the top left 3x2 block will have the label 0, the top right 3x2 block will have the
/// label 1, the left 3x2 block in the middle two rows will have the label 2, etc. for a total of 6
/// distinct labels.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(try_from = "ScaledUnvalidated<L>")
)]
pub struct Scaled<L: Label> {
    label: L,
    factor: (u64, u64),
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
struct ScaledUnvalidated<L: Label> {
    label: L,
    factor: (u64, u64),
}

impl<L: Label> TryFrom<ScaledUnvalidated<L>> for Scaled<L> {
    type Error = ScaledError;

    fn try_from(value: ScaledUnvalidated<L>) -> Result<Self, Self::Error> {
        let ScaledUnvalidated { label, factor } = value;

        Self::new(label, factor)
    }
}

/// Error type for [`Scaled`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ScaledError {
    /// Returned from [`Scaled::new`] if either of the scaling factors are zero.
    #[error("ZeroScale: horizontal and vertical scale factors must be positive")]
    ZeroScale,
}

impl<L: Label> Scaled<L> {
    /// Creates a new [`Scaled`] from a [`Label`] and scaling factors.
    pub fn new(label: L, factor: (u64, u64)) -> Result<Self, ScaledError> {
        if factor.0 == 0 || factor.1 == 0 {
            Err(ScaledError::ZeroScale)
        } else {
            Ok(Self { label, factor })
        }
    }

    /// Returns a reference to the inner [`Label`].
    pub fn inner(&self) -> &L {
        &self.label
    }

    /// Returns the horizontal and vertical scaling factors.
    pub fn scale(&self) -> (u64, u64) {
        self.factor
    }
}

impl<L: Label> Label for Scaled<L> {
    fn is_valid_size(&self, size: Size) -> bool {
        let (width, height) = size.into();
        let (sw, sh) = (
            width.div_ceil(self.factor.0),
            height.div_ceil(self.factor.1),
        );

        Size::new(sw, sh)
            .map(|size| self.label.is_valid_size(size))
            .unwrap_or_default()
    }

    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let (width, height) = size.into();
        let (sw, sh) = (
            width.div_ceil(self.factor.0),
            height.div_ceil(self.factor.1),
        );
        let (x, y) = (x / self.factor.0, y / self.factor.1);

        Size::new(sw, sh)
            .map(|size| self.label.position_label(size, (x, y)))
            .unwrap_or_default()
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();
        let (sw, sh) = (
            width.div_ceil(self.factor.0),
            height.div_ceil(self.factor.1),
        );

        Size::new(sw, sh)
            .map(|size| self.label.num_labels(size))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::label::label::RowGrids;

    use super::*;

    #[test]
    fn test_new() {
        assert!(Scaled::new(RowGrids, (0, 1)).is_err());
        assert!(Scaled::new(RowGrids, (1, 0)).is_err());
        assert!(Scaled::new(RowGrids, (1, 1)).is_ok());
        assert!(Scaled::new(RowGrids, (3, 5)).is_ok());
    }

    #[test]
    fn test_scale() {
        let label = Scaled::new(RowGrids, (3, 5)).unwrap();
        assert_eq!(label.scale(), (3, 5));
    }

    #[test]
    fn test_scaled_row_grids() {
        let size = Size::new(8, 5).unwrap();
        let label = Scaled::new(&RowGrids, (3, 2)).unwrap();

        let labels = (0..40)
            .map(|i| label.position_label(size, (i % 8, i / 8)))
            .collect::<Vec<_>>();
        let num_labels = label.num_labels(size);

        #[rustfmt::skip]
        assert_eq!(labels, vec![
            0, 0, 0, 1, 1, 1, 2, 2,
            0, 0, 0, 1, 1, 1, 2, 2,
            3, 3, 3, 4, 4, 4, 5, 5,
            3, 3, 3, 4, 4, 4, 5, 5,
            6, 6, 6, 7, 7, 7, 8, 8,
        ]);
        assert_eq!(num_labels, 9);
    }

    #[test]
    fn test_scaled_row_grids_2() {
        let size = Size::new(10, 10).unwrap();
        let label = Scaled::new(&RowGrids, (10, 5)).unwrap();

        let labels = (0..100)
            .map(|i| label.position_label(size, (i % 10, i / 10)))
            .collect::<Vec<_>>();
        let num_labels = label.num_labels(size);

        #[rustfmt::skip]
        assert_eq!(labels, vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ]);
        assert_eq!(num_labels, 2);
    }
}
