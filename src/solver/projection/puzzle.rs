use num_traits::Zero;

use crate::{
    algorithm::direction::Direction,
    puzzle::{label::label::Label, size::Size, sliding_puzzle::SlidingPuzzle},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ProjectedPuzzle<const N: usize> {
    pub(super) pieces: [u8; N],
    pub(super) gap: u8,
}

impl<const N: usize> ProjectedPuzzle<N> {
    pub(super) fn new(label_pieces: [u8; N], gap: u8) -> Self {
        Self {
            pieces: label_pieces,
            gap,
        }
    }

    pub(super) fn is_solved(&self, solved_state: &[u8; N]) -> bool {
        self.pieces == *solved_state
    }

    pub(super) fn do_move<const W: usize, const H: usize>(&mut self, dir: Direction) -> bool {
        let gap = self.gap as usize;
        let new_gap = Self::new_gap_pos(gap, dir, W, H);
        if new_gap == gap {
            return false;
        }
        self.pieces.swap(gap, new_gap);
        self.gap = new_gap as u8;
        true
    }

    fn new_gap_pos(gap: usize, dir: Direction, w: usize, h: usize) -> usize {
        let gx = gap % w;
        let gy = gap / w;
        match dir {
            Direction::Up => {
                if gy + 1 < h {
                    gap + w
                } else {
                    gap
                }
            }
            Direction::Left => {
                if gx + 1 < w {
                    gap + 1
                } else {
                    gap
                }
            }
            Direction::Down => {
                if gy > 0 {
                    gap - w
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

pub(super) fn project_puzzle<const W: usize, const H: usize, const N: usize, P, L>(
    puzzle: &P,
    label: &L,
) -> ProjectedPuzzle<N>
where
    P: SlidingPuzzle,
    L: Label,
{
    let size = Size::new(W as u64, H as u64).unwrap();
    let mut pieces = [0u8; N];
    for (i, p) in pieces.iter_mut().enumerate() {
        let piece = puzzle.piece_at(i as u64);
        if piece.is_zero() {
            *p = 0;
        } else {
            let solved_pos = puzzle.solved_pos_xy(piece);
            *p = label.position_label(size, solved_pos) as u8 + 1;
        }
    }
    ProjectedPuzzle {
        pieces,
        gap: puzzle.gap_position() as u8,
    }
}
