//! Defines an implementation of the [`SlidingPuzzle`] trait.

use num_traits::AsPrimitive;
use std::{fmt::Display, str::FromStr};
use thiserror::Error;

use crate::puzzle::{
    display::{DisplayGrid, DisplayInline},
    label::label::BijectiveLabel,
    size::{Size, SizeError},
    sliding_puzzle::SlidingPuzzle,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A sliding puzzle, with an implementation of the [`SlidingPuzzle`] trait.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Puzzle {
    pieces: Vec<u64>,
    size: Size,
    gap: u64,
}

/// Error type for [`Puzzle`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PuzzleError {
    /// Returned when there was an error creating a [`Size`].
    #[error("InvalidSize: {0}")]
    InvalidSize(SizeError),

    /// Returned from [`Puzzle::new_from_grid`] when the given grid does not contain any pieces.
    #[error("Empty: grid is empty")]
    Empty,

    /// Returned when the puzzle has rows of different lengths.
    #[error("UnequalRowLengths: all row lengths must be equal")]
    UnequalRowLengths,

    /// Returned when the puzzle has a piece out of range (0 to `width * height - 1`).
    #[error("PieceOutOfRange: piece {0} is out of range")]
    PieceOutOfRange(u64),

    /// Returned when the puzzle has multiple pieces with the same number.
    #[error("DuplicatePiece: piece {0} appears more than once")]
    DuplicatePiece(u64),
}

impl Puzzle {
    /// Create a new [`Puzzle`] of a given size in the solved state.
    #[must_use]
    pub fn new(size: Size) -> Self {
        Self {
            pieces: {
                let mut v: Vec<u64> = (1..size.area()).collect();
                v.push(0);
                v
            },
            size,
            gap: size.num_pieces(),
        }
    }

    /// Create a new [`Puzzle`] from a list of numbers and a size.
    pub fn with_pieces(pieces: Vec<u64>, size: Size) -> Result<Self, PuzzleError> {
        let mut gap = None;
        let mut seen = vec![false; size.area() as usize];
        for (i, &n) in pieces.iter().enumerate() {
            if n >= size.area() {
                return Err(PuzzleError::PieceOutOfRange(n));
            }
            if seen[n as usize] {
                return Err(PuzzleError::DuplicatePiece(n));
            }

            seen[n as usize] = true;

            if n == 0 {
                gap = Some(i as u64);
            }
        }

        // At this point, `gap` is guaranteed to be Some because we found w * h non-negative
        // integers, all less than w * h, with no duplicates, so 0 must have occurred somewhere.
        // So it is safe to call unwrap.
        Ok(Self {
            pieces,
            size,
            gap: gap.unwrap(),
        })
    }

    /// Returns the internal vector of pieces as a slice.
    #[must_use]
    pub fn pieces(&self) -> &[u64] {
        &self.pieces
    }

    /// Create a new [`Puzzle`] from a 2D grid of numbers.
    pub fn new_from_grid(grid: Vec<Vec<u64>>) -> Result<Self, PuzzleError> {
        if grid.is_empty() {
            return Err(PuzzleError::Empty);
        }

        if grid[0].is_empty() {
            return Err(PuzzleError::Empty);
        }

        let w = grid[0].len() as u64;
        let h = grid.len() as u64;

        // Check if all rows are the same length
        if grid.iter().any(|r| r.len() as u64 != w) {
            return Err(PuzzleError::UnequalRowLengths);
        }

        let pieces = grid.into_iter().flatten().collect();
        let size = Size::new(w, h).map_err(PuzzleError::InvalidSize)?;

        Self::with_pieces(pieces, size)
    }

    /// Equivalent to [`DisplayInline::new`].
    #[must_use]
    pub fn display_inline(&self) -> DisplayInline<'_, Self> {
        DisplayInline::new(self)
    }

    /// Equivalent to [`DisplayGrid::new`].
    #[must_use]
    pub fn display_grid(&self) -> DisplayGrid<'_, Self> {
        DisplayGrid::new(self)
    }
}

impl SlidingPuzzle for Puzzle {
    type Piece = u64;

    fn size(&self) -> Size {
        self.size
    }

    fn gap_position(&self) -> u64 {
        self.gap
    }

