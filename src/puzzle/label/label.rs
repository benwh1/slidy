//! Defines the [`Label`] trait and several implementations.

use std::cmp::Ordering;

use thiserror::Error;

use crate::puzzle::size::Size;

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

/// Error type for [`Label`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LabelError {
    /// Returned when the `(x, y)` position is outside the bounds of the puzzle.
    #[error("PositionOutOfBounds: position {pos:?} is out of bounds on a {size} puzzle")]
    PositionOutOfBounds {
        /// Size of the puzzle.
        size: Size,
        /// Piece position.
        pos: (u64, u64),
    },
}

/// Provides a function mapping an `(x, y)` coordinate on a puzzle to a number which we call the
/// label of `(x, y)`.
pub trait Label {
    /// Returns the label of `(x, y)` on a puzzle of the given size.
    ///
    /// The label must be an integer from 0 to `self.num_labels(size) - 1`.
    ///
    /// This function may not check whether `(x, y)` is within the bounds of the puzzle. If this
    /// condition is not satisfied, the function may panic or return an invalid label, e.g. an
    /// integer greater than or equal to `self.num_labels(size)`.
    #[must_use]
    fn position_label(&self, size: Size, pos: (u64, u64)) -> u64;

    /// See [`Self::position_label`].
    fn try_position_label(&self, size: Size, pos: (u64, u64)) -> Result<u64, LabelError> {
        if size.is_within_bounds(pos) {
            Ok(self.position_label(size, pos))
        } else {
            Err(LabelError::PositionOutOfBounds { size, pos })
        }
    }

    /// Returns the total number of distinct labels across all `(x, y)` positions in the puzzle.
    #[must_use]
    fn num_labels(&self, size: Size) -> u64;

    /// Restricts the [`Label`] to a single size.
    #[must_use]
    fn fixed_size(self, size: Size) -> FixedSize<Self>
    where
        Self: Sized,
    {
        FixedSize { label: self, size }
    }

    /// Restricts the [`Label`] to a single size, holding a reference to the inner label rather
    /// than taking ownership.
    #[must_use]
    fn fixed_size_ref(&self, size: Size) -> FixedSize<&Self>
    where
        Self: Sized,
    {
        FixedSize { label: self, size }
    }
}

impl<'a, L: Label> Label for &'a L {
    fn position_label(&self, size: Size, pos: (u64, u64)) -> u64 {
        (**self).position_label(size, pos)
    }

    fn num_labels(&self, size: Size) -> u64 {
        (**self).num_labels(size)
    }
}

impl<'a, L: Label> Label for &'a mut L {
    fn position_label(&self, size: Size, pos: (u64, u64)) -> u64 {
        (**self).position_label(size, pos)
    }

    fn num_labels(&self, size: Size) -> u64 {
        (**self).num_labels(size)
    }
}

impl<L: Label + ?Sized> Label for Box<L> {
    fn position_label(&self, size: Size, pos: (u64, u64)) -> u64 {
        (**self).position_label(size, pos)
    }

    fn num_labels(&self, size: Size) -> u64 {
        (**self).num_labels(size)
    }
}

/// A [`Label`] that is defined for a puzzle of a single size.
pub trait FixedSizeLabel {
    /// Returns the [`Size`] of the puzzle that this label is defined for.
    #[must_use]
    fn size(&self) -> Size;

    /// See [`Label::position_label`].
    #[must_use]
    fn position_label(&self, pos: (u64, u64)) -> u64;

    /// See [`Label::try_position_label`].
    fn try_position_label(&self, pos: (u64, u64)) -> Result<u64, LabelError> {
        let size = self.size();

        if size.is_within_bounds(pos) {
            Ok(self.position_label(pos))
        } else {
            Err(LabelError::PositionOutOfBounds { size, pos })
        }
    }

    /// See [`Label::num_labels`].
    #[must_use]
    fn num_labels(&self) -> u64;
}

impl<'a, L: FixedSizeLabel> FixedSizeLabel for &'a L {
    fn size(&self) -> Size {
        (**self).size()
    }

    fn position_label(&self, pos: (u64, u64)) -> u64 {
        (**self).position_label(pos)
    }

    fn num_labels(&self) -> u64 {
        (**self).num_labels()
    }
}

impl<'a, L: FixedSizeLabel> FixedSizeLabel for &'a mut L {
    fn size(&self) -> Size {
        (**self).size()
    }

