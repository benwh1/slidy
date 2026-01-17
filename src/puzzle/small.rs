use crate::{
    algorithm::direction::Direction,
    puzzle::{puzzle::PuzzleError, size::Size, sliding_puzzle::SlidingPuzzle},
};

const GAPS: [[u8; 4]; 16] = [
    [4, 1, 0, 0],
    [5, 2, 1, 0],
    [6, 3, 2, 1],
    [7, 3, 3, 2],
    [8, 5, 0, 4],
    [9, 6, 1, 4],
    [10, 7, 2, 5],
    [11, 7, 3, 6],
    [12, 9, 4, 8],
    [13, 10, 5, 8],
    [14, 11, 6, 9],
    [15, 11, 7, 10],
    [12, 13, 8, 12],
    [13, 14, 9, 12],
    [14, 15, 10, 13],
    [15, 15, 11, 14],
];

const SHIFTS: [[u8; 4]; 16] = {
    let mut out = [[0; 4]; 16];

    let mut gap = 0;
    while gap < 16 {
        let mut dir = 0;
        while dir < 4 {
            let other = GAPS[gap][dir];
            out[gap][dir] = if gap as u8 == other { 0 } else { other * 4 };
            dir += 1;
        }
        gap += 1;
    }

    out
};

const MOVE_MASKS: [[[u64; 16]; 4]; 16] = {
    let mut out = [[[0; 16]; 4]; 16];

    let mut gap = 0;
    while gap < 16 {
        let mut dir = 0;
        while dir < 4 {
            let other = GAPS[gap][dir];
            if gap as u8 != other {
                let mut piece = 0;
                while piece < 16 {
                    out[gap][dir][piece] = ((piece << (gap * 4)) | (piece << (other * 4))) as u64;
                    piece += 1;
                }
            }
            dir += 1;
        }
        gap += 1;
    }

    out
};

const SWAP_MASKS: [[[u64; 16]; 16]; 16] = {
    let mut out = [[[0; 16]; 16]; 16];

    let mut gap = 0;
    while gap < 16 {
        let mut other = 0;
        while other < 16 {
            if gap != other {
                let mut piece = 0;
                while piece < 16 {
                    out[gap][other][piece] = ((piece << (gap * 4)) | (piece << (other * 4))) as u64;
                    piece += 1;
                }
            }
            other += 1;
        }
        gap += 1;
    }

    out
};

const SOLVED_STATES: [u64; 16] = [
    0x0,
    0x01,
    0x021,
    0x0321,
    0x04321,
    0x054321,
    0x0654321,
    0x07654321,
    0x087654321,
    0x0987654321,
    0x0A987654321,
    0x0BA987654321,
    0x0CBA987654321,
    0x0DCBA987654321,
    0x0EDCBA987654321,
    0x0FEDCBA987654321,
];

pub struct Dimension<const N: u8>;

mod sealed {
    pub trait AllowedSize {}
}

use sealed::AllowedSize;

impl AllowedSize for Dimension<2> {}
impl AllowedSize for Dimension<3> {}
impl AllowedSize for Dimension<4> {}

#[derive(Clone, Copy, Debug)]
pub struct Puzzle<const W: u8, const H: u8>
where
    Dimension<W>: AllowedSize,
    Dimension<H>: AllowedSize,
{
    pieces: u64,
    gap: u8,
}

pub type Puzzle2x2 = Puzzle<2, 2>;
pub type Puzzle2x3 = Puzzle<2, 3>;
pub type Puzzle2x4 = Puzzle<2, 4>;
pub type Puzzle3x2 = Puzzle<3, 2>;
pub type Puzzle3x3 = Puzzle<3, 3>;
pub type Puzzle3x4 = Puzzle<3, 4>;
pub type Puzzle4x2 = Puzzle<4, 2>;
pub type Puzzle4x3 = Puzzle<4, 3>;
pub type Puzzle4x4 = Puzzle<4, 4>;

