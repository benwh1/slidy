use num_traits::PrimInt;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction, puzzle_move::Move},
    puzzle::solved_state::SolvedState,
};

pub trait SlidingPuzzle<Piece>
where
    Piece: PrimInt,
    Self: Sized,
{
    #[must_use]
    fn width(&self) -> usize;

    #[must_use]
    fn height(&self) -> usize;

    #[must_use]
    fn size(&self) -> (usize, usize) {
        (self.width(), self.height())
    }

    #[must_use]
    fn area(&self) -> usize {
        self.width() * self.height()
    }

    #[must_use]
    fn num_pieces(&self) -> usize {
        self.width() * self.height() - 1
    }

    #[must_use]
    fn gap_position(&self) -> usize;

    #[must_use]
    fn gap_position_xy(&self) -> (usize, usize) {
        let pos = self.gap_position();
        let w = self.width();
        (pos % w, pos / w)
    }

    fn reset(&mut self);

    #[must_use]
    fn is_solved<T: SolvedState>(&self) -> bool {
        T::is_solved::<Piece, Self>(self)
    }

    #[must_use]
    fn solved_pos_unchecked(&self, piece: Piece) -> usize {
        if piece == Piece::zero() {
            self.num_pieces()
        } else {
            piece.to_usize().unwrap() - 1
        }
    }

    #[must_use]
    fn solved_pos(&self, piece: Piece) -> Option<usize> {
        let n = self.num_pieces();
        match piece.to_usize() {
            Some(p) if p <= n => Some(self.solved_pos_unchecked(piece)),
            _ => None,
        }
    }

    #[must_use]
    fn solved_pos_xy_unchecked(&self, piece: Piece) -> (usize, usize) {
        let p = self.solved_pos_unchecked(piece);
        let w = self.width();
        (p % w, p / w)
    }

    #[must_use]
    fn solved_pos_xy(&self, piece: Piece) -> Option<(usize, usize)> {
        let n = self.num_pieces();
        match piece.to_usize() {
            Some(p) if p <= n => Some(self.solved_pos_xy_unchecked(piece)),
            _ => None,
        }
    }

    #[must_use]
    fn piece_at_unchecked(&self, idx: usize) -> Piece;

    #[must_use]
    fn piece_at(&self, idx: usize) -> Option<Piece> {
        if idx <= self.num_pieces() {
            Some(self.piece_at_unchecked(idx))
        } else {
            None
        }
    }

    #[must_use]
    fn piece_at_xy_unchecked(&self, x: usize, y: usize) -> Piece {
        self.piece_at_unchecked(x + self.width() * y)
    }

    #[must_use]
    fn piece_at_xy(&self, x: usize, y: usize) -> Option<Piece> {
        if x < self.width() && y < self.height() {
            Some(self.piece_at_xy_unchecked(x, y))
        } else {
            None
        }
    }

    fn swap_pieces(&mut self, idx1: usize, idx2: usize);
    fn swap_pieces_xy(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let w = self.width();
        self.swap_pieces(x1 + w * y1, x2 + w * y2);
    }

    #[must_use]
    fn can_move_dir(&self, dir: Direction) -> bool {
        let (gx, gy) = self.gap_position_xy();
        match dir {
            Direction::Up => gy + 1 < self.height(),
            Direction::Left => gx + 1 < self.width(),
            Direction::Down => gy > 0,
            Direction::Right => gx > 0,
        }
    }

    fn move_dir_unchecked(&mut self, dir: Direction);

    fn move_dir(&mut self, dir: Direction) -> bool {
        if self.can_move_dir(dir) {
            self.move_dir_unchecked(dir);
            true
        } else {
            false
        }
    }

    #[must_use]
    fn can_apply_move(&self, mv: Move) -> bool {
        let (gx, gy) = self.gap_position_xy();
        let amount = mv.amount as usize;
        match mv.direction {
            Direction::Up => gy + amount < self.height(),
            Direction::Left => gx + amount < self.width(),
            Direction::Down => gy >= amount,
            Direction::Right => gx >= amount,
        }
    }

    fn apply_move_unchecked(&mut self, mv: Move) {
        for _ in 0..mv.amount {
            self.move_dir_unchecked(mv.direction);
        }
    }

    fn apply_move(&mut self, mv: Move) -> bool {
        if self.can_apply_move(mv) {
            self.apply_move_unchecked(mv);
            true
        } else {
            false
        }
    }

    #[must_use]
    fn can_apply_alg(&self, alg: &Algorithm) -> bool {
        let (mut gx, mut gy) = self.gap_position_xy();

        for m in &alg.moves {
            let amount = m.amount.try_into().unwrap();
            let (new_gx, new_gy) = match m.direction {
                Direction::Up => (Some(gx), gy.checked_add(amount)),
                Direction::Left => (gx.checked_add(amount), Some(gy)),
                Direction::Down => (Some(gx), gy.checked_sub(amount)),
                Direction::Right => (gx.checked_sub(amount), Some(gy)),
            };

            if let (Some(new_gx), Some(new_gy)) = (new_gx, new_gy) {
                (gx, gy) = (new_gx, new_gy);
            } else {
                return false;
            }
        }

        true
    }

    fn apply_alg_unchecked(&mut self, alg: &Algorithm) {
        for &m in &alg.moves {
            self.apply_move_unchecked(m);
        }
    }

    fn apply_alg(&mut self, alg: &Algorithm) -> bool {
        if self.can_apply_alg(alg) {
            self.apply_alg_unchecked(alg);
            true
        } else {
            false
        }
    }
}