    fn position_label(&self, pos: (u64, u64)) -> u64 {
        (**self).position_label(pos)
    }

    fn num_labels(&self) -> u64 {
        (**self).num_labels()
    }
}

impl<L: FixedSizeLabel + ?Sized> FixedSizeLabel for Box<L> {
    fn size(&self) -> Size {
        (**self).size()
    }

    fn position_label(&self, pos: (u64, u64)) -> u64 {
        (**self).position_label(pos)
    }

    fn num_labels(&self) -> u64 {
        (**self).num_labels()
    }
}

/// A [`Label`] restricted to a single size.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FixedSize<L: Label> {
    label: L,
    size: Size,
}

impl<L: Label> FixedSizeLabel for FixedSize<L> {
    fn size(&self) -> Size {
        self.size
    }

    fn position_label(&self, pos: (u64, u64)) -> u64 {
        self.label.position_label(self.size, pos)
    }

    fn num_labels(&self) -> u64 {
        self.label.num_labels(self.size)
    }
}

/// Marker trait for [`Label`]s that assign distinct labels to every position.
/// These will always have `num_labels(width, height) == width * height`.
pub trait BijectiveLabel: Label {}

macro_rules! define_label {
    ($($(#[$annot:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$annot])*
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct $name;
        )*
    };
}

define_label!(
    /// Assigns 0 to every position.
    Trivial,
    /// Assigns distinct labels in left-to-right reading order: left to right along the top row,
    /// then left to right along the second row, etc.
    RowGrids,
    /// Assigns a label to each row.
    Rows,
    /// Assigns a label to each fringe (combined row and column, starting from the top left).
    Fringe,
    /// Assigns distinct labels in fringe order: left to right along the top row, then top to
    /// bottom down the first column, then left to right along the second row, etc.
    FringeGrids,
    /// Assigns labels to each row or column, until the remaining unlabelled part of the puzzle is
    /// a square, and then labels the rest with [`Fringe`].
    SquareFringe,
    /// Same as [`Fringe`], but the row and column parts of the fringe are given different labels.
    SplitFringe,
    /// Same as [`SquareFringe`], but uses [`SplitFringe`] for the square part.
    SplitSquareFringe,
    /// Assigns labels to each bottom-left to top-right diagonal, starting from the top left.
    Diagonals,
    /// Assigns labels to each of the first `height - 2` rows, then assigns labels to the last two
    /// rows in columns.
    LastTwoRows,
    /// Same as [`LastTwoRows`], but the labels of the last two rows restart from 0 (so for
    /// example, the top row and the bottom left piece are given the same label).
    SplitLastTwoRows,
    /// Assigns a label to each concentric rectangle around the puzzle.
    ConcentricRectangles,
    /// Assigns labels in a spiral pattern.
    ///
    /// The top row (minus the top right piece) gets label 0, then the right column (minus the
    /// bottom right piece) gets label 1, then the bottom row (minus the bottom left piece) gets
    /// label 2, etc.
    Spiral,
    /// Same as [`Spiral`] but every piece gets a distinct label.
    SpiralGrids,
    /// Assigns 0 to the top left, then 1 to the pieces adjacent to the top left, then 0 to the
    /// pieces adjacent to those, etc.
    Checkerboard,
);

impl BijectiveLabel for RowGrids {}
impl BijectiveLabel for FringeGrids {}
impl BijectiveLabel for SpiralGrids {}

impl Label for Trivial {
    fn position_label(&self, _size: Size, _pos: (u64, u64)) -> u64 {
        0
    }

    fn num_labels(&self, _size: Size) -> u64 {
        1
    }
}

impl Label for RowGrids {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        x + size.width() * y
    }

    fn num_labels(&self, size: Size) -> u64 {
        size.area()
    }
}

impl Label for Rows {
    fn position_label(&self, _size: Size, (_, y): (u64, u64)) -> u64 {
        y
    }

    fn num_labels(&self, size: Size) -> u64 {
        size.height()
    }
}

impl Label for Fringe {
    fn position_label(&self, _size: Size, (x, y): (u64, u64)) -> u64 {
        x.min(y)
    }

    fn num_labels(&self, size: Size) -> u64 {
        size.width().min(size.height())
    }
}

impl Label for FringeGrids {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        // Which (non-split) fringe is (x, y) in?
        let fringe = x.min(y);

