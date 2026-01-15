use crate::{
    algorithm::direction::Direction,
    solver::size4x4::mtm::consts::{GAPS, MASKS, SHIFTS},
};

#[derive(Clone, Copy, Debug)]
pub(super) struct FourBitPuzzle {
    pub(super) pieces: u64,
    pub(super) gap: u8,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ReducedFourBitPuzzle {
    pub(super) pieces: u64,
    pub(super) gap: u8,
}

impl FourBitPuzzle {
    pub(super) const SOLVED: u64 = 0x0FEDCBA987654321;

    pub(super) fn new() -> Self {
        Self {
            pieces: Self::SOLVED,
            gap: 15,
        }
    }

    fn pieces(&self) -> [u8; 16] {
        let mut pieces = [0; 16];

        for (i, piece) in pieces.iter_mut().enumerate() {
            *piece = ((self.pieces >> (4 * i)) & 0xF) as u8;
        }

        pieces
    }

    pub(super) fn transposed(&self) -> Self {
        let pieces = self.pieces();
        let pos = |i| pieces.iter().position(|&x| x == i).unwrap();

        let mut transposed_pieces = pieces;

        transposed_pieces.swap(pos(2), pos(5));
        transposed_pieces.swap(pos(3), pos(9));
        transposed_pieces.swap(pos(4), pos(13));
        transposed_pieces.swap(pos(7), pos(10));
        transposed_pieces.swap(pos(8), pos(14));
        transposed_pieces.swap(pos(12), pos(15));
        transposed_pieces.swap(1, 4);
        transposed_pieces.swap(2, 8);
        transposed_pieces.swap(3, 12);
        transposed_pieces.swap(6, 9);
        transposed_pieces.swap(7, 13);
        transposed_pieces.swap(11, 14);

        Self::from(transposed_pieces)
    }

    pub(super) fn reduced(&self) -> ReducedFourBitPuzzle {
        let mut pieces = 0;

        for i in 0..16 {
            let piece = ((self.pieces >> (4 * i)) & 0xF) as usize;
            let reduced_piece = ReducedFourBitPuzzle::SOLVED >> (4 * ((piece + 15) % 16)) & 0xF;
            pieces |= reduced_piece << (4 * i);
        }

        ReducedFourBitPuzzle {
            pieces,
            gap: self.gap,
        }
    }

    #[inline(always)]
    pub(super) fn do_move(&mut self, dir: Direction) -> bool {
        let gap = self.gap as usize;
        let dir = dir as usize;

        let shift = SHIFTS[gap][dir] as u64;
        let piece = ((self.pieces >> shift) & 0xF) as usize;

        let mask = MASKS[gap][dir][piece];
        self.pieces ^= mask;

        let next_gap = GAPS[gap][dir];
        self.gap = next_gap;

        next_gap != gap as u8
    }
}

impl ReducedFourBitPuzzle {
    const SOLVED: u64 = 0x0443443322332211;

    pub(super) fn new() -> Self {
        Self {
            pieces: Self::SOLVED,
            gap: 15,
        }
    }

    #[inline(always)]
    pub(super) fn do_move(&mut self, dir: Direction) -> bool {
        let gap = self.gap as usize;
        let dir = dir as usize;

        let shift = SHIFTS[gap][dir] as u64;
        let piece = ((self.pieces >> shift) & 0xF) as usize;

        let mask = MASKS[gap][dir][piece];
        self.pieces ^= mask;

        let next_gap = GAPS[gap][dir];
        self.gap = next_gap;

        next_gap != gap as u8
    }
}

impl From<[u8; 16]> for FourBitPuzzle {
    fn from(value: [u8; 16]) -> Self {
        let mut pieces = 0;
        let mut gap = 0;
        for (i, &piece) in value.iter().enumerate() {
            pieces |= (piece as u64) << (4 * i);
            if piece == 0 {
                gap = i as u8;
            }
        }
        Self { pieces, gap }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_do_move_1() {
        let mut puzzle = FourBitPuzzle::new();
        assert!(puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces, 0xCFED0BA987654321);
        assert_eq!(puzzle.gap, 11);
        assert!(puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces, 0xCFED8BA907654321);
        assert_eq!(puzzle.gap, 7);
        assert!(puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces, 0xCFED8BA947650321);
        assert_eq!(puzzle.gap, 3);
        assert!(!puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces, 0xCFED8BA947650321);
        assert_eq!(puzzle.gap, 3);
    }

    #[test]
    fn test_do_move_2() {
        let mut puzzle = FourBitPuzzle::new();
        puzzle.do_move(Direction::Right);
        assert_eq!(puzzle.pieces, 0xF0EDCBA987654321);
    }

    #[test]
    fn test_reduced() {
        let puzzle = FourBitPuzzle::new();
        let reduced = puzzle.reduced();
        assert_eq!(reduced.pieces, ReducedFourBitPuzzle::SOLVED);
    }

    #[test]
    fn test_reduced_2() {
        let puzzle = FourBitPuzzle {
            pieces: 0xd46f9b8ac0e51732,
            gap: 6,
        };
        let reduced = puzzle.reduced();
        assert_eq!(reduced.pieces, 0x3234342340431221);
        assert_eq!(reduced.gap, 6);
    }
}