    fn try_gap_position(&self) -> Option<u64> {
        Some(self.gap)
    }

    unsafe fn gap_position_unchecked(&self) -> u64 {
        self.gap
    }

    fn gap_position_xy(&self) -> (u64, u64) {
        let w = self.size.width();
        (self.gap % w, self.gap / w)
    }

    fn try_gap_position_xy(&self) -> Option<(u64, u64)> {
        Some(self.gap_position_xy())
    }

    unsafe fn gap_position_xy_unchecked(&self) -> (u64, u64) {
        self.gap_position_xy()
    }

    fn reset(&mut self) {
        self.pieces
            .iter_mut()
            .enumerate()
            .for_each(|(i, p)| *p = (i + 1) as u64);
        self.gap = self.size().num_pieces();
        self.pieces[self.gap as usize] = 0;
    }

    fn reset_to_label<L: BijectiveLabel>(&mut self, label: &L) {
        let (w, h) = self.size().into();
        let area = self.area();
        for y in 0..h {
            for x in 0..w {
                let label = label.position_label(self.size(), (x, y));
                let idx = x + w * y;
                if label + 1 == area {
                    self.pieces[idx as usize] = 0;
                    self.gap = idx;
                } else {
                    self.pieces[idx as usize] = label + 1;
                }
            }
        }
    }

    unsafe fn set_state_unchecked<P: SlidingPuzzle>(&mut self, other: &P)
    where
        P::Piece: AsPrimitive<Self::Piece>,
        Self::Piece: 'static,
    {
        for i in 0..other.area() {
            self.pieces[i as usize] = other.piece_at_unchecked(i).as_();
        }
    }

    fn solved_pos(&self, piece: u64) -> u64 {
        if piece == 0 {
            self.num_pieces()
        } else {
            piece - 1
        }
    }

    fn piece_at(&self, idx: u64) -> u64 {
        self.pieces[idx as usize]
    }

    unsafe fn piece_at_unchecked(&self, idx: u64) -> u64 {
        *self.pieces.get_unchecked(idx as usize)
    }

    fn swap_pieces(&mut self, idx1: u64, idx2: u64) {
        self.pieces.swap(idx1 as usize, idx2 as usize);
        if self.pieces[idx1 as usize] == 0 {
            self.gap = idx1;
        } else if self.pieces[idx2 as usize] == 0 {
            self.gap = idx2;
        }
    }

    unsafe fn swap_pieces_unchecked(&mut self, idx1: u64, idx2: u64) {
        self.pieces.swap(idx1 as usize, idx2 as usize);
        if self.piece_at_unchecked(idx1) == 0 {
            self.gap = idx1;
        } else if self.piece_at_unchecked(idx2) == 0 {
            self.gap = idx2;
        }
    }

    fn swap_non_gap_pieces(&mut self, idx1: u64, idx2: u64) {
        self.pieces.swap(idx1 as usize, idx2 as usize);
    }

    unsafe fn swap_non_gap_pieces_unchecked(&mut self, idx1: u64, idx2: u64) {
        self.swap_non_gap_pieces(idx1, idx2);
    }

    fn swap_piece_with_gap(&mut self, idx: u64) {
        self.pieces[self.gap as usize] = self.pieces[idx as usize];
        self.pieces[idx as usize] = 0;
        self.gap = idx;
    }

    unsafe fn swap_piece_with_gap_unchecked(&mut self, idx: u64) {
        *self.pieces.get_unchecked_mut(self.gap as usize) = self.piece_at_unchecked(idx);
        *self.pieces.get_unchecked_mut(idx as usize) = 0;
        self.gap = idx;
    }
}

impl Default for Puzzle {
    fn default() -> Self {
        Self::new(Size::default())
    }
}

impl Display for Puzzle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display_inline().fmt(f)
    }
}

/// Error type for [`Puzzle::from_str`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ParsePuzzleError {
    /// Returned when an unexpected character is found.
    #[error("InvalidCharacter: character {0} is invalid")]
    InvalidCharacter(char),

    /// Returned when the string is parsed successfully, but creating a [`Puzzle`] fails.
    #[error("PuzzleError: {0}")]
    PuzzleError(PuzzleError),
}

