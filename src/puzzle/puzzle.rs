use super::traits::SlidingPuzzle;
use crate::algorithm::direction::Direction;
use std::collections::HashSet;
use thiserror::Error;

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

const GAP_PIECE: u32 = 0;

impl Puzzle {
    pub fn new(width: usize, height: usize) -> Puzzle {
        Puzzle {
            pieces: {
                let mut v: Vec<u32> = (1..(width * height) as u32).collect();
                v.push(GAP_PIECE);
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

    fn gap_piece() -> u32 {
        GAP_PIECE
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
