use std::cmp::Ordering;

use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LabelError {
    #[error("InvalidSize: {width}x{height} is not a valid size")]
    InvalidSize { width: usize, height: usize },

    #[error(
        "PositionOutOfBounds: position ({x}, {y}) is out of bounds on a {width}x{height} puzzle."
    )]
    PositionOutOfBounds {
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    },
}

/// Provides a function mapping an `(x, y)` coordinate on a puzzle to a number which we call the
/// label of `(x, y)`.
pub trait Label {
    /// Checks if this `Label` can be used with a given puzzle size.
    #[must_use]
    fn is_valid_size(&self, width: usize, height: usize) -> bool;

    /// See also: [`Self::position_label`].
    ///
    /// This function may not check whether `width x height` is a valid puzzle size for the label,
    /// or whether `(x, y)` is within the bounds of the puzzle. If these conditions are not
    /// satisfied, the function may panic or return an invalid label, e.g. an integer greater than
    /// or equal to `self.num_labels_unchecked(width, height)`.
    #[must_use]
    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize;

    /// Returns the label of `(x, y)` on a `width x height` puzzle.
    ///
    /// The label must be an integer from 0 to `self.num_labels_unchecked(width, height) - 1`.
    fn position_label(
        &self,
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> Result<usize, LabelError> {
        if !self.is_valid_size(width, height) {
            Err(LabelError::InvalidSize { width, height })
        } else if x >= width || y >= height {
            Err(LabelError::PositionOutOfBounds {
                width,
                height,
                x,
                y,
            })
        } else {
            Ok(self.position_label_unchecked(width, height, x, y))
        }
    }

    /// See also: [`Self::num_labels`].
    ///
    /// This function may not check whether `width x height` is a valid puzzle size for the label.
    /// If it is not, the function may panic or return an invalid number.
    #[must_use]
    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize;

    /// Returns the total number of distinct labels across all `(x, y)` positions in the puzzle.
    fn num_labels(&self, width: usize, height: usize) -> Result<usize, LabelError> {
        if self.is_valid_size(width, height) {
            Ok(self.num_labels_unchecked(width, height))
        } else {
            Err(LabelError::InvalidSize { width, height })
        }
    }
}

/// Marker trait for [`Label`]s that assign distinct labels to every position.
/// These will always have `num_labels(width, height) == width * height`.
pub trait BijectiveLabel: Label {}

macro_rules! define_label {
    ($($(#[$annot:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$annot])*
            ///
            /// Valid with all puzzle sizes.
            #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Assigns labels in a spiral pattern: the top row (minus the top right piece) gets label 0,
    /// then the right column (minus the bottom right piece) gets label 1, then the bottom row
    /// (minus the bottom left piece) gets label 2, etc.
    Spiral,
    /// Same as [`Spiral`] but every piece gets a distinct label.
    SpiralGrids,
);

impl BijectiveLabel for RowGrids {}
impl BijectiveLabel for FringeGrids {}
impl BijectiveLabel for SpiralGrids {}

impl Label for Trivial {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, _: usize, _: usize, _: usize, _: usize) -> usize {
        0
    }

    fn num_labels_unchecked(&self, _: usize, _: usize) -> usize {
        1
    }
}

impl Label for RowGrids {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, width: usize, _height: usize, x: usize, y: usize) -> usize {
        x + width * y
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width * height
    }
}

impl Label for Rows {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(
        &self,
        _width: usize,
        _height: usize,
        _x: usize,
        y: usize,
    ) -> usize {
        y
    }

    fn num_labels_unchecked(&self, _width: usize, height: usize) -> usize {
        height
    }
}

impl Label for Fringe {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, _width: usize, _height: usize, x: usize, y: usize) -> usize {
        x.min(y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width.min(height)
    }
}

impl Label for FringeGrids {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        // Which (non-split) fringe is (x, y) in?
        let fringe = x.min(y);

        // Is it in the row part or the horizontal part?
        let vertical_part = x < y;

        // Sum w+h-1-2k, k=0..f-1 = f(w+h-f) = number of tiles in previous fringes
        let previous_fringes = fringe * (width + height - fringe);

        // How many pieces before this one in the current fringe?
        let current_fringe = if vertical_part {
            width - fringe + y - x - 1
        } else {
            x - y
        };

        previous_fringes + current_fringe
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width * height
    }
}

impl Label for SquareFringe {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
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
                    diff + Fringe.position_label_unchecked(width, width, x, y - diff)
                }
            }
            Ordering::Equal => Fringe.position_label_unchecked(width, height, x, y),
            Ordering::Greater => self.position_label_unchecked(height, width, y, x),
        }
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        Fringe.num_labels_unchecked(width, height) + width.abs_diff(height)
    }
}

