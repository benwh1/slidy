use crate::{
    algorithm::direction::Direction,
    puzzle::{
        sliding_puzzle::SlidingPuzzle as _,
        small::{sealed::SmallPuzzle as _, Puzzle4x4},
    },
};

#[derive(Clone, Copy, Debug)]
pub(super) struct FourBitPuzzle {
    pub(super) puzzle: Puzzle4x4,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ReducedFourBitPuzzle {
    pub(super) pieces: u64,
    pub(super) gap: u8,
}

impl FourBitPuzzle {
    pub(super) fn new() -> Self {
        Self {
            puzzle: Puzzle4x4::new(),
        }
    }

    #[allow(dead_code)]
    unsafe fn with_pieces_and_gap_unchecked(pieces: u64, gap: u8) -> Self {
        Self {
            puzzle: Puzzle4x4::with_pieces_and_gap_unchecked(pieces, gap),
        }
    }

    pub(super) fn pieces(&self) -> u64 {
        self.puzzle.pieces()
    }

    #[allow(dead_code)]
    fn gap(&self) -> u8 {
        self.puzzle.gap()
    }

    fn piece_array(&self) -> [u8; 16] {
        let raw_pieces = self.puzzle.pieces();
        let mut pieces = [0; 16];

        for (i, piece) in pieces.iter_mut().enumerate() {
            *piece = ((raw_pieces >> (4 * i)) & 0xF) as u8;
        }

        pieces
    }

    pub(super) fn transposed(&self) -> Self {
        let pieces = self.piece_array();
        let pos = |i| pieces.iter().position(|&x| x == i).unwrap() as u64;

        let mut transposed = self.puzzle;
        transposed.swap_pieces(pos(2), pos(5));
        transposed.swap_pieces(pos(3), pos(9));
        transposed.swap_pieces(pos(4), pos(13));
        transposed.swap_pieces(pos(7), pos(10));
        transposed.swap_pieces(pos(8), pos(14));
        transposed.swap_pieces(pos(12), pos(15));
        transposed.swap_pieces(1, 4);
        transposed.swap_pieces(2, 8);
        transposed.swap_pieces(3, 12);
        transposed.swap_pieces(6, 9);
        transposed.swap_pieces(7, 13);
        transposed.swap_pieces(11, 14);

        Self { puzzle: transposed }
    }

    pub(super) fn reduced(&self) -> ReducedFourBitPuzzle {
        let raw_pieces = self.puzzle.pieces();
        let mut pieces = 0;

        for i in 0..16 {
            let piece = ((raw_pieces >> (4 * i)) & 0xF) as usize;
            let reduced_piece = ReducedFourBitPuzzle::SOLVED >> (4 * ((piece + 15) % 16)) & 0xF;
            pieces |= reduced_piece << (4 * i);
        }

        ReducedFourBitPuzzle {
            pieces,
            gap: self.puzzle.gap(),
        }
    }

    #[inline(always)]
    pub(super) fn do_move(&mut self, dir: Direction) -> bool {
        self.puzzle.try_move_dir(dir)
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

    /// # Safety
    ///
    /// The nibbles of `pieces` must be a permutation of the nibbles of `Self::SOLVED`.
    /// `gap` must be less than 16, and `(pieces >> (gap * 4)) & 0xF` must be 0.
    unsafe fn with_pieces_and_gap_unchecked(pieces: u64, gap: u8) -> Self {
        Self { pieces, gap }
    }

    /// # Safety
    ///
    /// `piece_array` must contain a permutation of the nibbles in `Self::SOLVED`.
    pub(super) unsafe fn from_piece_array_unchecked(piece_array: [u8; 16]) -> Self {
        let mut pieces = 0;
        let mut gap = 0;

        for (i, &piece) in piece_array.iter().enumerate() {
            pieces |= (piece as u64) << (4 * i);
            if piece == 0 {
                gap = i as u8;
            }
        }

        // SAFETY: The nibbles of `pieces` have the same values as the elements of `piece_array`,
        // and `gap` is set correctly.
        unsafe { Self::with_pieces_and_gap_unchecked(pieces, gap) }
    }

    pub(super) fn pieces(&self) -> u64 {
        self.pieces
    }

    #[inline(always)]
    pub(super) fn do_move(&mut self, dir: Direction) -> bool {
        let gap = self.gap as usize;
        let dir = dir as usize;

        let shift = Puzzle4x4::SHIFTS[gap][dir] as u64;
        let piece = ((self.pieces >> shift) & 0xF) as usize;

        let mask = Puzzle4x4::MOVE_MASKS[gap][dir][piece];
        self.pieces ^= mask;

        let next_gap = Puzzle4x4::GAPS[gap][dir];
        self.gap = next_gap;

        next_gap != gap as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_move_1() {
        let mut puzzle = FourBitPuzzle::new();
        assert!(puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces(), 0xCFED0BA987654321);
        assert_eq!(puzzle.gap(), 11);
        assert!(puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces(), 0xCFED8BA907654321);
        assert_eq!(puzzle.gap(), 7);
        assert!(puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces(), 0xCFED8BA947650321);
        assert_eq!(puzzle.gap(), 3);
        assert!(!puzzle.do_move(Direction::Down));
        assert_eq!(puzzle.pieces(), 0xCFED8BA947650321);
        assert_eq!(puzzle.gap(), 3);
    }

    #[test]
    fn test_do_move_2() {
        let mut puzzle = FourBitPuzzle::new();
        puzzle.do_move(Direction::Right);
        assert_eq!(puzzle.pieces(), 0xF0EDCBA987654321);
    }

    #[test]
    fn test_reduced() {
        let puzzle = FourBitPuzzle::new();
        let reduced = puzzle.reduced();
        assert_eq!(reduced.pieces, ReducedFourBitPuzzle::SOLVED);
    }

    #[test]
    fn test_reduced_2() {
        let puzzle = unsafe { FourBitPuzzle::with_pieces_and_gap_unchecked(0xd46f9b8ac0e51732, 6) };
        let reduced = puzzle.reduced();
        assert_eq!(reduced.pieces, 0x3234342340431221);
        assert_eq!(reduced.gap, 6);
    }
}
