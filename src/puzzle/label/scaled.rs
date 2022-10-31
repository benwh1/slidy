//! Defines the [`Scaled`] label modifier.

use thiserror::Error;

use super::label::Label;

/// Scales a [`Label`] up by a horizontal factor and a vertical factor. For example, consider the
/// [`crate::puzzle::label::label::RowGrids`] label on a 6x4 puzzle, with 36 distinct labels. If we
/// scale it up by a factor of 3 horizontally, and a factor of 2 vertically, the top left 3x2 block
/// will have the label 0, the top right 3x2 block will have the label 1, the left 3x2 block in the
/// middle two rows will have the label 2, etc. for a total of 6 distinct labels.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scaled<'a, L: Label> {
    label: &'a L,
    horizontal: u32,
    vertical: u32,
}

/// Error type for [`Scaled`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScaledError {
    /// Returned from [`Scaled::new`] if either of the scaling factors are zero.
    #[error("ZeroScale: horizontal and vertical scale factors must be positive")]
    ZeroScale,
}

impl<'a, L: Label> Scaled<'a, L> {
    /// Creates a new [`Scaled`] from a [`Label`] and scaling factors.
    pub fn new(label: &'a L, horizontal: u32, vertical: u32) -> Result<Self, ScaledError> {
        if horizontal == 0 || vertical == 0 {
            Err(ScaledError::ZeroScale)
        } else {
            Ok(Self {
                label,
                horizontal,
                vertical,
            })
        }
    }
}

impl<'a, L: Label> Label for Scaled<'a, L> {
    fn is_valid_size(&self, width: usize, height: usize) -> bool {
        let width = width.div_ceil(self.horizontal as usize);
        let height = height.div_ceil(self.vertical as usize);
        self.label.is_valid_size(width, height)
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        let width = width.div_ceil(self.horizontal as usize);
        let height = height.div_ceil(self.vertical as usize);
        let x = x.div_floor(self.horizontal as usize);
        let y = y.div_floor(self.vertical as usize);
        self.label.position_label_unchecked(width, height, x, y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        let width = width.div_ceil(self.horizontal as usize);
        let height = height.div_ceil(self.vertical as usize);
        self.label.num_labels_unchecked(width, height)
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::label::label::RowGrids;

    use super::*;

    #[test]
    fn test_scaled_row_grids() {
        let label = Scaled::new(&RowGrids, 3, 2).unwrap();

        let labels = (0..40)
            .map(|i| label.position_label_unchecked(8, 5, i % 8, i / 8))
            .collect::<Vec<_>>();
        let num_labels = label.num_labels_unchecked(8, 5);

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
