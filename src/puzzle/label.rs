pub trait Label<Piece>
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize;
    fn piece_label(width: usize, height: usize, piece: Piece) -> usize;
    fn num_labels(width: usize, height: usize) -> usize;
}

pub struct Rows;
pub struct Columns;
pub struct RowsSetwise;
pub struct ColumnsSetwise;

impl<Piece> Label<Piece> for Rows
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        x + width * y
    }

    fn piece_label(width: usize, height: usize, piece: Piece) -> usize {
        let p = piece.into() as usize;
        if p == 0 {
            width * height - 1
        } else {
            p - 1
        }
    }

    fn num_labels(width: usize, height: usize) -> usize {
        width * height
    }
}

impl<Piece> Label<Piece> for Columns
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
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
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        if (x, y) == (width - 1, height - 1) {
            height
        } else {
            y
        }
    }

    fn piece_label(width: usize, height: usize, piece: Piece) -> usize {
        let p = piece.into() as usize;
        if p == 0 {
            height
        } else {
            p / width
        }
    }

    fn num_labels(_width: usize, height: usize) -> usize {
        height + 1
    }
}

impl<Piece> Label<Piece> for ColumnsSetwise
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, height: usize, x: usize, y: usize) -> usize {
        if (x, y) == (width - 1, height - 1) {
            width
        } else {
            x
        }
    }

    fn piece_label(width: usize, height: usize, piece: Piece) -> usize {
        let p = piece.into() as usize;
        if p == 0 {
            width
        } else {
            p % width
        }
    }

    fn num_labels(width: usize, _height: usize) -> usize {
        width + 1
    }
}