        // Is it in the row part or the horizontal part?
        let vertical_part = x < y;

        // Sum w+h-1-2k, k=0..f-1 = f(w+h-f) = number of tiles in previous fringes
        let (width, height) = size.into();
        let previous_fringes = fringe * (width + height - fringe);

        // How many pieces before this one in the current fringe?
        let current_fringe = if vertical_part {
            width - fringe + y - x - 1
        } else {
            x - y
        };

        previous_fringes + current_fringe
    }

    fn num_labels(&self, size: Size) -> u64 {
        size.area()
    }
}

impl Label for SquareFringe {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let (width, height) = size.into();
        match width.cmp(&height) {
            // Puzzle is taller than it is wide
            Ordering::Less => {
                let diff = height - width;
                // Top part of the puzzle, above square part
                if y < diff {
                    y
                }
                // Square part of the puzzle
                else {
                    diff + Fringe.position_label(size, (x, y - diff))
                }
            }
            Ordering::Equal => Fringe.position_label(size, (x, y)),
            Ordering::Greater => self.position_label(size.transpose(), (y, x)),
        }
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();
        Fringe.num_labels(size) + width.abs_diff(height)
    }
}

impl Label for SplitFringe {
    fn position_label(&self, _size: Size, (x, y): (u64, u64)) -> u64 {
        // Which (non-split) fringe is (x, y) in?
        let fringe = x.min(y);

        // Is it in the row part or the horizontal part?
        let vertical_part = x < y;

        2 * fringe + u64::from(vertical_part)
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();
        2 * width.min(height) - u64::from(height <= width)
    }
}

impl Label for SplitSquareFringe {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let (width, height) = size.into();
        let d = width.abs_diff(height);

        match width.cmp(&height) {
            Ordering::Less => {
                if y < d {
                    y
                } else {
                    d + SplitFringe.position_label(size, (x, y - d))
                }
            }
            Ordering::Equal => SplitFringe.position_label(size, (x, y)),
            Ordering::Greater => {
                if x < d {
                    x
                } else {
                    d + SplitFringe.position_label(size, (x - d, y))
                }
            }
        }
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();
        let diff = width.abs_diff(height);

        diff + SplitFringe.num_labels(size.shrink_to_square())
    }
}

impl Label for Diagonals {
    fn position_label(&self, _size: Size, (x, y): (u64, u64)) -> u64 {
        x + y
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();
        width + height - 1
    }
}

impl Label for LastTwoRows {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let h = size.height().saturating_sub(2);
        if y < h {
            y
        } else {
            h + x
        }
    }

    fn num_labels(&self, size: Size) -> u64 {
        size.width() + size.height().saturating_sub(2)
    }
}

impl Label for SplitLastTwoRows {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        if y < size.height().saturating_sub(2) {
            y
        } else {
            x
        }
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();
        width.max(height.saturating_sub(2))
    }
}

impl Label for ConcentricRectangles {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let (width, height) = size.into();
        x.min(y).min(width - 1 - x).min(height - 1 - y)
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();
        width.min(height).div_ceil(2)
    }
}

impl Label for Spiral {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let rect_label = ConcentricRectangles.position_label(size, (x, y));

        // Calculate the values (x, y, width, height) if we were to strip off any outer rectangles
        // from the puzzle.
        // e.g. the piece in position (1, 1) on a 4x5 puzzle becomes the piece in position (0, 0)
        // on a 2x3 puzzle when we remove the outer rectangle, so we would have
        // (rx, ry, rw, rh) = (0, 0, 2, 3).
        let (width, height) = size.into();
        let (rx, ry, rw, rh) = (
            x - rect_label,
            y - rect_label,
            width - 2 * rect_label,
            height - 2 * rect_label,
        );

        // Which side of the rectangle is the piece on?
        // If the rectangle has a side of length 1, just give the whole rectangle the same label
        // (instead of giving all pieces the same label except one, and the last a different label)
        let rect_side = if rw.min(rh) == 1 || (ry == 0 && rx < rw - 1) {
            // Top row
            0
        } else if rx == rw - 1 && ry < rh - 1 {
            // Right column
            1
        } else if ry == rh - 1 && rx > 0 {
            // Bottom row
            2
        } else {
            // Left column
            3
        };