impl FromStr for Puzzle {
    type Err = ParsePuzzleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pieces = Vec::new();
        let mut row_length = None;
        let mut num_rows = 0;
        let mut current_row_length = 0;
        let mut current_number = None;

        for c in s.chars() {
            if c == '/' || c == '\n' {
                if let Some(n) = current_number {
                    pieces.push(n);
                    current_row_length += 1;
                    current_number = None;
                }

                if row_length.is_some_and(|l| l != current_row_length) {
                    return Err(ParsePuzzleError::PuzzleError(
                        PuzzleError::UnequalRowLengths,
                    ));
                }

                row_length = Some(current_row_length);
                current_row_length = 0;
                num_rows += 1;
            } else if c.is_whitespace() {
                if let Some(n) = current_number {
                    pieces.push(n);
                    current_row_length += 1;
                    current_number = None;
                }
            } else if let Some(n) = c.to_digit(10) {
                current_number = Some(current_number.unwrap_or(0) * 10 + n as u64);
            } else {
                return Err(ParsePuzzleError::InvalidCharacter(c));
            }
        }

        if let Some(n) = current_number {
            pieces.push(n);
            current_row_length += 1;
        }

        if row_length.is_some_and(|l| l != current_row_length) {
            return Err(ParsePuzzleError::PuzzleError(
                PuzzleError::UnequalRowLengths,
            ));
        }

        row_length = Some(current_row_length);
        num_rows += 1;

        let size = Size::new(row_length.unwrap_or_default(), num_rows)
            .map_err(|s| ParsePuzzleError::PuzzleError(PuzzleError::InvalidSize(s)))?;

