use std::cmp::Ordering;

pub trait Label<Piece>
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize;
    fn piece_label(width: usize, height: usize, piece: Piece) -> usize {
        let piece = piece.into() as usize;
        let p = if piece == 0 { width * height } else { piece } - 1;
        Self::position_label(width, height, p % width, p / width)
    }

    fn num_labels(width: usize, height: usize) -> usize;
}

pub struct RowGrids;
pub struct ColumnGrids;
pub struct Rows;
pub struct Columns;
pub struct Fringe;
pub struct SquareFringe;
pub struct SplitFringe;
pub struct SplitSquareFringe;
pub struct Diagonals;

impl<Piece> Label<Piece> for RowGrids
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, _height: usize, x: usize, y: usize) -> usize {
        x + width * y
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width * height
    }
}

impl<Piece> Label<Piece> for ColumnGrids
where
    Piece: Into<u64>,
{
    fn position_label(_width: usize, height: usize, x: usize, y: usize) -> usize {
        y + height * x
    }

    fn piece_label(width: usize, height: usize, piece: Piece) -> usize {
        RowGrids::piece_label(width, height, piece)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width * height
    }
}

impl<Piece> Label<Piece> for Rows
where
    Piece: Into<u64>,
{
    fn position_label(_width: usize, _height: usize, _x: usize, y: usize) -> usize {
        y
    }

    fn num_labels(_width: usize, height: usize) -> usize {
        height
    }
}

impl<Piece> Label<Piece> for Columns
where
    Piece: Into<u64>,
{
    fn position_label(_width: usize, _height: usize, x: usize, _y: usize) -> usize {
        x
    }

    fn num_labels(width: usize, _height: usize) -> usize {
        width
    }
}

impl<Piece> Label<Piece> for Fringe
where
    Piece: Into<u64>,
{
    fn position_label(_width: usize, _height: usize, x: usize, y: usize) -> usize {
        x.min(y)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width.min(height)
    }
}

impl<Piece> Label<Piece> for SquareFringe
where
    Piece: Into<u64>,
{
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
                    diff + <Fringe as Label<Piece>>::position_label(width, width, x, y - diff)
                }
            }
            Ordering::Equal => <Fringe as Label<Piece>>::position_label(width, height, x, y),
            Ordering::Greater => <Self as Label<Piece>>::position_label(height, width, y, x),
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        <Fringe as Label<Piece>>::num_labels(width, height) + width.abs_diff(height)
    }
}

impl<Piece> Label<Piece> for SplitFringe
where
    Piece: Into<u64>,
{
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

impl<Piece> Label<Piece> for SplitSquareFringe
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        let d = width.abs_diff(height);

        match width.cmp(&height) {
            Ordering::Less => {
                if y < d {
                    y
                } else {
                    d + <SplitFringe as Label<Piece>>::position_label(width, width, x, y - d)
                }
            }
            Ordering::Equal => <SplitFringe as Label<Piece>>::position_label(width, width, x, y),
            Ordering::Greater => {
                if x < d {
                    x
                } else {
                    d + <SplitFringe as Label<Piece>>::position_label(height, height, x - d, y)
                }
            }
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        let diff = width.abs_diff(height);
        let min = width.min(height);

        diff + <SplitFringe as Label<Piece>>::num_labels(min, min)
    }
}

impl<Piece> Label<Piece> for Diagonals
where
    Piece: Into<u64>,
{
    fn position_label(_width: usize, _height: usize, x: usize, y: usize) -> usize {
        x + y
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width + height - 1
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::label::{ColumnGrids, Label};

    macro_rules! test_label {
        (fn $name:ident, $label:ty, $w:literal x $h:literal, $pos_label:expr, $piece_label:expr, $num_labels:expr) => {
            #[test]
            fn $name() {
                let wh = $w * $h;
                let pos = (0..wh)
                    .map(|i| <$label as Label<u64>>::position_label($w, $h, i % $w, i / $w))
                    .collect::<Vec<_>>();
                let piece = (0..wh)
                    .map(|i| <$label as Label<u64>>::piece_label($w, $h, i as u64))
                    .collect::<Vec<_>>();
                let num = <$label as Label<u64>>::num_labels($w, $h);
                assert_eq!(pos, $pos_label);
                assert_eq!(piece, $piece_label);
                assert_eq!(num, $num_labels);
            }
        };

        (fn $name:ident, $label:ty, $w:literal x $h:literal, $pos_label:expr) => {
            test_label!(fn $name, $label, $w x $h, $pos_label, {
                let mut v = $pos_label.clone();
                let last = v.pop().unwrap();
                let mut w = vec![last];
                w.append(&mut v);
                w
            }, $pos_label.iter().max().unwrap() + 1);
        };

        ($label:ty, $($w:literal x $h:literal : $pos:expr,)*) => {
            ::paste::paste! {
                mod [< $label:snake >] {
                    use crate::puzzle::label::{Label, $label};

                    $(test_label!( fn [< test_ $label:snake _ $w x $h >] , $label, $w x $h, $pos);)*
                }
            }
        };
    }

    test_label!(
        RowGrids,
        4 x 4: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        4 x 6: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
        6 x 4: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23],
    );

    #[test]
    fn test_column_grids() {
        let pos = (0..12)
            .map(|i| <ColumnGrids as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| ColumnGrids::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 3, 6, 9, 1, 4, 7, 10, 2, 5, 8, 11]);
        assert_eq!(piece, vec![11, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    test_label!(
        Rows,
        4 x 4: vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3],
        4 x 6: vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5],
        6 x 4: vec![0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3],
    );

    test_label!(
        Columns,
        4 x 4: vec![0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3],
        4 x 6: vec![0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3],
        6 x 4: vec![0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5],
    );

    test_label!(
        Fringe,
        4 x 4: vec![0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 3],
        4 x 6: vec![0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3],
        6 x 4: vec![0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 1, 2, 2, 2, 2, 0, 1, 2, 3, 3, 3],
    );

    test_label!(
        SquareFringe,
        4 x 4: vec![0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 3],
        4 x 6: vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 2, 3, 4, 4, 2, 3, 4, 5],
        6 x 4: vec![0, 1, 2, 2, 2, 2, 0, 1, 2, 3, 3, 3, 0, 1, 2, 3, 4, 4, 0, 1, 2, 3, 4, 5],
    );

    test_label!(
        SplitFringe,
        4 x 4: vec![0, 0, 0, 0, 1, 2, 2, 2, 1, 3, 4, 4, 1, 3, 5, 6],
        4 x 6: vec![0, 0, 0, 0, 1, 2, 2, 2, 1, 3, 4, 4, 1, 3, 5, 6, 1, 3, 5, 7, 1, 3, 5, 7],
        6 x 4: vec![0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 2, 2, 1, 3, 4, 4, 4, 4, 1, 3, 5, 6, 6, 6],
    );

    test_label!(
        SplitSquareFringe,
        4 x 4: vec![0, 0, 0, 0, 1, 2, 2, 2, 1, 3, 4, 4, 1, 3, 5, 6],
        4 x 6: vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 4, 4, 4, 3, 5, 6, 6, 3, 5, 7, 8],
        6 x 4: vec![0, 1, 2, 2, 2, 2, 0, 1, 3, 4, 4, 4, 0, 1, 3, 5, 6, 6, 0, 1, 3, 5, 7, 8],
    );

    test_label!(
        Diagonals,
        4 x 4: vec![0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6],
        4 x 6: vec![0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 7, 8],
        6 x 4: vec![0, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 6, 2, 3, 4, 5, 6, 7, 3, 4, 5, 6, 7, 8],
    );
}
