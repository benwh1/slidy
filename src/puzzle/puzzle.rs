//! Defines an implementation of the [`SlidingPuzzle`] trait.

use super::{
    display::{DisplayGrid, DisplayInline},
    sliding_puzzle::SlidingPuzzle,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashSet, fmt::Display, num::ParseIntError, str::FromStr};
use thiserror::Error;

/// A sliding puzzle, with an implementation of the [`SlidingPuzzle`] trait.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Puzzle {
    pieces: Vec<u32>,
    width: usize,
    height: usize,
    gap: usize,
}

/// Error type for [`Puzzle`].
#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PuzzleError {
    /// Returned when the width and height are not both greater than zero.
    #[error("InvalidSize: width ({width}) and height ({height}) must both be at least 1")]
    InvalidSize {
        /// Width of the puzzle.
        width: usize,
        /// Height of the puzzle.
        height: usize,
    },

    /// Returned from [`Puzzle::new_from_grid`] when the given grid is empty.
    #[error("Empty: grid is empty")]
    Empty,

    /// Returned when the puzzle has rows of different lengths.
    #[error("UnequalRowLengths: all row lengths must be equal")]
    UnequalRowLengths,

    /// Returned when the puzzle has a piece out of range (0 to `width * height - 1`).
    #[error("PieceOutOfRange: piece {0} is out of range")]
    PieceOutOfRange(u32),

    /// Returned when the puzzle has multiple pieces with the same number.
    #[error("DuplicatePiece: piece {0} appears more than once")]
    DuplicatePiece(u32),
}

impl Puzzle {
    /// Create a new [`Puzzle`] of a given size in the solved state.
    pub fn new(width: usize, height: usize) -> Result<Self, PuzzleError> {
        if width < 1 || height < 1 {
            Err(PuzzleError::InvalidSize { width, height })
        } else {
            Ok(Self {
                pieces: {
                    let mut v: Vec<u32> = (1..(width * height) as u32).collect();
                    v.push(0);
                    v
                },
                width,
                height,
                gap: width * height - 1,
            })
        }
    }

    /// Create a new [`Puzzle`] from a 2D grid of numbers.
    pub fn new_from_grid(grid: Vec<Vec<u32>>) -> Result<Self, PuzzleError> {
        if grid.is_empty() {
            return Err(PuzzleError::Empty);
        }

        if grid[0].is_empty() {
            return Err(PuzzleError::InvalidSize {
                width: 0,
                height: 0,
            });
        }

        let w = grid[0].len();
        let h = grid.len();

        // Check if all rows are the same length
        if grid.iter().any(|r| r.len() != w) {
            return Err(PuzzleError::UnequalRowLengths);
        }

        let mut gap = None;
        let mut pieces = HashSet::new();
        for (y, row) in grid.iter().enumerate() {
            for (x, &n) in row.iter().enumerate() {
                if n as usize >= w * h {
                    return Err(PuzzleError::PieceOutOfRange(n));
                }
                if pieces.contains(&n) {
                    return Err(PuzzleError::DuplicatePiece(n));
                }

                pieces.insert(n);

                if n == 0 {
                    gap = Some(x + w * y);
                }
            }
        }

        // At this point, `gap` is guaranteed to be Some because we found w * h non-negative
        // integers, all less than w * h, with no duplicates, so 0 must have occurred somewhere.
        // So it is safe to call unwrap.
        Ok(Self {
            pieces: grid.into_iter().flatten().collect(),
            width: w,
            height: h,
            gap: gap.unwrap(),
        })
    }

    /// Equivalent to [`DisplayInline::new`].
    #[must_use]
    pub fn display_inline(&self) -> DisplayInline<Self> {
        DisplayInline::new(self)
    }

    /// Equivalent to [`DisplayGrid::new`].
    #[must_use]
    pub fn display_grid(&self) -> DisplayGrid<Self> {
        DisplayGrid::new(self)
    }
}

impl SlidingPuzzle for Puzzle {
    type Piece = u32;

    #[inline]
    fn width(&self) -> usize {
        self.width
    }

    #[inline]
    fn height(&self) -> usize {
        self.height
    }

    #[inline]
    fn gap_position(&self) -> usize {
        self.gap
    }

    #[inline]
    fn solved_pos(&self, piece: u32) -> usize {
        if piece == 0 {
            self.num_pieces()
        } else {
            piece as usize - 1
        }
    }

    #[inline]
    fn piece_at(&self, idx: usize) -> u32 {
        self.pieces[idx]
    }

    #[inline]
    unsafe fn piece_at_unchecked(&self, idx: usize) -> u32 {
        *self.pieces.get_unchecked(idx)
    }

    #[inline]
    fn set_piece(&mut self, idx: usize, piece: u32) {
        self.pieces[idx] = piece;
        if piece == 0 {
            self.gap = idx;
        }
    }

