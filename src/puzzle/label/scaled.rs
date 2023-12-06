//! Defines the [`Scaled`] label modifier.

use thiserror::Error;

use crate::puzzle::size::Size;

use super::label::Label;

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Scales a [`Label`] up by a horizontal factor and a vertical factor. For example, consider the
/// [`crate::puzzle::label::label::RowGrids`] label on a 6x4 puzzle, with 36 distinct labels. If we
/// scale it up by a factor of 3 horizontally, and a factor of 2 vertically, the top left 3x2 block
/// will have the label 0, the top right 3x2 block will have the label 1, the left 3x2 block in the
/// middle two rows will have the label 2, etc. for a total of 6 distinct labels.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Scaled<L: Label> {
    label: L,
    factor: (u32, u32),
}

/// Error type for [`Scaled`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ScaledError {
    /// Returned from [`Scaled::new`] if either of the scaling factors are zero.
    #[error("ZeroScale: horizontal and vertical scale factors must be positive")]
    ZeroScale,
}

impl<L: Label> Scaled<L> {
    /// Creates a new [`Scaled`] from a [`Label`] and scaling factors.
    pub fn new(label: L, factor: (u32, u32)) -> Result<Self, ScaledError> {
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
    pub fn scale(&self) -> (u32, u32) {
        self.factor
    }
}

impl<L: Label> Label for Scaled<L> {
    fn is_valid_size(&self, size: Size) -> bool {
        let (width, height) = size.into();
        let (sw, sh) = (
            width.div_ceil(self.factor.0 as usize),
            height.div_ceil(self.factor.1 as usize),
        );

        Size::new(sw, sh)
            .map(|size| self.label.is_valid_size(size))
            .unwrap_or_default()
    }

    fn position_label(&self, size: Size, (x, y): (usize, usize)) -> usize {
        let (width, height) = size.into();
        let (sw, sh) = (
            width.div_ceil(self.factor.0 as usize),
            height.div_ceil(self.factor.1 as usize),
        );
        let (x, y) = (
            x.div_floor(self.factor.0 as usize),
            y.div_floor(self.factor.1 as usize),
        );

        Size::new(sw, sh)
            .map(|size| self.label.position_label(size, (x, y)))
            .unwrap_or_default()
    }

    fn num_labels(&self, size: Size) -> usize {
        let (width, height) = size.into();
        let (sw, sh) = (
            width.div_ceil(self.factor.0 as usize),
            height.div_ceil(self.factor.1 as usize),
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
}
