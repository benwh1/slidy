//! Defines the [`SlidingPuzzle`] trait, which is the main trait defining the properties of a
//! sliding puzzle.

use num_traits::{AsPrimitive, NumCast, One as _, PrimInt, ToPrimitive as _, Zero as _};

use crate::{
    algorithm::{
        as_slice::AsAlgorithmSlice,
        direction::Direction,
        r#move::{position_move::PositionMove, r#move::Move, try_into_move::TryIntoMove as _},
    },
    puzzle::{
        label::label::{BijectiveLabel, RowGrids},
        size::Size,
        solvable::Solvable as _,
        solved_state::SolvedState as _,
    },
};

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
///   `w` is below position 0
/// - Position `N` is the bottom right corner
#[allow(clippy::missing_safety_doc)]
pub trait SlidingPuzzle
where
    Self: Sized,
{
    /// The type representing a piece of the puzzle (likely the elements in an array or vector).
    type Piece: PrimInt;

    /// Size of the puzzle.
    #[must_use]
    fn size(&self) -> Size;

    /// Product of the width and height.
    #[must_use]
    fn area(&self) -> u64 {
        self.size().area()
    }

    /// Number of pieces in the puzzle.
    #[must_use]
    fn num_pieces(&self) -> u64 {
        self.size().num_pieces()
    }

    /// Position of piece `piece`.
    ///
    /// # Panics
    ///
    /// Panics if there is no piece `piece` (i.e. if `piece` is out of the valid range for the
    /// puzzle).
    #[must_use]
    fn piece_position(&self, piece: Self::Piece) -> u64 {
        self.try_piece_position(piece).unwrap()
    }

    /// See [`SlidingPuzzle::piece_position`].
    #[must_use]
    fn try_piece_position(&self, piece: Self::Piece) -> Option<u64> {
        (0..self.area()).find(|&idx| self.piece_at(idx) == piece)
    }

    /// See [`SlidingPuzzle::piece_position`].
    #[must_use]
    unsafe fn piece_position_unchecked(&self, piece: Self::Piece) -> u64 {
        self.piece_position(piece)
    }

    /// Position of piece `piece` as (x, y) coordinates.
    #[must_use]
    fn piece_position_xy(&self, piece: Self::Piece) -> (u64, u64) {
        let pos = self.piece_position(piece);
        let w = self.size().width();
        (pos % w, pos / w)
    }

    /// See [`SlidingPuzzle::piece_position_xy`].
    #[must_use]
    fn try_piece_position_xy(&self, piece: Self::Piece) -> Option<(u64, u64)> {
        let w = self.size().width();
        self.try_piece_position(piece).map(|p| (p % w, p / w))
    }

    /// See [`SlidingPuzzle::piece_position_xy`].
    #[must_use]
    unsafe fn piece_position_xy_unchecked(&self, piece: Self::Piece) -> (u64, u64) {
        let pos = self.piece_position_unchecked(piece);
        let w = self.size().width();
        (pos % w, pos / w)
    }

    /// Position of the empty space.
    ///
    /// See [`SlidingPuzzle::piece_position`].
    #[must_use]
    fn gap_position(&self) -> u64 {
        self.piece_position(Self::Piece::zero())
    }

    /// See [`SlidingPuzzle::try_piece_position`].
    #[must_use]
    fn try_gap_position(&self) -> Option<u64> {
        self.try_piece_position(Self::Piece::zero())
    }

    /// See [`SlidingPuzzle::piece_position_unchecked`].
    #[must_use]
    unsafe fn gap_position_unchecked(&self) -> u64 {
        self.piece_position_unchecked(Self::Piece::zero())
    }

    /// Position of the empty space as (x, y) coordinates.
    ///
    /// See [`SlidingPuzzle::piece_position_xy`].
    #[must_use]
    fn gap_position_xy(&self) -> (u64, u64) {
        self.piece_position_xy(Self::Piece::zero())
    }

    /// See [`SlidingPuzzle::try_piece_position_xy`].
    #[must_use]
    fn try_gap_position_xy(&self) -> Option<(u64, u64)> {
        self.try_piece_position_xy(Self::Piece::zero())
    }

    /// See [`SlidingPuzzle::piece_position_xy_unchecked`].
    #[must_use]
    unsafe fn gap_position_xy_unchecked(&self) -> (u64, u64) {
        self.piece_position_xy_unchecked(Self::Piece::zero())
    }

    /// Reset the puzzle to the default state.
    fn reset(&mut self) {
        self.reset_to_label(&RowGrids);
    }

    /// Reset the puzzle to the solved state as defined by a [`BijectiveLabel`]
    fn reset_to_label<L: BijectiveLabel>(&mut self, label: &L) {
        let (w, h) = self.size().into();
        let area = <Self::Piece as NumCast>::from(self.size().area()).unwrap();
        for y in 0..h {
            for x in 0..w {
                let label = label.position_label(self.size(), (x, y));
                let piece = {
                    let a = <Self::Piece as NumCast>::from(label).unwrap() + Self::Piece::one();
                    if a == area {
                        Self::Piece::zero()
                    } else {
                        a
                    }
                };
                self.swap_pieces_xy((x, y), self.piece_position_xy(piece));
            }
        }
    }

    /// Sets the state to `other`.
    ///
    /// # Panics
    ///
    /// Panics if `self` and `other` are not the same size.
    fn set_state<P: SlidingPuzzle>(&mut self, other: &P)
    where
        P::Piece: AsPrimitive<Self::Piece>,
        Self::Piece: 'static,
    {
        if !self.try_set_state(other) {
            panic!(
                "sizes of `self` ({}) and `other` ({}) must be equal",
                self.size(),
                other.size()
            );
        }
    }

    /// See [`SlidingPuzzle::set_state`].
    ///
    /// Returns `true` if the state was set successfully, `false` otherwise.
    fn try_set_state<P: SlidingPuzzle>(&mut self, other: &P) -> bool
    where
        P::Piece: AsPrimitive<Self::Piece>,
        Self::Piece: 'static,
    {
        if self.size() == other.size() {
            for i in 0..other.area() {
                self.swap_pieces(i, self.piece_position(other.piece_at(i).as_()));
            }
            true
        } else {
            false
        }
    }

    /// See [`SlidingPuzzle::set_state`].
    unsafe fn set_state_unchecked<P: SlidingPuzzle>(&mut self, other: &P)
    where
        P::Piece: AsPrimitive<Self::Piece>,
        Self::Piece: 'static,
    {
        for i in 0..other.area() {
            self.swap_pieces_unchecked(
                i,
                self.piece_position_unchecked(other.piece_at_unchecked(i).as_()),
            );
        }
    }

    /// Check if the puzzle is solved.
    #[must_use]
    fn is_solved(&self) -> bool {
        RowGrids.is_solved(self)
    }

    /// Check if the puzzle is solvable.
    #[must_use]
    fn is_solvable(&self) -> bool {
        RowGrids::is_solvable(self)
    }

    /// The position of `piece` when the puzzle is solved.
    ///
    /// # Panics
    ///
    /// If `piece` is not within the range `0 <= piece < self.area()`, the function may panic or
    /// return a wrong result.
    #[must_use]
    fn solved_pos(&self, piece: Self::Piece) -> u64 {
        if piece == Self::Piece::zero() {
            self.num_pieces()
        } else {
            piece.to_u64().unwrap() - 1
        }
    }

    /// See [`SlidingPuzzle::solved_pos`].
    #[must_use]
    fn try_solved_pos(&self, piece: Self::Piece) -> Option<u64> {
        let n = self.num_pieces();
        match piece.to_u64() {
            Some(p) if p <= n => Some(self.solved_pos(piece)),
            _ => None,
        }
    }

    /// See [`SlidingPuzzle::solved_pos`].
    #[must_use]
    unsafe fn solved_pos_unchecked(&self, piece: Self::Piece) -> u64 {
        self.solved_pos(piece)
    }

    /// The position of `piece` when the puzzle is solved as (x, y) coordinates.
    ///
    /// # Panics
    ///
    /// See [`SlidingPuzzle::solved_pos`].
    #[must_use]
    fn solved_pos_xy(&self, piece: Self::Piece) -> (u64, u64) {
        let p = self.solved_pos(piece);
        let w = self.size().width();
        (p % w, p / w)
    }

    /// See [`SlidingPuzzle::solved_pos_xy`].
    #[must_use]
    fn try_solved_pos_xy(&self, piece: Self::Piece) -> Option<(u64, u64)> {
        let n = self.num_pieces();
        match piece.to_u64() {
            Some(p) if p <= n => Some(self.solved_pos_xy(piece)),
            _ => None,
        }
    }

    /// See [`SlidingPuzzle::solved_pos_xy`].
    #[must_use]
    unsafe fn solved_pos_xy_unchecked(&self, piece: Self::Piece) -> (u64, u64) {
        let p = self.solved_pos_unchecked(piece);
        let w = self.size().width();
        (p % w, p / w)
    }

    /// The piece at a given position.
    ///
    /// # Panics
    ///
    /// If `idx` is not within the range `0 <= idx < self.area()`, the function may panic or return
    /// a wrong result.
    #[must_use]
    fn piece_at(&self, idx: u64) -> Self::Piece;

    /// See [`SlidingPuzzle::piece_at`].
    #[must_use]
    fn try_piece_at(&self, idx: u64) -> Option<Self::Piece> {
        if idx < self.area() {
            Some(self.piece_at(idx))
        } else {
            None
        }
    }

    /// See [`SlidingPuzzle::piece_at`].
    #[must_use]
    unsafe fn piece_at_unchecked(&self, idx: u64) -> Self::Piece {
        self.piece_at(idx)
    }

    /// The piece at a given (x, y) position.
    ///
    /// # Panics
    ///
    /// See [`SlidingPuzzle::piece_at`].
    #[must_use]
    fn piece_at_xy(&self, (x, y): (u64, u64)) -> Self::Piece {
        self.piece_at(x + self.size().width() * y)
    }

    /// See [`SlidingPuzzle::piece_at_xy`].
    #[must_use]
    fn try_piece_at_xy(&self, pos: (u64, u64)) -> Option<Self::Piece> {
        if self.size().is_within_bounds(pos) {
            Some(self.piece_at_xy(pos))
        } else {
            None
        }
    }

    /// See [`SlidingPuzzle::piece_at_xy`].
    #[must_use]
    unsafe fn piece_at_xy_unchecked(&self, (x, y): (u64, u64)) -> Self::Piece {
        self.piece_at_unchecked(x + self.size().width() * y)
    }

    /// Swaps the pieces at positions `idx1` and `idx2`.
    ///
    /// # Panics
    ///
    /// `idx1` and `idx2` must both satisfy `0 <= idx < self.area()`, otherwise the function may
    /// panic.
    fn swap_pieces(&mut self, idx1: u64, idx2: u64);

    /// See [`SlidingPuzzle::swap_pieces`].
    ///
    /// Returns `true` if `idx1` and `idx2` are within the valid range for the puzzle and the
    /// pieces were successfully swapped, and `false` otherwise.
    fn try_swap_pieces(&mut self, idx1: u64, idx2: u64) -> bool {
        let area = self.area();
        if idx1 < area && idx2 < area {
            self.swap_pieces(idx1, idx2);
            true
        } else {
            false
        }
    }

    /// See [`SlidingPuzzle::swap_pieces`].
    unsafe fn swap_pieces_unchecked(&mut self, idx1: u64, idx2: u64) {
        self.swap_pieces(idx1, idx2);
    }

    /// Swaps the pieces at positions `(x1, y1)` and `(x2, y2)`.
    ///
    /// # Panics
    ///
    /// See [`SlidingPuzzle::swap_pieces`].
    fn swap_pieces_xy(&mut self, (x1, y1): (u64, u64), (x2, y2): (u64, u64)) {
        let w = self.size().width();
        self.swap_pieces(x1 + w * y1, x2 + w * y2);
    }

    /// See [`SlidingPuzzle::swap_pieces_xy`].
    fn try_swap_pieces_xy(&mut self, (x1, y1): (u64, u64), (x2, y2): (u64, u64)) -> bool {
        let w = self.size().width();
        self.try_swap_pieces(x1 + w * y1, x2 + w * y2)
    }

    /// See [`SlidingPuzzle::swap_pieces_xy`].
    unsafe fn swap_pieces_xy_unchecked(&mut self, (x1, y1): (u64, u64), (x2, y2): (u64, u64)) {
        let w = self.size().width();
        self.swap_pieces_unchecked(x1 + w * y1, x2 + w * y2);
    }

    /// See [`SlidingPuzzle::swap_pieces`].
    ///
    /// This function exists as an optimization of `swap_pieces` for the case where the pieces are
    /// known to not be the gap piece. In this case, depending on the implementation of the puzzle,
    /// we may be able to eliminate some checks (e.g. updating the gap position).
    ///
    /// # Panics
    ///
    /// If one of the pieces is the gap piece, the function may panic or the puzzle may be
    /// transformed in an invalid way.
    fn swap_non_gap_pieces(&mut self, idx1: u64, idx2: u64) {
        self.swap_pieces(idx1, idx2);
    }

    /// See [`SlidingPuzzle::swap_non_gap_pieces`].
    ///
    /// Returns `true` if `idx1` and `idx2` are within the valid range for the puzzle, neither one
    /// is the gap piece, and the pieces were successfully swapped, and `false` otherwise.
    fn try_swap_non_gap_pieces(&mut self, idx1: u64, idx2: u64) -> bool {
        let gap = self.gap_position();
        if idx1 != gap && idx2 != gap {
            self.try_swap_pieces(idx1, idx2)
        } else {
            false
        }
    }

    /// See [`SlidingPuzzle::swap_non_gap_pieces`].
    unsafe fn swap_non_gap_pieces_unchecked(&mut self, idx1: u64, idx2: u64) {
        self.swap_pieces_unchecked(idx1, idx2);
    }

    /// See [`SlidingPuzzle::swap_non_gap_pieces`] and [`SlidingPuzzle::swap_pieces_xy`].
    fn swap_non_gap_pieces_xy(&mut self, (x1, y1): (u64, u64), (x2, y2): (u64, u64)) {
        let w = self.size().width();
        self.swap_non_gap_pieces(x1 + w * y1, x2 + w * y2);
    }

    /// See [`SlidingPuzzle::swap_non_gap_pieces_xy`]
    fn try_swap_non_gap_pieces_xy(&mut self, pos1: (u64, u64), pos2: (u64, u64)) -> bool {
        let w = self.size().width();
        self.try_swap_non_gap_pieces(pos1.0 + w * pos1.1, pos2.0 + w * pos2.1)
    }

    /// See [`SlidingPuzzle::swap_non_gap_pieces_xy`]
    unsafe fn swap_non_gap_pieces_xy_unchecked(
        &mut self,
        (x1, y1): (u64, u64),
        (x2, y2): (u64, u64),
    ) {
        let w = self.size().width();
        self.swap_non_gap_pieces_unchecked(x1 + w * y1, x2 + w * y2);
    }

    /// Swaps piece in position `idx` with the gap.
    fn swap_piece_with_gap(&mut self, idx: u64) {
        self.swap_pieces(idx, self.gap_position());
    }

    /// See [`SlidingPuzzle::swap_piece_with_gap`].
    fn try_swap_piece_with_gap(&mut self, idx: u64) -> bool {
        if idx < self.area() {
            self.swap_piece_with_gap(idx);
            true
        } else {
            false
        }
    }

    /// See [`SlidingPuzzle::swap_piece_with_gap`].
    unsafe fn swap_piece_with_gap_unchecked(&mut self, idx: u64) {
        self.swap_pieces_unchecked(idx, self.gap_position());
    }

    /// Checks if it is possible to move a piece in the given [`Direction`].
    #[must_use]
    fn can_move_dir(&self, dir: Direction) -> bool {
        let (gx, gy) = self.gap_position_xy();
        match dir {
            Direction::Up => gy + 1 < self.size().height(),
            Direction::Left => gx + 1 < self.size().width(),
            Direction::Down => gy > 0,
            Direction::Right => gx > 0,
        }
    }

    /// Moves a piece in the given [`Direction`].
    ///
    /// # Panics
    ///
    /// If `self.can_move_dir(dir)` is false, the function may panic or the puzzle may be
    /// transformed in an invalid way.
    fn move_dir(&mut self, dir: Direction) {
        let gap = self.gap_position();
        let piece = match dir {
            Direction::Up => gap + self.size().width(),
            Direction::Left => gap + 1,
            Direction::Down => gap - self.size().width(),
            Direction::Right => gap - 1,
        };
        self.swap_piece_with_gap(piece);
    }

    /// See [`SlidingPuzzle::move_dir`].
    ///
    /// Returns `true` if the piece was moved successfully, `false` otherwise.
    fn try_move_dir(&mut self, dir: Direction) -> bool {
        if self.can_move_dir(dir) {
            self.move_dir(dir);
            true
        } else {
            false
        }
    }

    /// See [`SlidingPuzzle::move_dir`].
    unsafe fn move_dir_unchecked(&mut self, dir: Direction) {
        let gap = self.gap_position_unchecked();
        let piece = match dir {
            Direction::Up => gap + self.size().width(),
            Direction::Left => gap + 1,
            Direction::Down => gap - self.size().width(),
            Direction::Right => gap - 1,
        };
        self.swap_piece_with_gap_unchecked(piece);
    }

    /// Checks if it is possible to apply the given [`Move`].
    #[must_use]
    fn can_apply_move(&self, mv: Move) -> bool {
        let (gx, gy) = self.gap_position_xy();
        let amount = mv.amount;
        match mv.direction {
            Direction::Up => gy + amount < self.size().height(),
            Direction::Left => gx + amount < self.size().width(),
            Direction::Down => gy >= amount,
            Direction::Right => gx >= amount,
        }
    }

    /// Applies the given [`Move`] to the puzzle.
    ///
    /// # Panics
    ///
    /// If `self.can_apply_move(mv)` is false, the function may panic or the puzzle may be
    /// transformed in an invalid way.
    fn apply_move(&mut self, mv: Move) {
        for _ in 0..mv.amount {
            self.move_dir(mv.direction);
        }
    }

    /// See [`SlidingPuzzle::apply_move`].
    ///
    /// Returns `true` if the move was applied successfully, `false` otherwise.
    fn try_apply_move(&mut self, mv: Move) -> bool {
        if self.can_apply_move(mv) {
            self.apply_move(mv);
            true
        } else {
            false
        }
    }

    /// See [`SlidingPuzzle::apply_move`].
    unsafe fn apply_move_unchecked(&mut self, mv: Move) {
        for _ in 0..mv.amount {
            self.move_dir_unchecked(mv.direction);
        }
    }

    /// Checks if it is possible to move the piece at position `idx`.
    ///
    /// Returns `true` if position is in the same row or column as the gap. Also returns `true`
    /// if `idx` is the gap position.
    #[must_use]
    fn can_move_position(&self, idx: u64) -> bool {
        let w = self.size().width();
        self.can_move_position_xy((idx % w, idx / w))
    }

    /// Moves the piece in position `idx`.
    ///
    /// # Panics
    ///
    /// If `self.can_move_position(idx)` is false, the function may panic or the puzzle may be
    /// transformed in an invalid way.
    fn move_position(&mut self, idx: u64) {
        let w = self.size().width();
        self.move_position_xy((idx % w, idx / w));
    }

    /// See [`SlidingPuzzle::move_position`].
    ///
    /// Returns `true` if the piece was moved successfully, `false` otherwise.
    fn try_move_position(&mut self, idx: u64) -> bool {
        let w = self.size().width();
        self.try_move_position_xy((idx % w, idx / w))
    }

    /// See [`SlidingPuzzle::move_position`].
    unsafe fn move_position_unchecked(&mut self, idx: u64) {
        self.move_position(idx);
    }

    /// Checks if it is possible to move the piece at position `(x, y)`.
    ///
    /// Returns `true` if position `(x, y)` is in the same row or column as the gap. Also returns
    /// `true` if `(x, y)` is the gap position.
    #[must_use]
    fn can_move_position_xy(&self, (x, y): (u64, u64)) -> bool {
        if self.size().is_within_bounds((x, y)) {
            let (gx, gy) = self.gap_position_xy();
            x == gx || y == gy
        } else {
            false
        }
    }

    /// Moves the piece in position `(x, y)`.
    ///
    /// # Panics
    ///
    /// If `self.can_move_position_xy((x, y))` is false, the function may panic or the puzzle may be
    /// transformed in an invalid way.
    fn move_position_xy(&mut self, pos: (u64, u64)) {
        self.try_move_position_xy(pos);
    }

    /// See [`SlidingPuzzle::move_position_xy`].
    ///
    /// Returns `true` if the piece was moved successfully, `false` otherwise.
    fn try_move_position_xy(&mut self, (x, y): (u64, u64)) -> bool {
        match PositionMove(x, y).try_into_move(self) {
            Ok(mv) => {
                self.apply_move(mv);
                true
            }
            Err(_) => false,
        }
    }

    /// See [`SlidingPuzzle::move_position_xy`].
    unsafe fn move_position_xy_unchecked(&mut self, pos: (u64, u64)) {
        self.move_position_xy(pos);
    }

    /// Checks if it is possible to move piece `n`.
    ///
    /// Returns `true` if the piece is in the same row or column as the gap. Also returns `true` if
    /// `n` is 0, i.e. the gap piece.
    #[must_use]
    fn can_move_piece(&self, piece: Self::Piece) -> bool {
        self.try_piece_position_xy(piece)
            .is_some_and(|pos| self.can_move_position_xy(pos))
    }

    /// Moves piece `n`.
    ///
    /// # Panics
    ///
    /// If `self.can_move_piece(n)` is false, the function may panic or the puzzle may be
    /// transformed in an invalid way.
    fn move_piece(&mut self, piece: Self::Piece) {
        self.move_position_xy(self.piece_position_xy(piece));
    }

    /// See [`SlidingPuzzle::move_piece`].
    ///
    /// Returns `true` if the piece was moved successfully, `false` otherwise.
    fn try_move_piece(&mut self, piece: Self::Piece) -> bool {
        self.try_piece_position_xy(piece)
            .is_some_and(|pos| self.try_move_position_xy(pos))
    }

    /// See [`SlidingPuzzle::move_piece`].
    unsafe fn move_piece_unchecked(&mut self, piece: Self::Piece) {
        self.move_piece(piece);
    }

    /// Checks if it is possible to apply the given [`Algorithm`].
    ///
    /// [`Algorithm`]: ../../algorithm/algorithm.html
    #[must_use]
    fn can_apply_alg<'a, Alg: AsAlgorithmSlice<'a>>(&self, alg: &'a Alg) -> bool {
        let (mut gx, mut gy) = self.gap_position_xy();

        for m in alg.as_slice().moves() {
            let (new_gx, new_gy) = match m.direction {
                Direction::Up => (Some(gx), gy.checked_add(m.amount)),
                Direction::Left => (gx.checked_add(m.amount), Some(gy)),
                Direction::Down => (Some(gx), gy.checked_sub(m.amount)),
                Direction::Right => (gx.checked_sub(m.amount), Some(gy)),
            };

            match (new_gx, new_gy) {
                (Some(new_gx), Some(new_gy)) if self.size().is_within_bounds((new_gx, new_gy)) => {
                    (gx, gy) = (new_gx, new_gy);
                }
                _ => return false,
            }
        }

        true
    }

    /// Applies the given [`Algorithm`] to the puzzle.
    ///
    /// # Panics
    ///
    /// If `self.can_apply_alg(alg)` is false, the function may panic or the puzzle may be
    /// transformed in an invalid way.
    ///
    /// [`Algorithm`]: ../../algorithm/algorithm.html
    fn apply_alg<'a, Alg: AsAlgorithmSlice<'a>>(&mut self, alg: &'a Alg) {
        for m in alg.as_slice().moves() {
            self.apply_move(m);
        }
    }

    /// See [`SlidingPuzzle::apply_alg`].
    ///
    /// Returns `true` if the algorithm was applied successfully, `false` otherwise.
    fn try_apply_alg<'a, Alg: AsAlgorithmSlice<'a>>(&mut self, alg: &'a Alg) -> bool {
        if self.can_apply_alg(alg) {
            self.apply_alg(alg);
            true
        } else {
            false
        }
    }

    /// See [`SlidingPuzzle::apply_alg`].
    unsafe fn apply_alg_unchecked<'a, Alg: AsAlgorithmSlice<'a>>(&mut self, alg: &'a Alg) {
        for m in alg.as_slice().moves() {
            self.apply_move_unchecked(m);
        }
    }

    /// Checks if it is possible to embed `self` into `puzzle`.
    #[must_use]
    fn can_embed_into(&self, puzzle: &Self) -> bool {
        let (w, h) = self.size().into();
        let (gx, gy) = puzzle.gap_position_xy();
        gx >= w - 1 && gy >= h - 1
    }

    /// Embeds `self` into `puzzle`. This is equivalent to finding a solution of `self`, then
    /// applying the inverse to `puzzle`.
    ///
    /// # Panics
    ///
    /// If `self.can_embed_into(puzzle)` is false, the function may panic or `puzzle` may be
    /// transformed in an invalid way.
    fn embed_into(&self, puzzle: &mut Self) {
        if !self.try_embed_into(puzzle) {
            panic!("failed to embed `self` into `puzzle`");
        }
    }

    /// See [`SlidingPuzzle::embed_into`].
    ///
    /// Returns `true` if the puzzle was embedded successfully, `false` otherwise.
    fn try_embed_into(&self, puzzle: &mut Self) -> bool {
        let (w, h) = self.size().into();
        let (gx, gy) = puzzle.gap_position_xy();
        let (Some(left), Some(top)) = (gx.checked_sub(w - 1), gy.checked_sub(h - 1)) else {
            return false;
        };

        let mut seen = vec![false; self.area() as usize];

        for y in 0..h {
            for x in 0..w {
                let i = (x + y * w) as usize;
                if seen[i] {
                    continue;
                }

                // Find the cycle starting at `i`
                let mut cycle = Vec::new();
                let mut index = i;
                while !seen[index] {
                    seen[index] = true;
                    cycle.push(index);
                    index = self.solved_pos(self.piece_at(index as u64)) as usize;
                }

                // Apply the cycle to `puzzle`
                let mut index = cycle[0];
                for &next in cycle.iter().skip(1) {
                    let idx = index as u64;
                    let nxt = next as u64;

                    let (ix, iy) = (idx % w, idx / w);
                    let (nx, ny) = (nxt % w, nxt / w);

                    puzzle.swap_pieces_xy((ix + left, iy + top), (nx + left, ny + top));

                    index = next;
                }
            }
        }

        true
    }

    /// See [`SlidingPuzzle::embed_into`].
    unsafe fn embed_into_unchecked(&self, puzzle: &mut Self) {
        self.embed_into(puzzle);
    }
}