        4 * rect_label + rect_side
    }

    fn num_labels(&self, size: Size) -> u64 {
        let (width, height) = size.into();

        // 4 * number of rectangles of width and height > 1, plus 1 if the innermost rectangle has
        // width or height 1.
        4 * (width.min(height) / 2) + width.min(height) % 2
    }
}

impl Label for SpiralGrids {
    fn position_label(&self, size: Size, (x, y): (u64, u64)) -> u64 {
        let rect_label = ConcentricRectangles.position_label(size, (x, y));

        // See `Spiral::position_label`
        let (width, height) = size.into();
        let (rx, ry, rw, rh) = (
            x - rect_label,
            y - rect_label,
            width - 2 * rect_label,
            height - 2 * rect_label,
        );

        // Number of pieces in the outer rectangles that we removed.
        // Number of pieces in rect k = 2(w+h-2) - 8k, so sum this from k = 0..rect_label-1
        let num_outer_pieces = 2 * rect_label * (width + height - 2 * rect_label);

        // Find which side of the rectangle the piece is on, and count how many pieces came before
        let rect_pieces = if ry == 0 && rx < rw - 1 {
            // Top row
            rx
        } else if rx == rw - 1 && ry < rh - 1 {
            // Right column
            rw - 1 + ry
        } else if ry == rh - 1 && rx > 0 {
            // Bottom row
            2 * (rw + rh - 2) - (rh - 1 + rx)
        } else {
            // Left column
            2 * (rw + rh - 2) - ry
        };

        num_outer_pieces + rect_pieces
    }

    fn num_labels(&self, size: Size) -> u64 {
        size.area()
    }
}

impl Label for Checkerboard {
    fn position_label(&self, _size: Size, (x, y): (u64, u64)) -> u64 {
        (x + y) % 2
    }

    fn num_labels(&self, _size: Size) -> u64 {
        2
    }
}

