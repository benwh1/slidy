//! Defines a fixed-size puzzle with at most 15 pieces. The puzzle grid is represented using a
//! single `u64` with 4 bits per piece. Moves are applied efficiently using bit operations.

use crate::{
    algorithm::direction::Direction,
    puzzle::{puzzle::PuzzleError, size::Size, sliding_puzzle::SlidingPuzzle},
};

/// A `WxH` puzzle.
#[derive(Clone, Copy, Debug)]
pub struct Puzzle<const W: usize, const H: usize> {
    pieces: u64,
    gap: u8,
}

/// [`Puzzle`] specialized to the 2x2 size.
pub type Puzzle2x2 = Puzzle<2, 2>;
/// [`Puzzle`] specialized to the 2x3 size.
pub type Puzzle2x3 = Puzzle<2, 3>;
/// [`Puzzle`] specialized to the 2x4 size.
pub type Puzzle2x4 = Puzzle<2, 4>;
/// [`Puzzle`] specialized to the 2x5 size.
pub type Puzzle2x5 = Puzzle<2, 5>;
/// [`Puzzle`] specialized to the 2x6 size.
pub type Puzzle2x6 = Puzzle<2, 6>;
/// [`Puzzle`] specialized to the 2x7 size.
pub type Puzzle2x7 = Puzzle<2, 7>;
/// [`Puzzle`] specialized to the 2x8 size.
pub type Puzzle2x8 = Puzzle<2, 8>;
/// [`Puzzle`] specialized to the 3x2 size.
pub type Puzzle3x2 = Puzzle<3, 2>;
/// [`Puzzle`] specialized to the 3x3 size.
pub type Puzzle3x3 = Puzzle<3, 3>;
/// [`Puzzle`] specialized to the 3x4 size.
pub type Puzzle3x4 = Puzzle<3, 4>;
/// [`Puzzle`] specialized to the 3x5 size.
pub type Puzzle3x5 = Puzzle<3, 5>;
/// [`Puzzle`] specialized to the 4x2 size.
pub type Puzzle4x2 = Puzzle<4, 2>;
/// [`Puzzle`] specialized to the 4x3 size.
pub type Puzzle4x3 = Puzzle<4, 3>;
/// [`Puzzle`] specialized to the 4x4 size.
pub type Puzzle4x4 = Puzzle<4, 4>;
/// [`Puzzle`] specialized to the 5x2 size.
pub type Puzzle5x2 = Puzzle<5, 2>;
/// [`Puzzle`] specialized to the 5x3 size.
pub type Puzzle5x3 = Puzzle<5, 3>;
/// [`Puzzle`] specialized to the 6x2 size.
pub type Puzzle6x2 = Puzzle<6, 2>;
/// [`Puzzle`] specialized to the 7x2 size.
pub type Puzzle7x2 = Puzzle<7, 2>;
/// [`Puzzle`] specialized to the 8x2 size.
pub type Puzzle8x2 = Puzzle<8, 2>;

pub(crate) mod sealed {
    use crate::puzzle::sliding_puzzle::SlidingPuzzle;

    pub trait SmallPuzzle: SlidingPuzzle<Piece = u8> {
        const W: usize;
        const H: usize;
        const N: usize;

        type PieceArray;

        fn new() -> Self;
        unsafe fn with_pieces_and_gap_unchecked(pieces: u64, gap: u8) -> Self;
        unsafe fn from_piece_array_unchecked(piece_array: Self::PieceArray) -> Self;

        fn pieces(&self) -> u64;
        fn gap(&self) -> u8;

        fn piece_array(&self) -> Self::PieceArray;
    }
}

use sealed::SmallPuzzle;

