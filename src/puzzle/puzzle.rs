use super::traits::SlidingPuzzle;
use crate::algorithm::puzzle_move::Direction;

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

    fn piece_at_mut(&mut self, idx: usize) -> &mut u32 {
        &mut self.pieces[idx]
    }

    fn piece_at_xy_mut(&mut self, x: usize, y: usize) -> &mut u32 {
        self.piece_at_mut(x + self.width() * y)
    }

    fn can_move_dir(&self, dir: Direction) -> bool {
        match dir {
            Direction::Up => self.gap_position_y() + 1 < self.height(),
            Direction::Left => self.gap_position_x() + 1 < self.width(),
            Direction::Down => self.gap_position_y() > 0,
            Direction::Right => self.gap_position_x() > 0,
        }
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
