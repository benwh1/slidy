use crate::{algorithm::direction::Direction, solver::size4x4::stm::pattern::Pattern};

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
                if gap.is_multiple_of(4) {
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
        for (i, p) in pos.iter_mut().enumerate().take(n) {
            *p = self.inverse[pattern.pieces[i] as usize];
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

        for (i, &p) in pos.iter().enumerate().take(n) {
            let pattern_piece = pattern.pieces[i];
            self.inverse[pattern_piece as usize] = p as u8;
            self.pieces[p] = pattern_piece;
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_decode_encode_pat4() {
        let mut puzzle = Puzzle::new();
        let pattern = Pattern::new(&[1, 2, 5, 6, 0]);

        for i in 0..524160 {
            puzzle.decode(i, &pattern);
            let encoded = puzzle.encode(&pattern);
            assert_eq!(encoded, i);
        }
    }

    #[test]
    fn test_encode_decode_pat4() {
        let pattern = Pattern::new(&[1, 2, 5, 6, 0]);

        for a in 0..16 {
            for b in 0..16 {
                if a == b {
                    continue;
                }

                for c in 0..16 {
                    if a == c || b == c {
                        continue;
                    }

                    for d in 0..16 {
                        if a == d || b == d || c == d {
                            continue;
                        }

                        for e in 0..16 {
                            if a == e || b == e || c == e || d == e {
                                continue;
                            }

                            let mut puzzle = Puzzle::new();
                            puzzle.pieces = [u8::MAX; 16];

                            puzzle.pieces[a as usize] = 1;
                            puzzle.pieces[b as usize] = 2;
                            puzzle.pieces[c as usize] = 5;
                            puzzle.pieces[d as usize] = 6;
                            puzzle.pieces[e as usize] = 0;
                            puzzle.inverse[1] = a;
                            puzzle.inverse[2] = b;
                            puzzle.inverse[5] = c;
                            puzzle.inverse[6] = d;
                            puzzle.inverse[0] = e;

                            let encoded = puzzle.encode(&pattern);
                            let mut puzzle2 = Puzzle::new();
                            puzzle2.decode(encoded, &pattern);

                            assert_eq!(puzzle.pieces, puzzle2.pieces);
                            assert_eq!(puzzle.inverse[1], puzzle2.inverse[1]);
                            assert_eq!(puzzle.inverse[2], puzzle2.inverse[2]);
                            assert_eq!(puzzle.inverse[5], puzzle2.inverse[5]);
                            assert_eq!(puzzle.inverse[6], puzzle2.inverse[6]);
                            assert_eq!(puzzle.inverse[0], puzzle2.inverse[0]);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_decode_encode_pat3() {
        let mut puzzle = Puzzle::new();
        let pattern = Pattern::new(&[11, 12, 15, 0]);

        for i in 0..43680 {
            puzzle.decode(i, &pattern);
            let encoded = puzzle.encode(&pattern);
            assert_eq!(encoded, i);
        }
    }

    #[test]
    fn test_encode_decode_pat3() {
        let pattern = Pattern::new(&[11, 12, 15, 0]);

        for a in 0..16 {
            for b in 0..16 {
                if a == b {
                    continue;
                }

                for c in 0..16 {
                    if a == c || b == c {
                        continue;
                    }

                    for d in 0..16 {
                        if a == d || b == d || c == d {
                            continue;
                        }

                        let mut puzzle = Puzzle::new();
                        puzzle.pieces = [u8::MAX; 16];

                        puzzle.pieces[a as usize] = 11;
                        puzzle.pieces[b as usize] = 12;
                        puzzle.pieces[c as usize] = 15;
                        puzzle.pieces[d as usize] = 0;
                        puzzle.inverse[11] = a;
                        puzzle.inverse[12] = b;
                        puzzle.inverse[15] = c;
                        puzzle.inverse[0] = d;

                        let encoded = puzzle.encode(&pattern);
                        let mut puzzle2 = Puzzle::new();
                        puzzle2.decode(encoded, &pattern);

                        assert_eq!(puzzle.pieces, puzzle2.pieces);
                        assert_eq!(puzzle.inverse[11], puzzle2.inverse[11]);
                        assert_eq!(puzzle.inverse[12], puzzle2.inverse[12]);
                        assert_eq!(puzzle.inverse[15], puzzle2.inverse[15]);
                        assert_eq!(puzzle.inverse[0], puzzle2.inverse[0]);
                    }
                }
            }
        }
    }
}