macro_rules! impl_puzzle {
    ($w:literal, $h:literal) => {
        impl Puzzle<$w, $h> {
            pub(crate) const SOLVED: u64 = {
                let mut out = 0;

                let mut i = 0;
                while i < Self::N - 1 {
                    out |= (i as u64 + 1) << (4 * i);
                    i += 1;
                }

                out
            };

            pub(crate) const GAPS: [[u8; 4]; Self::N] = {
                let mut out = [[0; 4]; Self::N];

                let mut i = 0;
                while i < Self::N as u8 {
                    let (gx, gy) = (i % $w, i / $w);
                    out[i as usize] = [
                        if gy + 1 < $h { i + $w } else { i },
                        if gx + 1 < $w { i + 1 } else { i },
                        if gy > 0 { i - $w } else { i },
                        if gx > 0 { i - 1 } else { i },
                    ];
                    i += 1;
                }

                out
            };

            pub(crate) const SHIFTS: [[u8; 4]; Self::N] = {
                let mut out = [[0; 4]; Self::N];

                let mut gap = 0;
                while gap < Self::N {
                    let mut dir = 0;
                    while dir < 4 {
                        let other = Self::GAPS[gap][dir];
                        out[gap][dir] = if gap as u8 == other { 0 } else { other * 4 };
                        dir += 1;
                    }
                    gap += 1;
                }

                out
            };

            pub(crate) const SWAP_MASKS: [[[u64; Self::N]; Self::N]; Self::N] = {
                #[allow(clippy::large_stack_arrays)]
                let mut out = [[[0; Self::N]; Self::N]; Self::N];

                let mut gap = 0;
                while gap < Self::N {
                    let mut other = 0;
                    while other < Self::N {
                        if gap != other {
                            let mut piece = 0;
                            while piece < Self::N {
                                out[gap][other][piece] =
                                    ((piece << (gap * 4)) | (piece << (other * 4))) as u64;
                                piece += 1;
                            }
                        }
                        other += 1;
                    }
                    gap += 1;
                }

                out
            };

            pub(crate) const MOVE_MASKS: [[[u64; Self::N]; 4]; Self::N] = {
                let mut out = [[[0; Self::N]; 4]; Self::N];

                let mut gap = 0;
                while gap < Self::N {
                    let mut dir = 0;
                    while dir < 4 {
                        let mut piece = 0;
                        while piece < Self::N {
                            let other = Self::GAPS[gap][dir] as usize;
                            out[gap][dir][piece] = Self::SWAP_MASKS[gap][other][piece];
                            piece += 1;
                        }
                        dir += 1;
                    }
                    gap += 1;
                }

                out
            };
        }

        impl SmallPuzzle for Puzzle<$w, $h> {
            const W: usize = $w;
            const H: usize = $h;
            const N: usize = $w * $h;

            type PieceArray = [u8; Self::N];

            fn new() -> Self {
                Self {
                    pieces: Self::SOLVED,
                    gap: Self::N as u8 - 1,
                }
            }

            /// Creates a new [`Puzzle`] with the given `pieces` and `gap`, without checking that the
            /// puzzle state is valid or that `gap` matches the given state.
            ///
            /// # Safety
            ///
            /// The lower `W * H` nibbles of `pieces` must contain the values 0 to `W * H - 1`, exactly
            /// once each.
            /// `gap` must be less than `W * H`, and `(pieces >> (gap * 4)) & 0xF` must be 0.
            ///
            /// This function is unsafe because, although it doesn't cause immediate UB if used
            /// incorrectly, can break the type's invariant, which could lead to UB in otherwise-correct
            /// unsafe code elsewhere.
            unsafe fn with_pieces_and_gap_unchecked(pieces: u64, gap: u8) -> Self {
                debug_assert!(gap < Self::N as u8);
                debug_assert_eq!((pieces >> (gap * 4)) & 0xF, 0);

                Self { pieces, gap }
            }

            /// # Safety
            ///
            /// `piece_array` must contain the values 0 to `W * H - 1`, exactly once each.
            unsafe fn from_piece_array_unchecked(piece_array: Self::PieceArray) -> Self {
                let mut pieces = 0;
                let mut gap = 0;

                for (i, &piece) in piece_array.iter().enumerate() {
                    pieces |= (piece as u64) << (4 * i);
                    if piece == 0 {
                        gap = i as u8;
                    }
                }

                // SAFETY: `piece_array` contains the values 0 to `W * H - 1` once each, and
                // `pieces` contains the same values packed into a u64. `gap` is also set as
                // required.
                unsafe { Self::with_pieces_and_gap_unchecked(pieces, gap) }
            }

            fn pieces(&self) -> u64 {
                self.pieces
            }

            fn gap(&self) -> u8 {
                self.gap
            }

            fn piece_array(&self) -> Self::PieceArray {
                let mut pieces = [0; Self::N];

                for (i, piece) in pieces.iter_mut().enumerate() {
                    *piece = ((self.pieces >> (4 * i)) & 0xF) as u8;
                }

                pieces
            }
        }

        impl SlidingPuzzle for Puzzle<$w, $h> {
            type Piece = u8;

            fn size(&self) -> Size {
                Size::new($w, $h).unwrap()
            }

            fn piece_at(&self, idx: u64) -> Self::Piece {
                ((self.pieces >> (idx * 4)) & 0xF) as u8
            }

            fn swap_pieces(&mut self, idx1: u64, idx2: u64) {
                let piece1 = self.piece_at(idx1);
                let piece2 = self.piece_at(idx2);

                let p = (piece1 ^ piece2) as u64;

                let mask = (p << (idx1 * 4)) | (p << (idx2 * 4));
                self.pieces ^= mask;

                if piece1 == 0 {
                    self.gap = idx2 as u8;
                } else if piece2 == 0 {
                    self.gap = idx1 as u8;
                }
            }

            fn gap_position(&self) -> u64 {
                self.gap as u64
            }

            fn try_gap_position(&self) -> Option<u64> {
                Some(self.gap_position())
            }

            unsafe fn gap_position_unchecked(&self) -> u64 {
                self.gap_position()
            }

            fn gap_position_xy(&self) -> (u64, u64) {
                ((self.gap % $w) as u64, (self.gap / $w) as u64)
            }

            fn try_gap_position_xy(&self) -> Option<(u64, u64)> {
                Some(self.gap_position_xy())
            }

            unsafe fn gap_position_xy_unchecked(&self) -> (u64, u64) {
                self.gap_position_xy()
            }

            fn reset(&mut self) {
                *self = Self::new();
            }

            fn swap_piece_with_gap(&mut self, idx: u64) {
                let gap = self.gap as usize;
                let piece = self.piece_at(idx) as usize;

                self.pieces ^= Self::SWAP_MASKS[gap][idx as usize][piece];
                self.gap = idx as u8;
            }

            fn try_swap_piece_with_gap(&mut self, idx: u64) -> bool {
                if idx < self.area() {
                    self.swap_piece_with_gap(idx);
                    true
                } else {
                    false
                }
            }

            unsafe fn swap_piece_with_gap_unchecked(&mut self, idx: u64) {
                self.swap_piece_with_gap(idx);
            }

            fn can_move_dir(&self, dir: Direction) -> bool {
                Self::GAPS[self.gap as usize][dir as usize] != self.gap
            }

            fn move_dir(&mut self, dir: Direction) {
                self.try_move_dir(dir);
            }

            fn try_move_dir(&mut self, dir: Direction) -> bool {
                let gap = self.gap as usize;
                let dir = dir as usize;

                let shift = Self::SHIFTS[gap][dir] as u64;
                let piece = ((self.pieces >> shift) & 0xF) as usize;

                let mask = Self::MOVE_MASKS[gap][dir][piece];
                self.pieces ^= mask;

                let next_gap = Self::GAPS[gap][dir];
                self.gap = next_gap;

                next_gap != gap as u8
            }

            unsafe fn move_dir_unchecked(&mut self, dir: Direction) {
                self.move_dir(dir);
            }
        }

        impl TryFrom<[u8; $w * $h]> for Puzzle<$w, $h> {
            type Error = PuzzleError;

            fn try_from(value: [u8; $w * $h]) -> Result<Self, Self::Error> {
                let mut pieces = 0;
                let mut gap = 0;

                let mut seen = [false; $w * $h];
                for (i, &piece) in value.iter().enumerate() {
                    if piece >= $w * $h {
                        return Err(PuzzleError::PieceOutOfRange(piece as u64));
                    }

                    if seen[piece as usize] {
                        return Err(PuzzleError::DuplicatePiece(piece as u64));
                    }

                    seen[piece as usize] = true;

                    pieces |= (piece as u64) << (4 * i);
                    if piece == 0 {
                        gap = i as u8;
                    }
                }

                // SAFETY: We checked that all the pieces are distinct and within range, and set the
                // gap piece correctly.
                unsafe { Ok(Self::with_pieces_and_gap_unchecked(pieces, gap)) }
            }
        }
    };
}

impl_puzzle!(2, 2);
impl_puzzle!(2, 3);
impl_puzzle!(2, 4);
impl_puzzle!(2, 5);
impl_puzzle!(2, 6);
impl_puzzle!(2, 7);
impl_puzzle!(2, 8);
impl_puzzle!(3, 2);
impl_puzzle!(3, 3);
impl_puzzle!(3, 4);
impl_puzzle!(3, 5);
impl_puzzle!(4, 2);
impl_puzzle!(4, 3);
impl_puzzle!(4, 4);
impl_puzzle!(5, 2);
impl_puzzle!(5, 3);
impl_puzzle!(6, 2);
impl_puzzle!(7, 2);
impl_puzzle!(8, 2);