        Self::with_pieces(pieces, size).map_err(ParsePuzzleError::PuzzleError)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Puzzle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = self.to_string();
        serializer.serialize_str(&str)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Puzzle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let puzzle_str = String::deserialize(deserializer)?;
        Self::from_str(&puzzle_str).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let size = Size::new(4, 4).unwrap();
        let p = Puzzle::new(size);
        assert_eq!(
            p,
            Puzzle {
                pieces: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0],
                size,
                gap: 15
            }
        );
    }

    #[test]
    fn test_with_pieces() {
        let size = Size::default();
        let pieces = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0];
        let p = Puzzle::with_pieces(pieces, size);
        assert!(p.is_ok());
    }

    #[test]
    fn test_new_from_grid() {
        let v = [11, 0, 1, 10, 4, 5, 15, 6, 2, 3, 13, 8, 7, 12, 9, 14]
            .chunks(4)
            .map(|c| c.to_vec())
            .collect();
        let p = Puzzle::new_from_grid(v);
        assert!(p.is_ok());
    }

    #[test]
    fn test_new_from_grid_2() {
        let p = Puzzle::new_from_grid(Vec::new());
        assert_eq!(p, Err(PuzzleError::Empty));
    }

    #[test]
    fn test_new_from_grid_3() {
        let p = Puzzle::new_from_grid(vec![vec![]]);
        assert_eq!(p, Err(PuzzleError::Empty));
    }

    #[test]
    fn test_new_from_grid_4() {
        let v = [11, 0, 1, 10, 4, 7, 15, 6, 2, 3, 13, 8, 7, 12, 9, 14]
            .chunks(4)
            .map(|c| c.to_vec())
            .collect();
        let p = Puzzle::new_from_grid(v);
        assert_eq!(p, Err(PuzzleError::DuplicatePiece(7)));
    }

    #[test]
    fn test_new_from_grid_5() {
        let v = [11, 16, 1, 10, 4, 5, 15, 6, 2, 3, 13, 8, 7, 12, 9, 14]
            .chunks(4)
            .map(|c| c.to_vec())
            .collect();
        let p = Puzzle::new_from_grid(v);
        assert_eq!(p, Err(PuzzleError::PieceOutOfRange(16)));
    }

    #[test]
    fn test_new_from_grid_6() {
        let v = vec![vec![1, 2], vec![3, 4, 0]];
        let p = Puzzle::new_from_grid(v);
        assert_eq!(p, Err(PuzzleError::UnequalRowLengths));
    }

    mod sliding_puzzle {
        use crate::{
            algorithm::{algorithm::Algorithm, direction::Direction, r#move::r#move::Move},
            puzzle::{
                label::label::FringeGrids,
                scrambler::{RandomInvertibleState, Scrambler as _},
            },
        };

        use super::*;

        #[test]
        fn test_piece_position() {
            let p =
                Puzzle::from_str("9 15 20 6/19 11 13 12/17 3 10 23/0 7 2 14/1 16 18 21/22 8 5 4")
                    .unwrap();
            assert_eq!(p.piece_position(9), 0);
            assert_eq!(p.piece_position_xy(9), (0, 0));
            assert_eq!(p.piece_position(3), 9);
            assert_eq!(p.piece_position_xy(3), (1, 2));
            assert_eq!(p.piece_position(4), 23);
            assert_eq!(p.piece_position_xy(4), (3, 5));
        }

        #[test]
        fn test_gap_position() {
            let mut p = Puzzle::new(Size::new(4, 6).unwrap());
            assert_eq!(p.gap_position(), 23);
            assert_eq!(p.gap_position_xy(), (3, 5));
            p.try_move_dir(Direction::Down);
            assert_eq!(p.gap_position(), 19);
            assert_eq!(p.gap_position_xy(), (3, 4));
            p.try_move_dir(Direction::Right);
            assert_eq!(p.gap_position(), 18);
            assert_eq!(p.gap_position_xy(), (2, 4));
        }

        #[test]
        fn test_reset() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            p.try_move_dir(Direction::Down);
            p.reset();
            assert_eq!(p, Puzzle::new(Size::new(4, 4).unwrap()));
        }

        #[test]
        fn test_reset_to_label() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            p.reset_to_label(&FringeGrids);
            assert_eq!(
                p.pieces,
                vec![1, 2, 3, 4, 5, 8, 9, 10, 6, 11, 13, 14, 7, 12, 15, 0,]
            );
        }

        #[test]
        fn test_set_state() {
            let mut p1 = Puzzle::from_str("6 15 12 4/14 0 9 11/7 10 8 5/3 1 2 13").unwrap();
            let p2 = Puzzle::default();
            let p3 = Puzzle::from_str("3 4 1/5 0 7/2 8 6").unwrap();
            assert!(p1.try_set_state(&p2));
            assert_eq!(p1.pieces, p2.pieces);
            assert!(!p1.try_set_state(&p3));
            assert_eq!(p1.pieces, p2.pieces);
        }

        #[test]
        fn test_solved_pos() {
            let p = Puzzle::new(Size::new(4, 6).unwrap());
            assert_eq!(p.try_solved_pos(0), Some(23));
            assert_eq!(p.try_solved_pos_xy(0), Some((3, 5)));
            assert_eq!(p.try_solved_pos(1), Some(0));
            assert_eq!(p.try_solved_pos_xy(1), Some((0, 0)));
            assert_eq!(p.try_solved_pos(23), Some(22));
            assert_eq!(p.try_solved_pos_xy(23), Some((2, 5)));
            assert_eq!(p.try_solved_pos(24), None);
            assert_eq!(p.try_solved_pos_xy(24), None);
        }

        #[test]
        fn test_piece_at() {
            let mut p = Puzzle::new(Size::new(4, 6).unwrap());
            p.try_apply_move(Move::new(Direction::Down, 2));
            p.try_apply_move(Move::new(Direction::Right, 3));
            assert_eq!(p.try_piece_at(0), Some(1));
            assert_eq!(p.try_piece_at_xy((0, 0)), Some(1));
            assert_eq!(p.try_piece_at(12), Some(0));
            assert_eq!(p.try_piece_at_xy((0, 3)), Some(0));
            assert_eq!(p.try_piece_at(13), Some(13));
            assert_eq!(p.try_piece_at_xy((1, 3)), Some(13));
            assert_eq!(p.try_piece_at(24), None);
            assert_eq!(p.try_piece_at_xy((4, 0)), None);
            assert_eq!(p.try_piece_at_xy((0, 6)), None);
        }

        #[test]
        fn test_swap_pieces() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            p.try_swap_pieces(0, 6);
            assert_eq!(p.try_piece_at(0), Some(7));
            assert_eq!(p.try_piece_at(6), Some(1));
            p.try_swap_pieces_xy((0, 0), (2, 1));
            assert_eq!(p.try_piece_at(0), Some(1));
            assert_eq!(p.try_piece_at(6), Some(7));
        }

        #[test]
        fn test_swap_pieces_2() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            p.try_swap_pieces(0, 15);
            assert_eq!(p.gap_position(), 0);
        }

        #[test]
        fn test_swap_non_gap_pieces() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            p.try_swap_non_gap_pieces(0, 6);
            assert_eq!(p.try_piece_at(0), Some(7));
            assert_eq!(p.try_piece_at(6), Some(1));
            p.try_swap_non_gap_pieces_xy((0, 0), (2, 1));
            assert_eq!(p.try_piece_at(0), Some(1));
            assert_eq!(p.try_piece_at(6), Some(7));
        }

        #[test]
        fn test_swap_non_gap_pieces_2() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            let p2 = p.clone();
            assert!(!p.try_swap_non_gap_pieces(0, 15));
            assert_eq!(p, p2);
        }

        #[test]
        fn test_can_move_dir() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            assert!(!p.can_move_dir(Direction::Up));
            assert!(!p.can_move_dir(Direction::Left));
            assert!(p.can_move_dir(Direction::Down));
            assert!(p.can_move_dir(Direction::Right));
            p.try_move_dir(Direction::Down);
            assert!(p.can_move_dir(Direction::Up));
            assert!(!p.can_move_dir(Direction::Left));
            assert!(p.can_move_dir(Direction::Down));
            assert!(p.can_move_dir(Direction::Right));
            p.try_move_dir(Direction::Right);
            assert!(p.can_move_dir(Direction::Up));
            assert!(p.can_move_dir(Direction::Left));
            assert!(p.can_move_dir(Direction::Down));
            assert!(p.can_move_dir(Direction::Right));
            p.try_apply_move(Move::new(Direction::Down, 2));
            assert!(p.can_move_dir(Direction::Up));
            assert!(p.can_move_dir(Direction::Left));
            assert!(!p.can_move_dir(Direction::Down));
            assert!(p.can_move_dir(Direction::Right));
            p.try_apply_move(Move::new(Direction::Right, 2));
            assert!(p.can_move_dir(Direction::Up));
            assert!(p.can_move_dir(Direction::Left));
            assert!(!p.can_move_dir(Direction::Down));
            assert!(!p.can_move_dir(Direction::Right));
        }

        #[test]
        fn test_move_dir() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            assert!(p.try_move_dir(Direction::Down));
            assert!(!p.try_move_dir(Direction::Left));
            assert!(p.try_move_dir(Direction::Right));
            assert!(p.try_move_dir(Direction::Up));
            assert!(!p.try_move_dir(Direction::Up));
            assert_eq!(
                p.pieces,
                vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 11, 13, 14, 0, 12]
            );
        }

        #[test]
        fn test_can_apply_move() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            p.try_move_dir(Direction::Down);
            p.try_move_dir(Direction::Right);
            assert!(p.can_apply_move(Move::new(Direction::Up, 1)));
            assert!(p.can_apply_move(Move::new(Direction::Left, 1)));
            assert!(p.can_apply_move(Move::new(Direction::Down, 2)));
            assert!(p.can_apply_move(Move::new(Direction::Right, 2)));
            assert!(!p.can_apply_move(Move::new(Direction::Up, 2)));
            assert!(!p.can_apply_move(Move::new(Direction::Left, 2)));
            assert!(!p.can_apply_move(Move::new(Direction::Down, 3)));
            assert!(!p.can_apply_move(Move::new(Direction::Right, 3)));
        }

        #[test]
        fn test_apply_move() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            assert!(p.try_apply_move(Move::new(Direction::Down, 2)));
            assert!(!p.try_apply_move(Move::new(Direction::Down, 2)));
            assert!(!p.try_apply_move(Move::new(Direction::Left, 1)));
            assert!(!p.try_apply_move(Move::new(Direction::Right, 4)));
            assert!(p.try_apply_move(Move::new(Direction::Right, 3)));
            assert!(!p.try_apply_move(Move::new(Direction::Up, 3)));
            assert!(p.try_apply_move(Move::new(Direction::Up, 1)));
            assert_eq!(
                p.pieces,
                vec![1, 2, 3, 4, 9, 5, 6, 7, 0, 10, 11, 8, 13, 14, 15, 12]
            );
        }

        #[test]
        fn test_can_move_position() {
            let p =
                Puzzle::from_str("9 15 20 6/19 11 13 12/17 3 10 23/7 0 2 14/1 16 18 21/22 8 5 4")
                    .unwrap();
            assert!(!p.can_move_position(0));
            assert!(p.can_move_position(1));
            assert!(!p.can_move_position(2));
            assert!(!p.can_move_position(11));
            assert!(p.can_move_position(12));
            assert!(p.can_move_position(13));
            assert!(p.can_move_position(15));
            assert!(!p.can_move_position(16));
            assert!(p.can_move_position(17));
            assert!(!p.can_move_position(23));

            assert!(!p.can_move_position(24));
            assert!(!p.can_move_position(25));
            assert!(!p.can_move_position(100));

            assert!(!p.can_move_position_xy((0, 0)));
            assert!(p.can_move_position_xy((1, 0)));
            assert!(!p.can_move_position_xy((2, 0)));
            assert!(!p.can_move_position_xy((3, 2)));
            assert!(p.can_move_position_xy((0, 3)));
            assert!(p.can_move_position_xy((1, 3)));
            assert!(p.can_move_position_xy((3, 3)));
            assert!(!p.can_move_position_xy((0, 4)));
            assert!(p.can_move_position_xy((1, 4)));
            assert!(!p.can_move_position_xy((3, 5)));
            assert!(!p.can_move_position_xy((0, 6)));

            assert!(!p.can_move_position_xy((0, 6)));
            assert!(!p.can_move_position_xy((1, 6)));
            assert!(!p.can_move_position_xy((4, 0)));
            assert!(!p.can_move_position_xy((4, 3)));
            assert!(!p.can_move_position_xy((10, 10)));
        }

        #[test]
        fn test_move_position() {
            let mut p =
                Puzzle::from_str("9 15 20 6/19 11 13 12/17 3 10 23/7 0 2 14/1 16 18 21/22 8 5 4")
                    .unwrap();

            p.try_move_position(1);
            p.try_move_position(6);
            p.try_move_position(0);
            p.try_move_position(16);
            p.try_move_position(15);
            p.try_move_position(100);
            p.try_move_position(19);

            assert_eq!(
                p.pieces,
                vec![
                    19, 9, 20, 6, 17, 15, 13, 12, 7, 11, 10, 23, 1, 3, 2, 14, 16, 18, 21, 0, 22, 8,
                    5, 4
                ]
            );
        }

        #[test]
        fn test_can_move_piece() {
            let p =
                Puzzle::from_str("9 15 20 6/19 11 13 12/17 3 10 23/7 0 2 14/1 16 18 21/22 8 5 4")
                    .unwrap();
            assert!(p.can_move_piece(15));
            assert!(p.can_move_piece(7));
            assert!(p.can_move_piece(14));
            assert!(p.can_move_piece(8));
            assert!(p.can_move_piece(0));
            assert!(!p.can_move_piece(9));
            assert!(!p.can_move_piece(6));
            assert!(!p.can_move_piece(13));
            assert!(!p.can_move_piece(17));
            assert!(!p.can_move_piece(22));

            assert!(!p.can_move_piece(24));
            assert!(!p.can_move_piece(100));
        }

        #[test]
        fn test_move_piece() {
            let mut p =
                Puzzle::from_str("9 15 20 6/19 11 13 12/17 3 10 23/7 0 2 14/1 16 18 21/22 8 5 4")
                    .unwrap();

            p.try_move_piece(15);
            p.try_move_piece(13);
            p.try_move_piece(9);
            p.try_move_piece(1);
            p.try_move_piece(14);
            p.try_move_piece(0);
            p.try_move_piece(100);
            p.try_move_piece(21);

            assert_eq!(
                p.pieces,
                vec![
                    19, 9, 20, 6, 17, 15, 13, 12, 7, 11, 10, 23, 1, 3, 2, 14, 16, 18, 21, 0, 22, 8,
                    5, 4
                ]
            );
        }

        #[test]
        fn test_can_apply_alg() {
            let p = Puzzle::new(Size::new(4, 4).unwrap());
            let a = Algorithm::from_str("D3RU2RD2RU3L3").unwrap();
            assert!(p.can_apply_alg(&a));
            let a = Algorithm::from_str("R2DL2UR2D2RU2LD3RULURDLDLU2RDLULD2RULDR2U").unwrap();
            assert!(p.can_apply_alg(&a));
        }

        #[test]
        fn test_can_apply_alg_2() {
            let p = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 12/0 13 14 15").unwrap();
            let a = Algorithm::from_str("L4").unwrap();
            assert!(!p.can_apply_alg(&a));
        }

        #[test]
        fn test_apply_alg() {
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());
            let a = Algorithm::from_str("D3RU2RD2RU3L3").unwrap();
            p.try_apply_alg(&a);
            assert_eq!(
                p.pieces,
                vec![5, 1, 7, 3, 9, 2, 11, 4, 13, 6, 10, 8, 14, 15, 12, 0]
            );
        }

        #[test]
        fn test_embed_into() {
            let p = Puzzle::from_str("5 13 3 2/6 1 14 0/15 7 9 8/10 12 11 4").unwrap();
            let mut p2 = Puzzle::from_str("42 25 52 8 26 63 16 39/27 62 4 45 40 59 58 55/56 17 57 35 6 15 20 38/51 24 53 31 13 33 3 23/54 14 48 29 5 2 37 46/10 12 28 30 18 0 36 19/32 41 44 50 22 7 60 49/21 34 61 11 43 1 47 9").unwrap();
            let expected = Puzzle::from_str("42 25 52 8 26 63 16 39/27 62 4 45 40 59 58 55/56 17 53 28 6 35 20 38/51 24 31 57 30 0 3 23/54 14 18 13 48 33 37 46/10 12 29 2 5 15 36 19/32 41 44 50 22 7 60 49/21 34 61 11 43 1 47 9").unwrap();
            assert!(p.try_embed_into(&mut p2));
            assert_eq!(p2, expected);
        }

        #[test]
        fn test_embed_into_2() {
            let p = Puzzle::from_str("1 5 9 13/2 6 10 14/3 7 11 15/4 8 12 0").unwrap();
            let mut p2 = p.clone();
            let expected = Puzzle::default();
            assert!(p.try_embed_into(&mut p2));
            assert_eq!(p2, expected);
        }

        #[test]
        fn test_embed_into_3() {
            let p = Puzzle::from_str("0 3/2 1").unwrap();
            let mut p2 = Puzzle::default();
            let expected = Puzzle::from_str("0 5 3 4/2 1 10 8/9 7 6 15/13 14 12 11").unwrap();
            assert!(p.try_embed_into(&mut p2));
            assert!(p.try_embed_into(&mut p2));
            assert!(p.try_embed_into(&mut p2));
            assert!(!p.try_embed_into(&mut p2));
            assert_eq!(p2, expected);
        }

        #[test]
        fn test_invert() {
            let mut p = Puzzle::from_str("10 6 2 12/11 5 14 13/3 7 1 8/4 9 15 0").unwrap();
            let p2 = Puzzle::from_str("11 3 9 13/6 2 10 12/14 1 5 4/8 7 15 0").unwrap();
            assert!(p.try_invert());
            assert_eq!(p, p2);
        }

        #[test]
        fn test_invert_2() {
            let mut p = Puzzle::from_str("10 6 2 12/11 5 14 13/3 7 1 8/4 9 0 15").unwrap();
            assert!(!p.try_invert());
        }

        #[test]
        fn test_invert_3() {
            let mut p = Puzzle::new(Size::new(20, 20).unwrap());

            for _ in 0..100 {
                RandomInvertibleState.scramble(&mut p);

                let p2 = p.clone();

                assert!(p.try_invert());
                assert!(p.try_invert());
                assert_eq!(p, p2);
            }
        }

        #[test]
        fn test_invert_4() {
            let mut p = Puzzle::from_str("6 13 14 11/7 1 5 8/10 9 4 12/2 3 15 0").unwrap();
            let p2 = p.clone();
            assert!(p.try_invert());
            assert_eq!(p, p2);
        }
    }

    mod from_str {
        use super::*;

        #[test]
        fn test_from_str() {
            let a = Puzzle::from_str("1 2 3 4/5 6 7 0");
            assert_eq!(
                a,
                Ok(Puzzle {
                    pieces: vec![1, 2, 3, 4, 5, 6, 7, 0],
                    size: Size::new(4, 2).unwrap(),
                    gap: 7
                })
            );
        }

        #[test]
        fn test_from_str_2() {
            let a = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0");
            assert_eq!(
                a,
                Ok(Puzzle {
                    pieces: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0],
                    size: Size::new(4, 4).unwrap(),
                    gap: 15
                })
            );
        }

        #[test]
        fn test_from_str_3() {
            let a = Puzzle::from_str("   1  2 3 4  \n5 6\t7 8/9 10 11 12 / 13 14   15 0\t");
            assert_eq!(
                a,
                Ok(Puzzle {
                    pieces: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0],
                    size: Size::new(4, 4).unwrap(),
                    gap: 15
                })
            );
        }

        #[test]
        fn test_from_str_4() {
            let a = Puzzle::from_str("1 2 3 4/5t 6 7 8/9 10 11 12/13 14 15 0");
            assert_eq!(a, Err(ParsePuzzleError::InvalidCharacter('t')));
        }
    }
}

