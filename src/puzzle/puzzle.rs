use super::traits::SlidingPuzzle;
use crate::algorithm::direction::Direction;

pub struct Puzzle {
    pieces: Vec<u32>,
    width: u32,
    height: u32,
    gap: usize,
}

const GAP_PIECE: u32 = 0;

impl Puzzle {
    pub fn new(width: u32, height: u32) -> Puzzle {
        Puzzle {
            pieces: {
                let mut v: Vec<u32> = (1..width * height).collect();
                v.push(GAP_PIECE);
                v
            },
            width,
            height,
            gap: (width * height - 1) as usize,
        }
    }
}

impl SlidingPuzzle<u32> for Puzzle {
    fn width(&self) -> usize {
        self.width as usize
    }

    fn height(&self) -> usize {
        self.height as usize
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
