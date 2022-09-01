use num_traits::PrimInt;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction, puzzle_move::Move},
    puzzle::{label::label::BijectiveLabel, solved_state::SolvedState},
};

use super::label::label::RowGrids;

/// The main trait defining the properties of a sliding puzzle.
///
/// The pieces are represented by the integers from 1 to `N` inclusive, where `N` is the number of
/// pieces. The empty space is represented by 0.
///
/// The position of a piece within the puzzle is represented by an integer from 0 to `N`, ordered
/// from top to bottom, left to right. For example:
///
/// - Position 0 is the top left
/// - Position 1 is to the right of position 0
/// - If `w` is the width of the puzzle, then position `w-1` is the top right corner and position
/// `w` is below position 0
/// - Position `N` is the bottom right corner
pub trait SlidingPuzzle<Piece>
where
    Piece: PrimInt,
    Self: Sized,
{
    /// Width of the puzzle.
    #[must_use]
    fn width(&self) -> usize;

    /// Height of the puzzle.
    #[must_use]
    fn height(&self) -> usize;

    /// Size of the puzzle in the form `(width, height)`.
    #[must_use]
    fn size(&self) -> (usize, usize) {
        (self.width(), self.height())
    }

    /// Product of the width and height.
    #[must_use]
    fn area(&self) -> usize {
        self.width() * self.height()
    }

    /// Number of pieces in the puzzle.
    #[must_use]
    fn num_pieces(&self) -> usize {
        self.width() * self.height() - 1
    }

    /// Position of the empty space.
    #[must_use]
    fn gap_position(&self) -> usize {
        (0..self.area())
            .position(|idx| self.piece_at_unchecked(idx) == Piece::zero())
            .unwrap()
    }

    /// Position of the empty space as (x, y) coordinates.
    #[must_use]
    fn gap_position_xy(&self) -> (usize, usize) {
        let pos = self.gap_position();
        let w = self.width();
        (pos % w, pos / w)
    }

    /// Reset the puzzle to the default state.
    fn reset(&mut self) {
        self.reset_to_label(&RowGrids);
    }

    /// Reset the puzzle to the solved state as defined by a [`BijectiveLabel`]
    fn reset_to_label<L: BijectiveLabel>(&mut self, label: &L) {
        let (w, h) = self.size();
        let area = Piece::from(w * h).unwrap();
        for y in 0..h {
            for x in 0..w {
                let label = label.position_label_unchecked(w, h, x, y);
                let piece = {
                    let a = Piece::from(label).unwrap() + Piece::one();
                    if a == area {
                        Piece::zero()
                    } else {
                        a
                    }
                };
                self.set_piece_xy_unchecked(x, y, piece);
            }
        }
    }

    /// Check if the puzzle is solved.
    #[must_use]
    fn is_solved(&self) -> bool {
        RowGrids.is_solved(self)
    }

    /// The position of `piece` when the puzzle is solved.
    ///
    /// This function does not check whether `piece` is within the valid range for the puzzle.
    /// If `piece` is outside the valid range, the function may return a wrong result or panic.
    #[must_use]
    fn solved_pos_unchecked(&self, piece: Piece) -> usize {
        if piece == Piece::zero() {
            self.num_pieces()
        } else {
            piece.to_usize().unwrap() - 1
        }
    }

    /// The position of `piece` when the puzzle is solved.
    ///
    /// Returns `None` if `piece` is outside the valid range for the puzzle.
    #[must_use]
    fn solved_pos(&self, piece: Piece) -> Option<usize> {
        let n = self.num_pieces();
        match piece.to_usize() {
            Some(p) if p <= n => Some(self.solved_pos_unchecked(piece)),
            _ => None,
        }
    }

    /// The position of `piece` when the puzzle is solved as (x, y) coordinates.
    ///
    /// This function does not check whether `piece` is within the valid range for the puzzle.
    /// If `piece` is outside the valid range, the function may return a wrong result or panic.
    #[must_use]
    fn solved_pos_xy_unchecked(&self, piece: Piece) -> (usize, usize) {
        let p = self.solved_pos_unchecked(piece);
        let w = self.width();
        (p % w, p / w)
    }

    /// The position of `piece` when the puzzle is solved as (x, y) coordinates.
    ///
    /// Returns `None` if `piece` is outside the valid range for the puzzle.
    #[must_use]
    fn solved_pos_xy(&self, piece: Piece) -> Option<(usize, usize)> {
        let n = self.num_pieces();
        match piece.to_usize() {
            Some(p) if p <= n => Some(self.solved_pos_xy_unchecked(piece)),
            _ => None,
        }
    }

    /// The piece at a given position.
    ///
    /// This function does not check whether `idx` is within the valid range for the puzzle.
    /// If `idx` is outside the valid range, the function may return a wrong result or panic.
    #[must_use]
    fn piece_at_unchecked(&self, idx: usize) -> Piece;

    /// The piece at a given position.
    ///
    /// Returns `None` if `idx` is outside the valid range for the puzzle.
    #[must_use]
    fn piece_at(&self, idx: usize) -> Option<Piece> {
        if idx <= self.num_pieces() {
            Some(self.piece_at_unchecked(idx))
        } else {
            None
        }
    }

    /// The piece at a given (x, y) position.
    /// See also: [`SlidingPuzzle::piece_at_unchecked`].
    #[must_use]
    fn piece_at_xy_unchecked(&self, x: usize, y: usize) -> Piece {
        self.piece_at_unchecked(x + self.width() * y)
    }

    /// The piece at a given (x, y) position.
    /// See also: [`SlidingPuzzle::piece_at`].
    #[must_use]
    fn piece_at_xy(&self, x: usize, y: usize) -> Option<Piece> {
        if x < self.width() && y < self.height() {
            Some(self.piece_at_xy_unchecked(x, y))
        } else {
            None
        }
    }

    /// Set the piece at a given position to `piece`.
    ///
    /// This function may create invalid states if used incorrectly, e.g. creating multiple pieces
    /// with the same number.
    ///
    /// This function does not check whether `idx` is within the valid range for the puzzle.
    /// If `idx` is outside the valid range, the function may panic.
    fn set_piece_unchecked(&mut self, idx: usize, piece: Piece);

    /// Attempts to set the piece at a given position to `piece`.
    ///
    /// This function may create invalid states if used incorrectly, e.g. creating multiple pieces
    /// with the same number.
    ///
    /// Returns `true` if `idx` is within the valid range for the puzzle and the piece was
    /// successfully set, and `false` otherwise.
    fn set_piece(&mut self, idx: usize, piece: Piece) -> bool {
        if idx < self.area() {
            self.set_piece_unchecked(idx, piece);
            true
        } else {
            false
        }
    }

    /// Set the piece at a given (x, y) position to `piece`.
    /// See also: [`SlidingPuzzle::set_piece_unchecked`].
    fn set_piece_xy_unchecked(&mut self, x: usize, y: usize, piece: Piece) {
        self.set_piece_unchecked(x + self.width() * y, piece);
    }

    /// Set the piece at a given (x, y) position to `piece`.
    /// See also: [`SlidingPuzzle::set_piece`].
    fn set_piece_xy(&mut self, x: usize, y: usize, piece: Piece) -> bool {
        self.set_piece(x + self.width() * y, piece)
    }

    /// Swaps the pieces at positions `idx1` and `idx2`.
    ///
    /// This function does not check whether `idx1` and `idx2` are within the valid range for the
    /// puzzle. If `idx1` or `idx2` is outside the valid range, the function may panic.
    fn swap_pieces_unchecked(&mut self, idx1: usize, idx2: usize) {
        let piece = self.piece_at_unchecked(idx1);
        self.set_piece_unchecked(idx1, self.piece_at_unchecked(idx2));
        self.set_piece_unchecked(idx2, piece);
    }

    /// Attempts to swap the pieces at positions `idx1` and `idx2`.
    ///
    /// Returns `true` if `idx1` and `idx2` are within the valid range for the puzzle and the
    /// pieces were successfully swapped, and `false` otherwise.
    fn swap_pieces(&mut self, idx1: usize, idx2: usize) -> bool {
        let area = self.area();
        if idx1 < area && idx2 < area {
            self.swap_pieces_unchecked(idx1, idx2);
            true
        } else {
            false
        }
    }

    /// Swaps the pieces at positions `(x1, y1)` and `(x2, y2)`.
    /// See also: [`SlidingPuzzle::swap_pieces_unchecked`].
    fn swap_pieces_xy_unchecked(&mut self, (x1, y1): (usize, usize), (x2, y2): (usize, usize)) {
        let w = self.width();
        self.swap_pieces_unchecked(x1 + w * y1, x2 + w * y2);
    }

    /// Attempts to swap the pieces at positions `(x1, y1)` and `(x2, y2)`.
    /// See also: [`SlidingPuzzle::swap_pieces`].
    fn swap_pieces_xy(&mut self, (x1, y1): (usize, usize), (x2, y2): (usize, usize)) -> bool {
        let w = self.width();
        self.swap_pieces(x1 + w * y1, x2 + w * y2)
    }

    /// Checks if it is possible to move a piece in the given [`Direction`].
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

    /// Moves a piece in the given [`Direction`].
    ///
    /// This function does not check whether the piece can be moved. If it can not, the function
    /// may panic or the puzzle may be transformed in an invalid way.
    fn move_dir_unchecked(&mut self, dir: Direction) {
        let gap = self.gap_position();
        let piece = match dir {
            Direction::Up => gap + self.width(),
            Direction::Left => gap + 1,
            Direction::Down => gap - self.width(),
            Direction::Right => gap - 1,
        };
        self.swap_pieces_unchecked(gap, piece);
    }

    /// Attempts to move a piece in the given [`Direction`].
    ///
    /// Returns `true` if the piece was moved successfully, `false` otherwise.
    fn move_dir(&mut self, dir: Direction) -> bool {
        if self.can_move_dir(dir) {
            self.move_dir_unchecked(dir);
            true
        } else {
            false
        }
    }

    /// Checks if it is possible to apply the given [`Move`].
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

    /// Applies the given [`Move`] to the puzzle.
    ///
    /// This function does not check whether the move is valid. If it is not, the function may
    /// panic or the puzzle may be transformed in an invalid way.
    fn apply_move_unchecked(&mut self, mv: Move) {
        for _ in 0..mv.amount {
            self.move_dir_unchecked(mv.direction);
        }
    }

    /// Attempts to apply the given [`Move`] to the puzzle.
    ///
    /// Returns `true` if the move was applied successfully, `false` otherwise.
    fn apply_move(&mut self, mv: Move) -> bool {
        if self.can_apply_move(mv) {
            self.apply_move_unchecked(mv);
            true
        } else {
            false
        }
    }

    /// Checks if it is possible to apply the given [`Algorithm`].
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

    /// Applies the given [`Algorithm`] to the puzzle.
    ///
    /// This function does not check whether the algorithm is valid. If it is not, the function may
    /// panic or the puzzle may be transformed in an invalid way.
    fn apply_alg_unchecked(&mut self, alg: &Algorithm) {
        for &m in &alg.moves {
            self.apply_move_unchecked(m);
        }
    }

    /// Attempts to apply the given [`Algorithm`] to the puzzle.
    ///
    /// Returns `true` if the algorithm was applied successfully, `false` otherwise.
    fn apply_alg(&mut self, alg: &Algorithm) -> bool {
        if self.can_apply_alg(alg) {
            self.apply_alg_unchecked(alg);
            true
        } else {
            false
        }
    }
}
