use num_traits::Zero as _;

use crate::{
    algorithm::direction::Direction,
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle},
    solver::projection::encoding,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ProjectedPuzzle {
    pieces: [u8; encoding::MAX_PIECES],
    gap: u8,
    width: u8,
    height: u8,
    gx: u8,
    gy: u8,
}

impl ProjectedPuzzle {
    pub(super) fn new(pieces: &[u8], gap: u8, width: u8) -> Self {
        let len = pieces.len();
        assert!(len <= encoding::MAX_PIECES);
        let height = (len / width as usize) as u8;
        let mut buf = [0u8; encoding::MAX_PIECES];
        buf[..len].copy_from_slice(pieces);
        Self {
            pieces: buf,
            gap,
            width,
            height,
            gx: gap % width,
            gy: gap / width,
        }
    }

    pub(super) fn do_move(&mut self, dir: Direction) -> bool {
        let (new_gap, next_gx, next_gy) = match dir {
            Direction::Up => {
                if self.gy + 1 < self.height {
                    (self.gap + self.width, self.gx, self.gy + 1)
                } else {
                    return false;
                }
            }
            Direction::Left => {
                if self.gx + 1 < self.width {
                    (self.gap + 1, self.gx + 1, self.gy)
                } else {
                    return false;
                }
            }
            Direction::Down => {
                if self.gy > 0 {
                    (self.gap - self.width, self.gx, self.gy - 1)
                } else {
                    return false;
                }
            }
            Direction::Right => {
                if self.gx > 0 {
                    (self.gap - 1, self.gx - 1, self.gy)
                } else {
                    return false;
                }
            }
        };
        self.pieces.swap(self.gap as usize, new_gap as usize);
        self.gap = new_gap;
        self.gx = next_gx;
        self.gy = next_gy;
        true
    }

    pub(super) fn encode(&self, tally: &[u8]) -> u64 {
        encoding::encode(&self.pieces, tally)
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
    ProjectedPuzzle::new(&pieces, puzzle.gap_position() as u8, size.width() as u8)
}
