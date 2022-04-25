use super::traits::SlidingPuzzle;
use crate::algorithm::direction::Direction;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashSet, num::ParseIntError, str::FromStr};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Puzzle {
    pieces: Vec<u32>,
    width: usize,
    height: usize,
    gap: usize,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PuzzleError {
    #[error("Empty: grid is empty")]
    Empty,

    #[error("MismatchedRowLengths: all row lengths must be equal")]
    MismatchedRowLengths,

    #[error("PieceOutOfRange: piece {0} is out of range")]
    PieceOutOfRange(u32),

    #[error("DuplicatePiece: piece {0} appears more than once")]
    DuplicatePiece(u32),
}

impl Puzzle {
    pub fn new(width: usize, height: usize) -> Puzzle {
        Puzzle {
            pieces: {
                let mut v: Vec<u32> = (1..(width * height) as u32).collect();
                v.push(0);
                v
            },
            width,
            height,
            gap: width * height - 1,
        }
    }

    pub fn new_from_grid(grid: Vec<Vec<u32>>) -> Result<Puzzle, PuzzleError> {
        if grid.is_empty() {
            return Err(PuzzleError::Empty);
        }

        let w = grid[0].len();
        let h = grid.len();

        // Check if all rows are the same length
        if grid.iter().any(|r| r.len() != w) {
            return Err(PuzzleError::MismatchedRowLengths);
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
        Ok(Puzzle {
            pieces: grid.into_iter().flatten().collect(),
            width: w,
            height: h,
            gap: gap.unwrap(),
        })
    }
}

impl SlidingPuzzle<u32> for Puzzle {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn gap_position(&self) -> usize {
        self.gap
    }

    fn gap_position_xy(&self) -> (usize, usize) {
        let g = self.gap_position();
        let w = self.width();
        (g % w, g / w)
    }

    fn piece_at(&self, idx: usize) -> u32 {
        self.pieces[idx]
    }

    fn piece_at_xy(&self, x: usize, y: usize) -> u32 {
        self.piece_at(x + self.width() * y)
    }

    fn swap_pieces(&mut self, idx1: usize, idx2: usize) {
        self.pieces.swap(idx1, idx2)
    }

    fn swap_pieces_xy(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let w = self.width();
        self.swap_pieces(x1 + w * y1, x2 + w * y2)
    }

    fn move_dir(&mut self, dir: Direction) {
        if !self.can_move_dir(dir) {
            return;
        }

        let gap = self.gap_position();
        let piece = match dir {
            Direction::Up => gap + self.width(),
            Direction::Left => gap + 1,
            Direction::Down => gap - self.width(),
            Direction::Right => gap - 1,
        };

        self.pieces.swap(gap, piece);
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParsePuzzleError {
    #[error("InvalidCharacter: character {0} is invalid")]
    InvalidCharacter(char),

    #[error("ParseIntError: {0}")]
    ParseIntError(ParseIntError),

    #[error("PuzzleError: {0}")]
    PuzzleError(PuzzleError),
}

impl FromStr for Puzzle {
    type Err = ParsePuzzleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Verify that no invalid characters are used
        for c in s.chars() {
            if !(c.is_whitespace() || c.is_digit(10) || c == '/') {
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

        Puzzle::new_from_grid(grid).map_err(ParsePuzzleError::PuzzleError)
    }
}

#[cfg(test)]
mod tests {
    use super::Puzzle;

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
    }
}
