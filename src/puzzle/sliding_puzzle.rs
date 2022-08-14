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
    fn num_pieces(&self) -> usize {
        self.width() * self.height() - 1
    }

    #[must_use]
    fn gap_position(&self) -> usize;

    #[must_use]
    fn gap_position_xy(&self) -> (usize, usize);

    #[must_use]
    fn gap_position_x(&self) -> usize {
        self.gap_position_xy().0
    }

    #[must_use]
    fn gap_position_y(&self) -> usize {
        self.gap_position_xy().1
    }

    fn reset(&mut self);

    #[must_use]
    fn is_solved<T: SolvedState>(&self) -> bool {
        T::is_solved::<Piece, Self>(self)
    }

    #[must_use]
    fn solved_pos(&self, piece: Piece) -> usize {
        if piece == Piece::zero() {
            self.num_pieces()
        } else {
            piece.to_usize().expect("Failed to convert Piece to usize") - 1
        }
    }

    #[must_use]
    fn solved_pos_xy(&self, piece: Piece) -> (usize, usize) {
        let p = self.solved_pos(piece);
        let w = self.width();
        (p % w, p / w)
    }

    #[must_use]
    fn piece_at(&self, idx: usize) -> Piece;

    #[must_use]
    fn piece_at_xy(&self, x: usize, y: usize) -> Piece;

    fn swap_pieces(&mut self, idx1: usize, idx2: usize);
    fn swap_pieces_xy(&mut self, x1: usize, y1: usize, x2: usize, y2: usize);

    #[must_use]
    fn can_move_dir(&self, dir: Direction) -> bool {
        match dir {
            Direction::Up => self.gap_position_y() + 1 < self.height(),
            Direction::Left => self.gap_position_x() + 1 < self.width(),
            Direction::Down => self.gap_position_y() > 0,
            Direction::Right => self.gap_position_x() > 0,
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
        match mv.direction {
            Direction::Up => self.gap_position_y() + (mv.amount as usize) < self.height(),
            Direction::Left => self.gap_position_x() + (mv.amount as usize) < self.width(),
            Direction::Down => self.gap_position_y() >= mv.amount as usize,
            Direction::Right => self.gap_position_x() >= mv.amount as usize,
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

    fn apply_alg_unchecked(&mut self, alg: &Algorithm) {
        alg.moves
            .iter()
            .map(|&m| self.apply_move_unchecked(m))
            .collect()
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
