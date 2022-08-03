use std::cmp::Ordering;

pub trait Label {
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize;
    fn num_labels(width: usize, height: usize) -> usize;
}

pub struct RowGrids;
pub struct Rows;
pub struct Fringe;
pub struct SquareFringe;
pub struct SplitFringe;
pub struct SplitSquareFringe;
pub struct Diagonals;
pub struct LastTwoRows;
pub struct SplitLastTwoRows;
pub struct ConcentricRectangles;
pub struct Spiral;

impl Label for RowGrids {
    fn position_label(width: usize, _height: usize, x: usize, y: usize) -> usize {
        x + width * y
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width * height
    }
}

impl Label for Rows {
    fn position_label(_width: usize, _height: usize, _x: usize, y: usize) -> usize {
        y
    }

    fn num_labels(_width: usize, height: usize) -> usize {
        height
    }
}

impl Label for Fringe {
    fn position_label(_width: usize, _height: usize, x: usize, y: usize) -> usize {
        x.min(y)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width.min(height)
    }
}

impl Label for SquareFringe {
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
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
                    diff + <Fringe as Label>::position_label(width, width, x, y - diff)
                }
            }
            Ordering::Equal => <Fringe as Label>::position_label(width, height, x, y),
            Ordering::Greater => <Self as Label>::position_label(height, width, y, x),
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        <Fringe as Label>::num_labels(width, height) + width.abs_diff(height)
    }
}

impl Label for SplitFringe {
    fn position_label(_width: usize, _height: usize, x: usize, y: usize) -> usize {
        // Which (non-split) fringe is (x, y) in?
        let fringe = x.min(y);

        // Is it in the row part or the horizontal part?
        let vertical_part = x < y;

        2 * fringe + if vertical_part { 1 } else { 0 }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        2 * width.min(height) - if height > width { 0 } else { 1 }
    }
}

impl Label for SplitSquareFringe {
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        let d = width.abs_diff(height);

        match width.cmp(&height) {
            Ordering::Less => {
                if y < d {
                    y
                } else {
                    d + <SplitFringe as Label>::position_label(width, width, x, y - d)
                }
            }
            Ordering::Equal => <SplitFringe as Label>::position_label(width, width, x, y),
            Ordering::Greater => {
                if x < d {
                    x
                } else {
                    d + <SplitFringe as Label>::position_label(height, height, x - d, y)
                }
            }
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        let diff = width.abs_diff(height);
        let min = width.min(height);

        diff + <SplitFringe as Label>::num_labels(min, min)
    }
}

impl Label for Diagonals {
    fn position_label(_width: usize, _height: usize, x: usize, y: usize) -> usize {
        x + y
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width + height - 1
    }
}

impl Label for LastTwoRows {
    fn position_label(_width: usize, height: usize, x: usize, y: usize) -> usize {
        if y < height - 2 {
            y
        } else {
            height - 2 + x
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width + height - 2
    }
}

impl Label for SplitLastTwoRows {
    fn position_label(_width: usize, height: usize, x: usize, y: usize) -> usize {
        if y < height - 2 {
            y
        } else {
            x
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width.max(height - 2)
    }
}

impl Label for ConcentricRectangles {
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        x.min(y).min(width - 1 - x).min(height - 1 - y)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width.min(height).div_ceil(2)
    }
}

impl Label for Spiral {
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        let rect_label = ConcentricRectangles::position_label(width, height, x, y);

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

    fn num_labels(width: usize, height: usize) -> usize {
        // 4 * number of rectangles of width and height > 1, plus 1 if the innermost rectangle has
        // width or height 1.
        4 * width.min(height).div_floor(2) + if width.min(height) % 2 == 1 { 1 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    macro_rules! test_label {
        (fn $name:ident, $label:ty, $w:literal x $h:literal, $labels:expr) => {
            #[test]
            fn $name() {
                let labels = (0..$w * $h)
                    .map(|i| <$label as Label>::position_label($w, $h, i % $w, i / $w))
                    .collect::<Vec<_>>();
                let num_labels = <$label as Label>::num_labels($w, $h);
                let expected_num_labels = $labels.iter().max().unwrap() + 1;
                assert_eq!(labels, $labels);
                assert_eq!(num_labels, expected_num_labels);
            }
        };

        ($label:ty, $($w:literal x $h:literal : $labels:expr),+ $(,)?) => {
            ::paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::label::label::{Label, $label};

                    $(test_label!( fn [< test_ $label:snake _ $w x $h >] , $label, $w x $h, $labels);)*
                }
            }
        };
    }

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
}