#[cfg(all(feature = "nightly", test))]
mod benchmarks {
    extern crate test;

    use rand::SeedableRng;
    use rand_xoshiro::Xoroshiro128StarStar;
    use test::{black_box, Bencher};

    use crate::{
        algorithm::algorithm::Algorithm,
        puzzle::scrambler::{RandomState, Scrambler},
    };

    use super::*;

    #[bench]
    fn bench_reset(b: &mut Bencher) {
        let mut p = Puzzle::default();
        b.iter(|| {
            for _ in 0..100 {
                p.reset();
            }
        });
        black_box(p);
    }

    #[bench]
    fn bench_is_solved(b: &mut Bencher) {
        let p = Puzzle::default();
        b.iter(|| {
            for _ in 0..100 {
                black_box(p.is_solved());
            }
        });
    }

    #[bench]
    fn bench_is_solved_100(b: &mut Bencher) {
        let p = Puzzle::new(Size::new(100, 100).unwrap());
        b.iter(|| black_box(p.is_solved()));
    }

    #[bench]
    fn bench_solved_pos(b: &mut Bencher) {
        let p = Puzzle::new(Size::new(4, 4).unwrap());
        b.iter(|| {
            for _ in 0..1000 {
                black_box(p.solved_pos(10));
            }
        });
    }