impl<const W: u8, const H: u8> Puzzle<W, H>
where
    Dimension<W>: AllowedSize,
    Dimension<H>: AllowedSize,
{
    pub const SOLVED: u64 = SOLVED_STATES[const { (W * H - 1) as usize }];

    pub fn new() -> Self {
        Self {
            pieces: Self::SOLVED,
            gap: W * H - 1,
        }
    }

    /// Creates a new [`Puzzle`] with the given `pieces` and `gap`, without checking that the
    /// puzzle state is valid or that `gap` matches the given state.
    ///
    /// # Safety
    ///
    /// The lower `W * H` nibbles of `pieces` must contain the values 0 to `W * H - 1`, exactly
    /// once each, and `(pieces >> (gap * 4)) & 0xF` must be 0.
    ///
    /// This function is unsafe because, although it doesn't cause immediate UB if used
    /// incorrectly, can break the type's invariant, which could lead to UB in otherwise-correct
    /// unsafe code elsewhere.
    pub unsafe fn with_pieces_and_gap_unchecked(pieces: u64, gap: u8) -> Self {
        Self { pieces, gap }
    }

    pub fn pieces(&self) -> u64 {
        self.pieces
    }

    pub fn gap(&self) -> u8 {
        self.gap
    }
}

macro_rules! impl_from_piece_array {
    ($w:literal, $h:literal) => {
        impl Puzzle<$w, $h> {
            pub unsafe fn from_piece_array_unchecked(piece_array: [u8; $w * $h]) -> Self {
                let mut pieces = 0;
                let mut gap = 0;
                for (i, &piece) in piece_array.iter().enumerate() {
                    pieces |= (piece as u64) << (4 * i);
                    if piece == 0 {
                        gap = i as u8;
                    }
                }
                unsafe { Self::with_pieces_and_gap_unchecked(pieces, gap) }
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

                unsafe { Ok(Self::with_pieces_and_gap_unchecked(pieces, gap)) }
            }
        }
    };
}

impl_from_piece_array!(2, 2);
impl_from_piece_array!(2, 3);
impl_from_piece_array!(2, 4);
impl_from_piece_array!(3, 2);
impl_from_piece_array!(3, 3);
impl_from_piece_array!(3, 4);
impl_from_piece_array!(4, 2);
impl_from_piece_array!(4, 3);
impl_from_piece_array!(4, 4);

impl<const W: u8, const H: u8> SlidingPuzzle for Puzzle<W, H>
where
    Dimension<W>: AllowedSize,
    Dimension<H>: AllowedSize,
{
    type Piece = u8;

    fn size(&self) -> Size {
        Size::new(W as u64, H as u64).unwrap()
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
        ((self.gap % W) as u64, (self.gap / W) as u64)
    }

    fn try_gap_position_xy(&self) -> Option<(u64, u64)> {
        Some(self.gap_position_xy())
    }

    unsafe fn gap_position_xy_unchecked(&self) -> (u64, u64) {
        self.gap_position_xy()
    }

    fn reset(&mut self) {
        self.pieces = Self::SOLVED;
        self.gap = const { W * H - 1 };
    }

    fn swap_piece_with_gap(&mut self, idx: u64) {
        let gap = self.gap as usize;
        let piece = self.piece_at(idx) as usize;

        self.pieces ^= SWAP_MASKS[gap][idx as usize][piece];
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
        GAPS[self.gap as usize][dir as usize] != self.gap
    }

    fn move_dir(&mut self, dir: Direction) {
        self.try_move_dir(dir);
    }

    fn try_move_dir(&mut self, dir: Direction) -> bool {
        let gap = self.gap as usize;
        let dir = dir as usize;

        let shift = SHIFTS[gap][dir] as u64;
        let piece = ((self.pieces >> shift) & 0xF) as usize;

        let mask = MOVE_MASKS[gap][dir][piece];
        self.pieces ^= mask;

        let next_gap = GAPS[gap][dir];
        self.gap = next_gap;

        next_gap != gap as u8
    }

    unsafe fn move_dir_unchecked(&mut self, dir: Direction) {
        self.move_dir(dir);
    }
}