impl Label for SplitFringe {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, _width: usize, _height: usize, x: usize, y: usize) -> usize {
        // Which (non-split) fringe is (x, y) in?
        let fringe = x.min(y);

        // Is it in the row part or the horizontal part?
        let vertical_part = x < y;

        2 * fringe + if vertical_part { 1 } else { 0 }
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        2 * width.min(height) - if height > width { 0 } else { 1 }
    }
}

impl Label for SplitSquareFringe {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        let d = width.abs_diff(height);

        match width.cmp(&height) {
            Ordering::Less => {
                if y < d {
                    y
                } else {
                    d + SplitFringe.position_label_unchecked(width, width, x, y - d)
                }
            }
            Ordering::Equal => SplitFringe.position_label_unchecked(width, width, x, y),
            Ordering::Greater => {
                if x < d {
                    x
                } else {
                    d + SplitFringe.position_label_unchecked(height, height, x - d, y)
                }
            }
        }
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        let diff = width.abs_diff(height);
        let min = width.min(height);

        diff + SplitFringe.num_labels_unchecked(min, min)
    }
}

impl Label for Diagonals {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, _width: usize, _height: usize, x: usize, y: usize) -> usize {
        x + y
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width + height - 1
    }
}

impl Label for LastTwoRows {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, _width: usize, height: usize, x: usize, y: usize) -> usize {
        if y < height - 2 {
            y
        } else {
            height - 2 + x
        }
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width + height - 2
    }
}

impl Label for SplitLastTwoRows {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, _width: usize, height: usize, x: usize, y: usize) -> usize {
        if y < height - 2 {
            y
        } else {
            x
        }
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width.max(height - 2)
    }
}

impl Label for ConcentricRectangles {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        x.min(y).min(width - 1 - x).min(height - 1 - y)
    }

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width.min(height).div_ceil(2)
    }
}

impl Label for Spiral {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        let rect_label = ConcentricRectangles.position_label_unchecked(width, height, x, y);

        // Calculate the values (x, y, width, height) if we were to strip off any outer rectangles
        // from the puzzle.
        // e.g. the piece in position (1, 1) on a 4x5 puzzle becomes the piece in position (0, 0)
        // on a 2x3 puzzle when we remove the outer rectangle, so we would have
        // (rx, ry, rw, rh) = (0, 0, 2, 3).
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

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        // 4 * number of rectangles of width and height > 1, plus 1 if the innermost rectangle has
        // width or height 1.
        4 * width.min(height).div_floor(2) + if width.min(height) % 2 == 1 { 1 } else { 0 }
    }
}

impl Label for SpiralGrids {
    fn is_valid_size(&self, _width: usize, _height: usize) -> bool {
        true
    }