    #[bench]
    fn bench_try_solved_pos(b: &mut Bencher) {
        let p = Puzzle::new(Size::new(4, 4).unwrap());
        b.iter(|| {
            for _ in 0..1000 {
                black_box(p.try_solved_pos(10).unwrap());
            }
        });
    }

    #[bench]
    fn bench_solved_pos_xy(b: &mut Bencher) {
        let p = Puzzle::new(Size::new(4, 4).unwrap());
        b.iter(|| {
            for _ in 0..1000 {
                black_box(p.solved_pos_xy(10));
            }
        });
    }

    #[bench]
    fn bench_try_solved_pos_xy(b: &mut Bencher) {
        let p = Puzzle::new(Size::new(4, 4).unwrap());
        b.iter(|| {
            for _ in 0..1000 {
                black_box(p.try_solved_pos_xy(10).unwrap());
            }
        });
    }

    #[bench]
    fn bench_can_apply_alg(b: &mut Bencher) {
        let p = Puzzle::default();
        let a = Algorithm::from_str(
            "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2R3D3",
        )
        .unwrap();
        b.iter(|| black_box(p.can_apply_alg(&a)));
    }

    #[bench]
    fn bench_apply_alg(b: &mut Bencher) {
        let mut p = Puzzle::default();
        let a = Algorithm::from_str(
            "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2",
        )
        .unwrap();
        b.iter(|| black_box(p.apply_alg(&a)));
    }

    #[bench]
    fn bench_from_str(b: &mut Bencher) {
        let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
        let mut puzzle = Puzzle::new(Size::new(50, 50).unwrap());
        RandomState.scramble_with_rng(&mut puzzle, &mut rng);
        let s = puzzle.to_string();

        b.iter(|| {
            black_box(Puzzle::from_str(&s).unwrap());
        });
    }
}