#[cfg(test)]
mod tests {
    macro_rules! test_label {
        ($label:ty, $($w:literal x $h:literal : $labels:expr),+ $(,)?) => {
            paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::{label::label::{Label as _, $label}, size::Size};

                    $(#[test]
                    fn [< test_ $label:snake _ $w x $h >] () {
                        let size = Size::new($w, $h).unwrap();
                        let labels = (0..size.area())
                            .map(|i| {
                                #[allow(clippy::modulo_one)]
                                let position = (i % $w, i / $w);

                                $label.position_label(size, position)
                            })
                            .collect::<Vec<_>>();
                        let num_labels = $label.num_labels(size);
                        let expected_num_labels = $labels.iter().max().unwrap() + 1;
                        assert_eq!(labels, $labels);
                        assert_eq!(num_labels, expected_num_labels);
                    })*
                }
            }
        };
    }

    test_label!(
        Trivial,
        4 x 4: [
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
        ],
    );

    test_label!(
        RowGrids,
        1 x 4: [
            0,
            1,
            2,
            3,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
             0,  1,  2,  3,
             4,  5,  6,  7,
             8,  9, 10, 11,
            12, 13, 14, 15,
        ],
        4 x 6: [
             0,  1,  2,  3,
             4,  5,  6,  7,
             8,  9, 10, 11,
            12, 13, 14, 15,
            16, 17, 18, 19,
            20, 21, 22, 23,
        ],
        6 x 4: [
             0,  1,  2,  3,  4,  5,
             6,  7,  8,  9, 10, 11,
            12, 13, 14, 15, 16, 17,
            18, 19, 20, 21, 22, 23,
        ],
    );

    test_label!(
        Rows,
        1 x 4: [
            0,
            1,
            2,
            3,
        ],
        4 x 1: [
            0, 0, 0, 0,
        ],
        4 x 4: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
        ],
        4 x 6: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 4, 4, 4,
            5, 5, 5, 5,
        ],
        6 x 4: [
            0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1,
            2, 2, 2, 2, 2, 2,
            3, 3, 3, 3, 3, 3,
        ],
    );

    test_label!(
        Fringe,
        1 x 4: [
            0,
            0,
            0,
            0,
        ],
        4 x 1: [
            0, 0, 0, 0,
        ],
        4 x 4: [
            0, 0, 0, 0,
            0, 1, 1, 1,
            0, 1, 2, 2,
            0, 1, 2, 3,
        ],
        4 x 6: [
            0, 0, 0, 0,
            0, 1, 1, 1,
            0, 1, 2, 2,
            0, 1, 2, 3,
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        6 x 4: [
            0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1,
            0, 1, 2, 2, 2, 2,
            0, 1, 2, 3, 3, 3
        ],
    );

    test_label!(
        FringeGrids,
        1 x 4: [
            0,
            1,
            2,
            3,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
            0,  1,  2,  3,
            4,  7,  8,  9,
            5, 10, 12, 13,
            6, 11, 14, 15,
        ],
        4 x 6: [
            0,  1,  2,  3,
            4,  9, 10, 11,
            5, 12, 16, 17,
            6, 13, 18, 21,
            7, 14, 19, 22,
            8, 15, 20, 23,
        ],
        6 x 4: [
            0,  1,  2,  3,  4,  5,
            6,  9, 10, 11, 12, 13,
            7, 14, 16, 17, 18, 19,
            8, 15, 20, 21, 22, 23,
        ],
    );

    test_label!(
        SquareFringe,
        1 x 4: [
            0,
            1,
            2,
            3,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
            0, 0, 0, 0,
            0, 1, 1, 1,
            0, 1, 2, 2,
            0, 1, 2, 3,
        ],
        4 x 6: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            2, 3, 3, 3,
            2, 3, 4, 4,
            2, 3, 4, 5,
        ],
        6 x 4: [
            0, 1, 2, 2, 2, 2,
            0, 1, 2, 3, 3, 3,
            0, 1, 2, 3, 4, 4,
            0, 1, 2, 3, 4, 5,
        ],
    );

    test_label!(
        SplitFringe,
        1 x 4: [
            0,
            1,
            1,
            1,
        ],
        4 x 1: [
            0, 0, 0, 0,
        ],
        4 x 4: [
            0, 0, 0, 0,
            1, 2, 2, 2,
            1, 3, 4, 4,
            1, 3, 5, 6,
        ],
        4 x 6: [
            0, 0, 0, 0,
            1, 2, 2, 2,
            1, 3, 4, 4,
            1, 3, 5, 6,
            1, 3, 5, 7,
            1, 3, 5, 7,
        ],
        6 x 4: [
            0, 0, 0, 0, 0, 0,
            1, 2, 2, 2, 2, 2,
            1, 3, 4, 4, 4, 4,
            1, 3, 5, 6, 6, 6,
        ],
    );

    test_label!(
        SplitSquareFringe,
        1 x 4: [
            0,
            1,
            2,
            3,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
            0, 0, 0, 0,
            1, 2, 2, 2,
            1, 3, 4, 4,
            1, 3, 5, 6,
        ],
        4 x 6: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 4, 4, 4,
            3, 5, 6, 6,
            3, 5, 7, 8,
        ],
        6 x 4: [
            0, 1, 2, 2, 2, 2,
            0, 1, 3, 4, 4, 4,
            0, 1, 3, 5, 6, 6,
            0, 1, 3, 5, 7, 8,
        ],
    );

    test_label!(
        Diagonals,
        1 x 4: [
            0,
            1,
            2,
            3,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
            0, 1, 2, 3,
            1, 2, 3, 4,
            2, 3, 4, 5,
            3, 4, 5, 6,
        ],
        4 x 6: [
            0, 1, 2, 3,
            1, 2, 3, 4,
            2, 3, 4, 5,
            3, 4, 5, 6,
            4, 5, 6, 7,
            5, 6, 7, 8,
        ],
        6 x 4: [
            0, 1, 2, 3, 4, 5,
            1, 2, 3, 4, 5, 6,
            2, 3, 4, 5, 6, 7,
            3, 4, 5, 6, 7, 8,
        ],
    );

    test_label!(
        LastTwoRows,
        1 x 4: [
            0,
            1,
            2,
            2,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 3, 4, 5,
            2, 3, 4, 5,
        ],
        4 x 6: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 5, 6, 7,
            4, 5, 6, 7,
        ],
        6 x 4: [
            0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1,
            2, 3, 4, 5, 6, 7,
            2, 3, 4, 5, 6, 7,
        ],
        4 x 2: [
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        2 x 4: [
            0, 0,
            1, 1,
            2, 3,
            2, 3,
        ],
    );

    test_label!(
        SplitLastTwoRows,
        1 x 4: [
            0,
            1,
            0,
            0,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        4 x 6: [
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        6 x 4: [
            0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1,
            0, 1, 2, 3, 4, 5,
            0, 1, 2, 3, 4, 5,
        ],
        4 x 2: [
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        2 x 4: [
            0, 0,
            1, 1,
            0, 1,
            0, 1,
        ],
    );

    test_label!(
        ConcentricRectangles,
        1 x 4: [
            0,
            0,
            0,
            0,
        ],
        4 x 1: [
            0, 0, 0, 0,
        ],
        4 x 4: [
            0, 0, 0, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 0, 0, 0,
        ],
        4 x 6: [
            0, 0, 0, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 0, 0, 0,
        ],
        6 x 4: [
            0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 0,
            0, 1, 1, 1, 1, 0,
            0, 0, 0, 0, 0, 0,
        ],
        5 x 5: [
            0, 0, 0, 0, 0,
            0, 1, 1, 1, 0,
            0, 1, 2, 1, 0,
            0, 1, 1, 1, 0,
            0, 0, 0, 0, 0,
        ],
        7 x 8: [
            0, 0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1, 0,
            0, 1, 2, 2, 2, 1, 0,
            0, 1, 2, 3, 2, 1, 0,
            0, 1, 2, 3, 2, 1, 0,
            0, 1, 2, 2, 2, 1, 0,
            0, 1, 1, 1, 1, 1, 0,
            0, 0, 0, 0, 0, 0, 0,
        ],
    );

    test_label!(
        Spiral,
        1 x 4: [
            0,
            0,
            0,
            0,
        ],
        4 x 1: [
            0, 0, 0, 0,
        ],
        4 x 4: [
            0, 0, 0, 1,
            3, 4, 5, 1,
            3, 7, 6, 1,
            3, 2, 2, 2,
        ],
        4 x 6: [
            0, 0, 0, 1,
            3, 4, 5, 1,
            3, 7, 5, 1,
            3, 7, 5, 1,
            3, 7, 6, 1,
            3, 2, 2, 2,
        ],
        6 x 4: [
            0, 0, 0, 0, 0, 1,
            3, 4, 4, 4, 5, 1,
            3, 7, 6, 6, 6, 1,
            3, 2, 2, 2, 2, 2,
        ],
    );

    test_label!(
        SpiralGrids,
        1 x 4: [
            0,
            1,
            2,
            3,
        ],
        4 x 1: [
            0, 1, 2, 3,
        ],
        4 x 4: [
             0,  1,  2,  3,
            11, 12, 13,  4,
            10, 15, 14,  5,
             9,  8,  7,  6,
        ],
        4 x 6: [
             0,  1,  2,  3,
            15, 16, 17,  4,
            14, 23, 18,  5,
            13, 22, 19,  6,
            12, 21, 20,  7,
            11, 10,  9,  8,
        ],
        6 x 4: [
             0,  1,  2,  3,  4,  5,
            15, 16, 17, 18, 19,  6,
            14, 23, 22, 21, 20,  7,
            13, 12, 11, 10,  9,  8,
        ],
        5 x 7: [
             0,  1,  2,  3,  4,
            19, 20, 21, 22,  5,
            18, 31, 32, 23,  6,
            17, 30, 33, 24,  7,
            16, 29, 34, 25,  8,
            15, 28, 27, 26,  9,
            14, 13, 12, 11, 10,
        ],
        7 x 5: [
             0,  1,  2,  3,  4,  5,  6,
            19, 20, 21, 22, 23, 24,  7,
            18, 31, 32, 33, 34, 25,  8,
            17, 30, 29, 28, 27, 26,  9,
            16, 15, 14, 13, 12, 11, 10,
        ],
    );

    test_label!(
        Checkerboard,
        1 x 4: [
            0,
            1,
            0,
            1,
        ],
        4 x 1: [
            0, 1, 0, 1,
        ],
        4 x 4: [
            0, 1, 0, 1,
            1, 0, 1, 0,
            0, 1, 0, 1,
            1, 0, 1, 0,
        ],
        4 x 6: [
            0, 1, 0, 1,
            1, 0, 1, 0,
            0, 1, 0, 1,
            1, 0, 1, 0,
            0, 1, 0, 1,
            1, 0, 1, 0,
        ],
        6 x 4: [
            0, 1, 0, 1, 0, 1,
            1, 0, 1, 0, 1, 0,
            0, 1, 0, 1, 0, 1,
            1, 0, 1, 0, 1, 0,
        ],
    );
}
