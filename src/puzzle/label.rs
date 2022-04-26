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
}

pub struct Rows;
pub struct Columns;
pub struct RowsSetwise;
pub struct ColumnsSetwise;

impl<Piece> Label<Piece> for Rows
where
    Piece: Into<u64>,
{
    fn position_label(width: usize, _height: usize, x: usize, y: usize) -> usize {
        x + width * y
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
}
