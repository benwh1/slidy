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
        width + height - if height > width { 1 } else { 0 }
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
    use crate::puzzle::label::{
        ColumnGrids, Columns, Diagonals, Fringe, Label, RowGrids, Rows, SquareFringe,
    };

    #[test]
    fn test_row_grids() {
        let pos = (0..12)
            .map(|i| <RowGrids as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| RowGrids::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(piece, vec![11, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

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

    #[test]
    fn test_rows() {
        let pos = (0..12)
            .map(|i| <Rows as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| Rows::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]);
        assert_eq!(piece, vec![2, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn test_columns() {
        let pos = (0..12)
            .map(|i| <Columns as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| Columns::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3]);
        assert_eq!(piece, vec![3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2]);
    }

    #[test]
    fn test_fringe() {
        let pos = (0..12)
            .map(|i| <Fringe as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| Fringe::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2, 2]);
        assert_eq!(piece, vec![2, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2]);
    }

    #[test]
    fn test_square_fringe() {
        let pos = (0..12)
            .map(|i| <SquareFringe as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| SquareFringe::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 3]);
        assert_eq!(piece, vec![3, 0, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2]);
    }

    #[test]
    fn test_diagonals() {
        let pos = (0..12)
            .map(|i| <Diagonals as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| Diagonals::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5]);
        assert_eq!(piece, vec![5, 0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4]);
    }
}
