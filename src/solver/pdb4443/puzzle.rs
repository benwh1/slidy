use crate::{algorithm::direction::Direction, solver::pdb4443::pattern::Pattern};

pub(super) enum MoveResult {
    MovedPiece,
    MovedIgnoredPiece,
    CantMove,
}

pub(super) struct Puzzle {
    pieces: [u8; 16],
    inverse: [u8; 16],
}

impl From<[u8; 16]> for Puzzle {
    fn from(pieces: [u8; 16]) -> Self {
        let mut seen = [false; 16];
        for i in 0..16 {
            assert!(
                !seen[pieces[i] as usize],
                "duplicate piece {} in puzzle",
                pieces[i],
            );
            seen[pieces[i] as usize] = true;
        }

        let mut inverse = [0; 16];

        for i in 0..16 {
            inverse[pieces[i] as usize] = i as u8;
        }

        Self { pieces, inverse }
    }
}

impl Puzzle {
    pub(super) fn new() -> Self {
        Self {
            pieces: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0],
            inverse: [15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
        }
    }

    pub(super) fn do_move(&mut self, dir: Direction) -> MoveResult {
        let gap = self.inverse[0];

        let piece_pos = match dir {
            Direction::Up => {
                if gap >= 12 {
                    return MoveResult::CantMove;
                }
                gap + 4
            }
            Direction::Left => {
                if gap % 4 == 3 {
                    return MoveResult::CantMove;
                }
                gap + 1
            }
            Direction::Down => {
                if gap < 4 {
                    return MoveResult::CantMove;
                }
                gap - 4
            }
            Direction::Right => {
                if gap % 4 == 0 {
                    return MoveResult::CantMove;
                }
                gap - 1
            }
        };

        let piece = self.pieces[piece_pos as usize];
        self.pieces[piece_pos as usize] = 0;
        self.pieces[gap as usize] = piece;
        self.inverse[0] = piece_pos;

        if piece == u8::MAX {
            MoveResult::MovedIgnoredPiece
        } else {
            self.inverse[piece as usize] = gap;
            MoveResult::MovedPiece
        }
    }

    pub(super) fn encode(&self, pattern: &Pattern) -> usize {
        let n = pattern.pieces.len();

        let mut pos = [0; 5];
        for i in 0..n {
            pos[i] = self.inverse[pattern.pieces[i] as usize];
        }

        let mut total = 0;

        for i in 0..n - 1 {
            total += pos[i] as usize;
            total *= 15 - i;

            for j in i + 1..n {
                if pos[i] < pos[j] {
                    pos[j] -= 1;
                }
            }
        }

        total += pos[n - 1] as usize;

        total
    }

    pub(super) fn decode(&mut self, mut idx: usize, pattern: &Pattern) {
        let n = pattern.pieces.len();

        let mut pos = [0; 16];

        for i in (0..n).rev() {
            pos[i] = idx % (16 - i);
            idx /= 16 - i;
        }

        for i in (0..n).rev() {
            for j in i + 1..n {
                if pos[i] <= pos[j] {
                    pos[j] += 1;
                }
            }
        }

        self.pieces = [u8::MAX; 16];

        for i in 0..n {
            let pattern_piece = pattern.pieces[i];
            self.inverse[pattern_piece as usize] = pos[i] as u8;
            self.pieces[pos[i]] = pattern_piece;
        }
    }

    fn swap_tiles(&mut self, a: u8, b: u8) {
        self.pieces.swap(
            self.inverse[a as usize] as usize,
            self.inverse[b as usize] as usize,
        );
        self.inverse.swap(a as usize, b as usize);
    }

    fn swap_positions(&mut self, a: usize, b: usize) {
        self.pieces.swap(a, b);
        self.inverse
            .swap(self.pieces[a] as usize, self.pieces[b] as usize);
    }

    pub(super) fn reflect_left_right(&mut self) {
        self.swap_tiles(1, 4);
        self.swap_tiles(2, 3);
        self.swap_tiles(5, 8);
        self.swap_tiles(6, 7);
        self.swap_tiles(9, 12);
        self.swap_tiles(10, 11);
        self.swap_tiles(13, 15);
        self.swap_positions(0, 3);
        self.swap_positions(1, 2);
        self.swap_positions(4, 7);
        self.swap_positions(5, 6);
        self.swap_positions(8, 11);
        self.swap_positions(9, 10);
        self.swap_positions(12, 15);
        self.swap_positions(13, 14);
    }

    pub(super) fn reflect_up_down(&mut self) {
        self.swap_tiles(1, 13);
        self.swap_tiles(5, 9);
        self.swap_tiles(2, 14);
        self.swap_tiles(6, 10);
        self.swap_tiles(3, 15);
        self.swap_tiles(7, 11);
        self.swap_tiles(4, 12);
        self.swap_positions(0, 12);
        self.swap_positions(4, 8);
        self.swap_positions(1, 13);
        self.swap_positions(5, 9);
        self.swap_positions(2, 14);
        self.swap_positions(6, 10);
        self.swap_positions(3, 15);
        self.swap_positions(7, 11);
    }
}
