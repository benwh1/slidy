use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction, puzzle_move::Move},
    puzzle::solved_state::SolvedState,
};

pub trait SlidingPuzzle<Piece>
where
    Piece: Into<u64>,
    Self: Sized,
{
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn size(&self) -> (usize, usize) {
        (self.width(), self.height())
    }
    fn num_pieces(&self) -> usize {
        self.width() * self.height() - 1
    }

    fn gap_position(&self) -> usize;
    fn gap_position_xy(&self) -> (usize, usize);

    fn gap_position_x(&self) -> usize {
        self.gap_position_xy().0
    }
    fn gap_position_y(&self) -> usize {
        self.gap_position_xy().1
    }

    fn is_solved<T: SolvedState<Piece, Self>>(&self) -> bool {
        T::is_solved(self)
    }

    fn solved_pos(&self, piece: Piece) -> (usize, usize) {
        let p = piece.into() as usize;
        if p == 0 {
            let (w, h) = self.size();
            (w - 1, h - 1)
        } else {
            let w = self.width();
            ((p - 1) % w, (p - 1) / w)
        }
    }

    fn piece_at(&self, idx: usize) -> Piece;
    fn piece_at_xy(&self, x: usize, y: usize) -> Piece;

    fn swap_pieces(&mut self, idx1: usize, idx2: usize);
    fn swap_pieces_xy(&mut self, x1: usize, y1: usize, x2: usize, y2: usize);

    fn can_move_dir(&self, dir: Direction) -> bool {
        match dir {
            Direction::Up => self.gap_position_y() + 1 < self.height(),
            Direction::Left => self.gap_position_x() + 1 < self.width(),
            Direction::Down => self.gap_position_y() > 0,
            Direction::Right => self.gap_position_x() > 0,
        }
    }
    fn move_dir(&mut self, dir: Direction);

    fn can_apply_move(&self, mv: Move) -> bool {
        match mv.direction {
            Direction::Up => self.gap_position_y() + (mv.amount as usize) < self.height(),
            Direction::Left => self.gap_position_x() + (mv.amount as usize) < self.width(),
            Direction::Down => self.gap_position_y() >= mv.amount as usize,
            Direction::Right => self.gap_position_x() >= mv.amount as usize,
        }
    }
    fn apply_move(&mut self, mv: Move) {
        if !self.can_apply_move(mv) {
            return;
        }

        for _ in 0..mv.amount {
            self.move_dir(mv.direction);
        }
    }

    fn can_apply_alg(&self, alg: &Algorithm) -> bool {
        let (gx, gy) = self.gap_position_xy();
        let (mut gx, mut gy) = (gx as isize, gy as isize);

        for m in &alg.moves {
            let amount = m.amount as isize;
            match m.direction {
                Direction::Up => gy += amount,
                Direction::Left => gx += amount,
                Direction::Down => gy -= amount,
                Direction::Right => gx -= amount,
            }
            if gx < 0 || gx >= self.width() as isize || gy < 0 || gy > self.height() as isize {
                return false;
            }
        }
        true
    }
    fn apply_alg(&mut self, alg: &Algorithm) {
        alg.moves.iter().map(|&m| self.apply_move(m)).collect()
    }
}