    #[inline]
    unsafe fn set_piece_unchecked(&mut self, idx: usize, piece: u32) {
        *self.pieces.get_unchecked_mut(idx) = piece;
        if piece == 0 {
            self.gap = idx;
        }
    }

    fn swap_pieces(&mut self, idx1: usize, idx2: usize) {
        self.pieces.swap(idx1, idx2);
        if self.pieces[idx1] == 0 {
            self.gap = idx1;
        } else if self.pieces[idx2] == 0 {
            self.gap = idx2;
        }
    }

    #[inline]
    unsafe fn swap_pieces_unchecked(&mut self, idx1: usize, idx2: usize) {
        self.pieces.swap_unchecked(idx1, idx2);
        if self.piece_at_unchecked(idx1) == 0 {
            self.gap = idx1;
        } else if self.piece_at_unchecked(idx2) == 0 {
            self.gap = idx2;
        }
    }

    fn swap_piece_with_gap(&mut self, idx: usize) {
        self.pieces[self.gap] = self.pieces[idx];
        self.pieces[idx] = 0;
        self.gap = idx;
    }

    unsafe fn swap_piece_with_gap_unchecked(&mut self, idx: usize) {
        *self.pieces.get_unchecked_mut(self.gap) = self.piece_at_unchecked(idx);
        *self.pieces.get_unchecked_mut(idx) = 0;
        self.gap = idx;
    }
}

impl Default for Puzzle {
    fn default() -> Self {
        Self::new(4, 4).unwrap()
    }
}

impl Display for Puzzle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display_inline().fmt(f)
    }
}

/// Error type for [`Puzzle::from_str`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParsePuzzleError {
    /// Returned when an unexpected character is found.
    #[error("InvalidCharacter: character {0} is invalid")]
    InvalidCharacter(char),

    /// Returned when an integer parse fails.
    #[error("ParseIntError: {0}")]
    ParseIntError(ParseIntError),

    /// Returned when the string is parsed successfully, but creating a [`Puzzle`] fails.
    #[error("PuzzleError: {0}")]
    PuzzleError(PuzzleError),
}

impl FromStr for Puzzle {
    type Err = ParsePuzzleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Verify that no invalid characters are used
        for c in s.chars() {
            if !(c.is_whitespace() || c.is_ascii_digit() || c == '/') {
                return Err(ParsePuzzleError::InvalidCharacter(c));
            }
        }

        // Match on numbers, slashes, new lines
        lazy_static! {
            static ref RE: Regex = Regex::new(r"\d+|\n|/").unwrap();
        }

        let mut grid: Vec<Vec<u32>> = Vec::new();
        let mut row = Vec::new();
        for m in RE.find_iter(s) {
            let m = m.as_str();
            match m {
                // End of a row
                "\n" | "/" => {
                    grid.push(row);
                    row = Vec::new();
                }
                // Must be a number
                _ => {
                    let n = m.parse::<u32>().map_err(ParsePuzzleError::ParseIntError)?;
                    row.push(n);
                }
            }
        }

        // Append the last row
        grid.push(row);

