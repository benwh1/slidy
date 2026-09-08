use num_traits::Zero as _;

use crate::{
    algorithm::direction::Direction,
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle},
    solver::projection::encoding,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ProjectedPuzzle {
    pieces: Vec<u8>,
    gap: u8,
    width: u8,
}

impl ProjectedPuzzle {
    pub(super) fn new(pieces: Vec<u8>, gap: u8, width: u8) -> Self {
        assert!(pieces.len() <= encoding::MAX_PIECES);
        Self { pieces, gap, width }
    }

    pub(super) fn is_solved(&self, solved_state: &[u8]) -> bool {
        self.pieces == solved_state
    }

    pub(super) fn do_move(&mut self, dir: Direction) -> bool {
        let width = self.width as usize;
        let height = self.pieces.len() / width;
        let gap = self.gap as usize;
        let new_gap = Self::new_gap_pos(gap, width, height, dir);
        if new_gap == gap {
            return false;
        }
        self.pieces.swap(gap, new_gap);
        self.gap = new_gap as u8;
        true
    }

    pub(super) fn encode(&self, tally: &[u8]) -> u64 {
        let mut buf = [0u8; encoding::MAX_PIECES];
        buf[..self.pieces.len()].copy_from_slice(&self.pieces);
        encoding::encode(&buf, tally)
    }

    fn new_gap_pos(gap: usize, width: usize, height: usize, dir: Direction) -> usize {
        let gx = gap % width;
        let gy = gap / width;
        match dir {
            Direction::Up => {
                if gy + 1 < height {
                    gap + width
                } else {
                    gap
                }
            }
            Direction::Left => {
                if gx + 1 < width {
                    gap + 1
                } else {
                    gap
                }
            }
            Direction::Down => {
                if gy > 0 {
                    gap - width
                } else {
                    gap
                }
            }
            Direction::Right => {
                if gx > 0 {
                    gap - 1
                } else {
                    gap
                }
            }
        }
    }
}

pub(super) fn project_puzzle<P, L>(puzzle: &P, label: &L) -> ProjectedPuzzle
where
    P: SlidingPuzzle,
    L: Label,
{
    let size = puzzle.size();
    let mut pieces = Vec::with_capacity(size.area() as usize);
    for i in 0..size.area() {
        let piece = puzzle.piece_at(i);
        if piece.is_zero() {
            pieces.push(0);
        } else {
            let solved_pos = puzzle.solved_pos_xy(piece);
            pieces.push(label.position_label(size, solved_pos) as u8 + 1);
        }
    }
    ProjectedPuzzle::new(pieces, puzzle.gap_position() as u8, size.width() as u8)
}