    fn position_label_unchecked(&self, width: usize, height: usize, x: usize, y: usize) -> usize {
        let rect_label = ConcentricRectangles.position_label_unchecked(width, height, x, y);

        // See `Spiral::position_label_unchecked`
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
        let rect_pieces = if rw.min(rh) == 1 || (ry == 0 && rx < rw - 1) {
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

    fn num_labels_unchecked(&self, width: usize, height: usize) -> usize {
        width * height
    }
}

#[cfg(test)]
mod tests {
    macro_rules! test_label {
        ($label:ty, $($w:literal x $h:literal : $labels:expr),+ $(,)?) => {
            paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::label::label::{Label, $label};

                    $(#[test]
                    fn [< test_ $label:snake _ $w x $h >] () {
                        let labels = (0..$w * $h)
                            .map(|i| $label.position_label_unchecked($w, $h, i % $w, i / $w))
                            .collect::<Vec<_>>();
                        let num_labels = $label.num_labels_unchecked($w, $h);
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
        4 x 4: vec![
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
        ],
    );

    test_label!(
        RowGrids,
        4 x 4: vec![
             0,  1,  2,  3,
             4,  5,  6,  7,
             8,  9, 10, 11,
            12, 13, 14, 15,
        ],
        4 x 6: vec![
             0,  1,  2,  3,
             4,  5,  6,  7,
             8,  9, 10, 11,
            12, 13, 14, 15,
            16, 17, 18, 19,
            20, 21, 22, 23,
        ],
        6 x 4: vec![
             0,  1,  2,  3,  4,  5,
             6,  7,  8,  9, 10, 11,
            12, 13, 14, 15, 16, 17,
            18, 19, 20, 21, 22, 23,
        ],
    );

    test_label!(
        Rows,
        4 x 4: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 4, 4, 4,
            5, 5, 5, 5,
        ],
        6 x 4: vec![
            0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1,
            2, 2, 2, 2, 2, 2,
            3, 3, 3, 3, 3, 3,
        ],
    );

    test_label!(
        Fringe,
        4 x 4: vec![
            0, 0, 0, 0,
            0, 1, 1, 1,
            0, 1, 2, 2,
            0, 1, 2, 3,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            0, 1, 1, 1,
            0, 1, 2, 2,
            0, 1, 2, 3,
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        6 x 4: vec![
            0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 1,
            0, 1, 2, 2, 2, 2,
            0, 1, 2, 3, 3, 3
        ],
    );

    test_label!(
        FringeGrids,
        4 x 4: vec![
            0,  1,  2,  3,
            4,  7,  8,  9,
            5, 10, 12, 13,
            6, 11, 14, 15,
        ],
        4 x 6: vec![
            0,  1,  2,  3,
            4,  9, 10, 11,
            5, 12, 16, 17,
            6, 13, 18, 21,
            7, 14, 19, 22,
            8, 15, 20, 23,
        ],
        6 x 4: vec![
            0,  1,  2,  3,  4,  5,
            6,  9, 10, 11, 12, 13,
            7, 14, 16, 17, 18, 19,
            8, 15, 20, 21, 22, 23,
        ],
    );

    test_label!(
        SquareFringe,
        4 x 4: vec![
            0, 0, 0, 0,
            0, 1, 1, 1,
            0, 1, 2, 2,
            0, 1, 2, 3,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            2, 3, 3, 3,
            2, 3, 4, 4,
            2, 3, 4, 5,
        ],
        6 x 4: vec![
            0, 1, 2, 2, 2, 2,
            0, 1, 2, 3, 3, 3,
            0, 1, 2, 3, 4, 4,
            0, 1, 2, 3, 4, 5,
        ],
    );

    test_label!(
        SplitFringe,
        4 x 4: vec![
            0, 0, 0, 0,
            1, 2, 2, 2,
            1, 3, 4, 4,
            1, 3, 5, 6,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            1, 2, 2, 2,
            1, 3, 4, 4,
            1, 3, 5, 6,
            1, 3, 5, 7,
            1, 3, 5, 7,
        ],
        6 x 4: vec![
            0, 0, 0, 0, 0, 0,
            1, 2, 2, 2, 2, 2,
            1, 3, 4, 4, 4, 4,
            1, 3, 5, 6, 6, 6,
        ],
    );

    test_label!(
        SplitSquareFringe,
        4 x 4: vec![
            0, 0, 0, 0,
            1, 2, 2, 2,
            1, 3, 4, 4,
            1, 3, 5, 6,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 4, 4, 4,
            3, 5, 6, 6,
            3, 5, 7, 8,
        ],
        6 x 4: vec![
            0, 1, 2, 2, 2, 2,
            0, 1, 3, 4, 4, 4,
            0, 1, 3, 5, 6, 6,
            0, 1, 3, 5, 7, 8,
        ],
    );

    test_label!(
        Diagonals,
        4 x 4: vec![
            0, 1, 2, 3,
            1, 2, 3, 4,
            2, 3, 4, 5,
            3, 4, 5, 6,
        ],
        4 x 6: vec![
            0, 1, 2, 3,
            1, 2, 3, 4,
            2, 3, 4, 5,
            3, 4, 5, 6,
            4, 5, 6, 7,
            5, 6, 7, 8,
        ],
        6 x 4: vec![
            0, 1, 2, 3, 4, 5,
            1, 2, 3, 4, 5, 6,
            2, 3, 4, 5, 6, 7,
            3, 4, 5, 6, 7, 8,
        ],
    );

    test_label!(
        LastTwoRows,
        4 x 4: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 3, 4, 5,
            2, 3, 4, 5,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            4, 5, 6, 7,
            4, 5, 6, 7,
        ],
        6 x 4: vec![
            0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1,
            2, 3, 4, 5, 6, 7,
            2, 3, 4, 5, 6, 7,
        ],
        4 x 2: vec![
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        2 x 4: vec![
            0, 0,
            1, 1,
            2, 3,
            2, 3,
        ],
    );

    test_label!(
        SplitLastTwoRows,
        4 x 4: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            1, 1, 1, 1,
            2, 2, 2, 2,
            3, 3, 3, 3,
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        6 x 4: vec![
            0, 0, 0, 0, 0, 0,
            1, 1, 1, 1, 1, 1,
            0, 1, 2, 3, 4, 5,
            0, 1, 2, 3, 4, 5,
        ],
        4 x 2: vec![
            0, 1, 2, 3,
            0, 1, 2, 3,
        ],
        2 x 4: vec![
            0, 0,
            1, 1,
            0, 1,
            0, 1,
        ],
    );

    test_label!(
        ConcentricRectangles,
        4 x 4: vec![
            0, 0, 0, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 0, 0, 0,
        ],
        4 x 6: vec![
            0, 0, 0, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 1, 1, 0,
            0, 0, 0, 0,
        ],
        6 x 4: vec![
            0, 0, 0, 0, 0, 0,
            0, 1, 1, 1, 1, 0,
            0, 1, 1, 1, 1, 0,
            0, 0, 0, 0, 0, 0,
        ],
        5 x 5: vec![
            0, 0, 0, 0, 0,
            0, 1, 1, 1, 0,
            0, 1, 2, 1, 0,
            0, 1, 1, 1, 0,
            0, 0, 0, 0, 0,
        ],
        7 x 8: vec![
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
        4 x 4: vec![
            0, 0, 0, 1,
            3, 4, 5, 1,
            3, 7, 6, 1,
            3, 2, 2, 2,
        ],
        4 x 6: vec![
            0, 0, 0, 1,
            3, 4, 5, 1,
            3, 7, 5, 1,
            3, 7, 5, 1,
            3, 7, 6, 1,
            3, 2, 2, 2,
        ],
        6 x 4: vec![
            0, 0, 0, 0, 0, 1,
            3, 4, 4, 4, 5, 1,
            3, 7, 6, 6, 6, 1,
            3, 2, 2, 2, 2, 2,
        ],
    );

    test_label!(
        SpiralGrids,
        4 x 4: vec![
             0,  1,  2,  3,
            11, 12, 13,  4,
            10, 15, 14,  5,
             9,  8,  7,  6,
        ],
        4 x 6: vec![
             0,  1,  2,  3,
            15, 16, 17,  4,
            14, 23, 18,  5,
            13, 22, 19,  6,
            12, 21, 20,  7,
            11, 10,  9,  8,
        ],
        6 x 4: vec![
             0,  1,  2,  3,  4,  5,
            15, 16, 17, 18, 19,  6,
            14, 23, 22, 21, 20,  7,
            13, 12, 11, 10,  9,  8,
        ],
    );
}
