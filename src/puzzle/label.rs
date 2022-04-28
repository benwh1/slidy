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

pub struct Rows;
pub struct Columns;
pub struct RowsSetwise;
pub struct ColumnsSetwise;
pub struct FringeSetwise;
pub struct SquareFringeSetwise;
pub struct DiagonalsSetwise;

impl<Piece> Label<Piece> for Rows
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

impl<Piece> Label<Piece> for Columns
where
    Piece: Into<u64>,
{
    fn position_label(_width: usize, height: usize, x: usize, y: usize) -> usize {
        y + height * x
    }

    fn piece_label(width: usize, height: usize, piece: Piece) -> usize {
        Rows::piece_label(width, height, piece)
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width * height
    }
}

impl<Piece> Label<Piece> for RowsSetwise
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

impl<Piece> Label<Piece> for ColumnsSetwise
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

impl<Piece> Label<Piece> for FringeSetwise
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

impl<Piece> Label<Piece> for SquareFringeSetwise
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
                    diff + <FringeSetwise as Label<Piece>>::position_label(
                        width,
                        width,
                        x,
                        y - diff,
                    )
                }
            }
            Ordering::Equal => <FringeSetwise as Label<Piece>>::position_label(width, height, x, y),
            Ordering::Greater => <Self as Label<Piece>>::position_label(height, width, y, x),
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        <FringeSetwise as Label<Piece>>::num_labels(width, height) + width.abs_diff(height)
    }
}

impl<Piece> Label<Piece> for DiagonalsSetwise
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
        Columns, ColumnsSetwise, DiagonalsSetwise, FringeSetwise, Label, Rows, RowsSetwise,
        SquareFringeSetwise,
    };

    #[test]
    fn test_rows() {
        let pos = (0..12)
            .map(|i| <Rows as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| Rows::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(piece, vec![11, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_columns() {
        let pos = (0..12)
            .map(|i| <Columns as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| Columns::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 3, 6, 9, 1, 4, 7, 10, 2, 5, 8, 11]);
        assert_eq!(piece, vec![11, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_rows_setwise() {
        let pos = (0..12)
            .map(|i| <RowsSetwise as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| RowsSetwise::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]);
        assert_eq!(piece, vec![2, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn test_columns_setwise() {
        let pos = (0..12)
            .map(|i| <ColumnsSetwise as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| ColumnsSetwise::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3]);
        assert_eq!(piece, vec![3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2]);
    }

    #[test]
    fn test_fringe_setwise() {
        let pos = (0..12)
            .map(|i| <FringeSetwise as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| FringeSetwise::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2, 2]);
        assert_eq!(piece, vec![2, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2]);
    }

    #[test]
    fn test_square_fringe_setwise() {
        let pos = (0..12)
            .map(|i| <SquareFringeSetwise as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| SquareFringeSetwise::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 3]);
        assert_eq!(piece, vec![3, 0, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2]);
    }

    #[test]
    fn test_diagonals_setwise() {
        let pos = (0..12)
            .map(|i| <DiagonalsSetwise as Label<u64>>::position_label(4, 3, i % 4, i / 4))
            .collect::<Vec<_>>();
        let piece = (0..12)
            .map(|i: u64| DiagonalsSetwise::piece_label(4, 3, i))
            .collect::<Vec<_>>();
        assert_eq!(pos, vec![0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5]);
        assert_eq!(piece, vec![5, 0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4]);
    }
}
