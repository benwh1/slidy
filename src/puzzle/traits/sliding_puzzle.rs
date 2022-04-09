use crate::algorithm::puzzle_move::Direction;

pub trait SlidingPuzzle<Piece>
where
    Piece: Copy,
{
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn num_pieces(&self) -> usize {
        self.width() * self.height() - 1
    }

    fn gap_piece() -> Piece;

    fn gap_position(&self) -> usize;
    fn gap_position_xy(&self) -> (usize, usize);

    fn gap_position_x(&self) -> usize {
        self.gap_position_xy().0
    }
    fn gap_position_y(&self) -> usize {
        self.gap_position_xy().1
    }

    fn piece_at(&self, idx: usize) -> Piece;
    fn piece_at_xy(&self, x: usize, y: usize) -> Piece;

    fn piece_at_mut(&mut self, idx: usize) -> &mut Piece;
    fn piece_at_xy_mut(&mut self, x: usize, y: usize) -> &mut Piece;

    fn can_move_dir(&self, dir: Direction) -> bool;
    fn move_dir(&mut self, dir: Direction);
}