        Self::new_from_grid(grid).map_err(ParsePuzzleError::PuzzleError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let p = Puzzle::new(4, 4).unwrap();
        assert_eq!(
            p,
            Puzzle {
                pieces: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0],
                width: 4,
                height: 4,
                gap: 15
            }
        );
    }

    #[test]
    fn test_new_2() {
        let p = Puzzle::new(0, 0);
        assert!(p.is_err());

        let p = Puzzle::new(3, 0);
        assert!(p.is_err());

        let p = Puzzle::new(0, 5);
        assert!(p.is_err());
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
        assert_eq!(
            p,
            Err(PuzzleError::InvalidSize {
                width: 0,
                height: 0
            })
        );
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
            puzzle::label::label::FringeGrids,
        };

        use super::*;

        #[test]
        fn test_size() {
            let p = Puzzle::new(4, 6).unwrap();
            assert_eq!(p.width(), 4);
            assert_eq!(p.height(), 6);
            assert_eq!(p.size(), (4, 6));
        }

        #[test]
        fn test_area() {
            let p = Puzzle::new(4, 6).unwrap();
            assert_eq!(p.area(), 24);
        }

        #[test]
        fn test_num_pieces() {
            let p = Puzzle::new(4, 6).unwrap();
            assert_eq!(p.num_pieces(), 23);
        }

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
            let mut p = Puzzle::new(4, 6).unwrap();
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
            let mut p = Puzzle::new(4, 4).unwrap();
            p.try_move_dir(Direction::Down);
            p.reset();
            assert_eq!(p, Puzzle::new(4, 4).unwrap());
        }

        #[test]
        fn test_reset_to_label() {
            let mut p = Puzzle::new(4, 4).unwrap();
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
            let p = Puzzle::new(4, 6).unwrap();
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
            let mut p = Puzzle::new(4, 6).unwrap();
            p.try_apply_move(Move::new(Direction::Down, 2));
            p.try_apply_move(Move::new(Direction::Right, 3));
            assert_eq!(p.try_piece_at(0), Some(1));
            assert_eq!(p.try_piece_at_xy(0, 0), Some(1));
            assert_eq!(p.try_piece_at(12), Some(0));
            assert_eq!(p.try_piece_at_xy(0, 3), Some(0));
            assert_eq!(p.try_piece_at(13), Some(13));
            assert_eq!(p.try_piece_at_xy(1, 3), Some(13));
            assert_eq!(p.try_piece_at(24), None);
            assert_eq!(p.try_piece_at_xy(4, 0), None);
            assert_eq!(p.try_piece_at_xy(0, 6), None);
        }

        #[test]
        fn test_set_piece() {
            let mut p = Puzzle::new(4, 4).unwrap();
            p.try_set_piece(0, 6);
            p.try_set_piece_xy((1, 1), 1);
            assert_eq!(
                p.pieces,
                vec![6, 2, 3, 4, 5, 1, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0]
            );
        }

        #[test]
        fn test_set_piece_2() {
            let mut p = Puzzle::new(4, 4).unwrap();
            p.try_set_piece(0, 0);
            p.try_set_piece_xy((3, 3), 1);
            assert_eq!(
                p.pieces,
                vec![0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1,]
            );
            assert_eq!(p.gap, 0);
        }

        #[test]
        fn test_swap_pieces() {
            let mut p = Puzzle::new(4, 4).unwrap();
            p.try_swap_pieces(0, 6);
            assert_eq!(p.try_piece_at(0), Some(7));
            assert_eq!(p.try_piece_at(6), Some(1));
            p.try_swap_pieces_xy((0, 0), (2, 1));
            assert_eq!(p.try_piece_at(0), Some(1));
            assert_eq!(p.try_piece_at(6), Some(7));
        }

        #[test]
        fn test_swap_pieces_2() {
            let mut p = Puzzle::new(4, 4).unwrap();
            p.try_swap_pieces(0, 15);
            assert_eq!(p.gap_position(), 0);
        }

        #[test]
        fn test_can_move_dir() {
            let mut p = Puzzle::new(4, 4).unwrap();
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
            let mut p = Puzzle::new(4, 4).unwrap();
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
            let mut p = Puzzle::new(4, 4).unwrap();
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
            let mut p = Puzzle::new(4, 4).unwrap();
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
            let p = Puzzle::new(4, 4).unwrap();
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
            let mut p = Puzzle::new(4, 4).unwrap();
            let a = Algorithm::from_str("D3RU2RD2RU3L3").unwrap();
            p.try_apply_alg(&a);
            assert_eq!(
                p.pieces,
                vec![5, 1, 7, 3, 9, 2, 11, 4, 13, 6, 10, 8, 14, 15, 12, 0]
            );
        }
    }

    mod from_str {
        use super::*;
        use std::str::FromStr;

        #[test]
        fn test_from_str() {
            let a = Puzzle::from_str("1 2 3 4/5 6 7 0");
            assert_eq!(
                a,
                Ok(Puzzle {
                    pieces: vec![1, 2, 3, 4, 5, 6, 7, 0],
                    width: 4,
                    height: 2,
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
                    width: 4,
                    height: 4,
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
                    width: 4,
                    height: 4,
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

#[cfg(test)]
mod benchmarks {
    extern crate test;

    use std::str::FromStr;

    use test::{black_box, Bencher};

    use crate::algorithm::algorithm::Algorithm;

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
        let p = Puzzle::new(100, 100).unwrap();
        b.iter(|| black_box(p.is_solved()));
    }

    #[bench]
    fn bench_solved_pos(b: &mut Bencher) {
        let p = Puzzle::new(4, 4).unwrap();
        b.iter(|| {
            for _ in 0..1000 {
                black_box(p.solved_pos(10));
            }
        });
    }

    #[bench]
    fn bench_try_solved_pos(b: &mut Bencher) {
        let p = Puzzle::new(4, 4).unwrap();
        b.iter(|| {
            for _ in 0..1000 {
                black_box(p.try_solved_pos(10).unwrap());
            }
        });
    }

    #[bench]
    fn bench_solved_pos_xy(b: &mut Bencher) {
        let p = Puzzle::new(4, 4).unwrap();
        b.iter(|| {
            for _ in 0..1000 {
                black_box(p.solved_pos_xy(10));
            }
        });
    }

    #[bench]
    fn bench_try_solved_pos_xy(b: &mut Bencher) {
        let p = Puzzle::new(4, 4).unwrap();
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
}
