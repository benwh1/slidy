//! Defines a fixed-size puzzle with at most 15 pieces. The puzzle grid is represented using a
//! single `u64` with 4 bits per piece. Moves are applied efficiently using bit operations.

use num_traits::AsPrimitive;

use crate::{
    algorithm::{direction::Direction, r#move::r#move::Move},
    puzzle::{
        label::label::BijectiveLabel,
        puzzle::PuzzleError,
        size::Size,
        sliding_puzzle::{self, SlidingPuzzle},
    },
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

#[expect(clippy::inline_modules)]
pub(crate) mod sealed {
    use crate::puzzle::sliding_puzzle::SlidingPuzzle;

    pub trait SmallPuzzle: SlidingPuzzle<Piece = u8> {
        const W: usize;
        const H: usize;
        const N: usize;

        type PieceArray;
        type TransposedPuzzle;

        fn new() -> Self;
        unsafe fn with_pieces_and_gap_unchecked(pieces: u64, gap: u8) -> Self;
        unsafe fn from_piece_array_unchecked(piece_array: Self::PieceArray) -> Self;

        fn pieces(&self) -> u64;
        fn gap(&self) -> u8;

        fn piece_array(&self) -> Self::PieceArray;
        fn conjugate_with_transpose(&self) -> Self::TransposedPuzzle;
    }
}

use sealed::SmallPuzzle;

/// Returns the position of the lowest nibble of `pieces` equal to `value`, or 16 if there is none.
#[inline(always)]
pub(crate) const fn find_nibble(pieces: u64, value: u8) -> u32 {
    const ONES: u64 = 0x1111_1111_1111_1111;
    const HIGHS: u64 = 0x8888_8888_8888_8888;

    // The nibbles equal to `value` are the zero nibbles of `x`. A zero nibble sets its high bit in
    // `zeros`, and so can a nibble above a zero one, because of the borrow, but the lowest bit set
    // in `zeros` is always the lowest zero nibble.
    let x = pieces ^ (value as u64 * ONES);
    let zeros = x.wrapping_sub(ONES) & !x & HIGHS;
    zeros.trailing_zeros() / 4
}

macro_rules! impl_puzzle {
    ($w:literal, $h:literal) => {
        impl Puzzle<$w, $h> {
            pub(crate) const SOLVED: u64 = {
                let mut out = 0;

                let mut i = 0;
                while i < Self::N as u64 - 1 {
                    out |= (i + 1) << (4 * i);
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

            /// The longest possible move.
            const MAX_AMOUNT: usize = if $w > $h { $w - 1 } else { $h - 1 };

            /// `DISTANCES[gap][dir]` is how far the gap can move in direction `dir`, i.e. the
            /// largest amount of a move in direction `dir` that can be applied.
            const DISTANCES: [[u8; 4]; Self::N] = {
                let mut out = [[0; 4]; Self::N];

                let mut i = 0;
                while i < Self::N as u8 {
                    let (gx, gy) = (i % $w, i / $w);
                    out[i as usize] = [$h - 1 - gy, $w - 1 - gx, gy, gx];
                    i += 1;
                }

                out
            };

            /// `MULTI_MASKS[gap][dir][amount]` selects the pieces moved by a move in direction
            /// `dir` of `amount` pieces, with the gap at `gap`. It is 0 if the move can't be
            /// applied.
            pub(crate) const MULTI_MASKS: [[[u64; Self::MAX_AMOUNT + 1]; 4]; Self::N] = {
                let mut out = [[[0; Self::MAX_AMOUNT + 1]; 4]; Self::N];

                let mut gap = 0;
                while gap < Self::N {
                    let mut dir = 0;
                    while dir < 4 {
                        let mut amount = 1;
                        while amount <= Self::DISTANCES[gap][dir] as usize {
                            let cell = Self::NEXT_GAPS[gap][dir][amount] as u64;
                            out[gap][dir][amount] = out[gap][dir][amount - 1] | (0xF << (4 * cell));
                            amount += 1;
                        }
                        dir += 1;
                    }
                    gap += 1;
                }

                out
            };

            /// `NEXT_GAPS[gap][dir][amount]` is the position of the gap after a move in direction
            /// `dir` of `amount` pieces, with the gap at `gap`.
            const NEXT_GAPS: [[[u8; Self::MAX_AMOUNT + 1]; 4]; Self::N] = {
                let mut out = [[[0; Self::MAX_AMOUNT + 1]; 4]; Self::N];

                let mut gap = 0;
                while gap < Self::N {
                    let mut dir = 0;
                    while dir < 4 {
                        out[gap][dir][0] = gap as u8;
                        let mut amount = 1;
                        while amount <= Self::MAX_AMOUNT {
                            out[gap][dir][amount] =
                                Self::GAPS[out[gap][dir][amount - 1] as usize][dir];
                            amount += 1;
                        }
                        dir += 1;
                    }
                    gap += 1;
                }

                out
            };

            /// `ROTATIONS[dir]` rotates the pieces moved by a move in direction `dir` into their
            /// new positions. The moved pieces never wrap around, so this is a shift by one row or
            /// column.
            pub(crate) const ROTATIONS: [u32; 4] = [4 * $w, 4, 64 - 4 * $w, 60];

            /// Rearranges the puzzle so that the piece at each position `i` is `target(i)`, like
            /// [`sliding_puzzle::rearrange`], which this falls back to if the targets aren't a
            /// permutation of the pieces.
            fn rearrange_to(&mut self, mut target: impl FnMut(u64) -> u8) {
                let mut pieces = 0;
                let mut gap = 0;
                let mut seen = 0u32;

                for i in 0..Self::N as u64 {
                    let piece = target(i);
                    if usize::from(piece) < Self::N {
                        seen |= 1 << piece;
                    }
                    pieces |= u64::from(piece) << (4 * i);
                    if piece == 0 {
                        gap = i as u8;
                    }
                }

                if seen == (1 << Self::N) - 1 {
                    self.pieces = pieces;
                    self.gap = gap;
                } else {
                    sliding_puzzle::rearrange(self, target);
                }
            }

            pub(crate) const SWAP_MASKS: [[[u64; Self::N]; Self::N]; Self::N] = {
                #[allow(clippy::large_stack_arrays)]
                let mut out = [[[0; Self::N]; Self::N]; Self::N];

                let n = Self::N as u64;

                let mut gap = 0;
                while gap < n {
                    let mut other = 0;
                    while other < n {
                        if gap != other {
                            let mut piece = 0;
                            while piece < n {
                                let g = gap as usize;
                                let o = other as usize;
                                let p = piece as usize;
                                out[g][o][p] = (piece << (gap * 4)) | (piece << (other * 4));
                                piece += 1;
                            }
                        }
                        other += 1;
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
            type TransposedPuzzle = Puzzle<$h, $w>;

            fn new() -> Self {
                Self {
                    pieces: Self::SOLVED,
                    gap: Self::N as u8 - 1,
                }
            }

            /// Creates a new [`Puzzle`] with the given `pieces` and `gap`, without checking that
            /// the puzzle state is valid or that `gap` matches the given state.
            ///
            /// # Safety
            ///
            /// The lower `W * H` nibbles of `pieces` must contain the values 0 to `W * H - 1`,
            /// exactly once each.
            /// `gap` must be less than `W * H`, and `(pieces >> (gap * 4)) & 0xF` must be 0.
            ///
            /// This function is unsafe because, although it doesn't cause immediate UB if used
            /// incorrectly, can break the type's invariant, which could lead to UB in
            /// otherwise-correct unsafe code elsewhere.
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

            fn conjugate_with_transpose(&self) -> Self::TransposedPuzzle {
                let mut transposed_pieces = [0; Self::TransposedPuzzle::N];

                for y in 0..Self::H {
                    for x in 0..Self::W {
                        let piece = self.piece_at_xy((x as u64, y as u64));
                        let (sx, sy) = self.solved_pos_xy(piece);
                        let transposed_piece = {
                            let p = 1 + sy + Self::H as u64 * sx;
                            if p == Self::N as u64 { 0 } else { p as u8 }
                        };
                        transposed_pieces[y + Self::H * x] = transposed_piece;
                    }
                }

                // SAFETY: The entries of `transposed_pieces` are a permutation of the pieces of
                // `self`, so contain the values 0 to `W * H - 1` once each.
                unsafe { Self::TransposedPuzzle::from_piece_array_unchecked(transposed_pieces) }
            }
        }

        impl SlidingPuzzle for Puzzle<$w, $h> {
            type Piece = u8;

            #[inline]
            fn size(&self) -> Size {
                Size::new($w, $h).unwrap()
            }

            #[inline]
            fn area(&self) -> u64 {
                $w * $h
            }

            #[inline]
            fn num_pieces(&self) -> u64 {
                $w * $h - 1
            }

            fn try_piece_position(&self, piece: Self::Piece) -> Option<u64> {
                if usize::from(piece) >= Self::N {
                    return None;
                }

                // The unused nibbles above the puzzle are 0, so they only match the gap, which
                // comes first.
                Some(u64::from(find_nibble(self.pieces, piece)))
            }

            #[inline]
            fn piece_at(&self, idx: u64) -> Self::Piece {
                ((self.pieces >> (idx * 4)) & 0xF) as u8
            }

            #[inline]
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

            #[inline]
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

            #[inline]
            fn reset(&mut self) {
                *self = Self::new();
            }

            fn reset_to_label<L>(&mut self, label: &L)
            where
                L: BijectiveLabel,
            {
                // The same as the default implementation, but without allocating.
                let size = self.size();
                let w = size.width();
                let area = <u8 as num_traits::NumCast>::from(size.area()).unwrap();
                self.rearrange_to(|i| {
                    let label = label.position_label(size, (i % w, i / w));
                    let a = <u8 as num_traits::NumCast>::from(label).unwrap() + 1;
                    if a == area { 0 } else { a }
                });
            }

            fn try_set_state<P>(&mut self, other: &P) -> bool
            where
                P: SlidingPuzzle,
                P::Piece: AsPrimitive<Self::Piece>,
                Self::Piece: 'static,
            {
                if self.size() == other.size() {
                    self.rearrange_to(|i| other.piece_at(i).as_());
                    true
                } else {
                    false
                }
            }

            fn try_invert(&mut self) -> bool {
                // The same as the default implementation, but without allocating.
                if !self.invertible() {
                    return false;
                }

                let mut inv = [0; $w * $h];
                for i in 0..Self::N as u64 {
                    let piece = self
                        .piece_at(self.solved_pos(<u8 as num_traits::NumCast>::from(i).unwrap()));
                    inv[self.solved_pos(piece) as usize] = i;
                }

                self.rearrange_to(|i| <u8 as num_traits::NumCast>::from(inv[i as usize]).unwrap());

                true
            }

            fn swap_piece_with_gap(&mut self, idx: u64) {
                let gap = self.gap as usize;
                let piece = self.piece_at(idx) as usize;

                self.pieces ^= Self::SWAP_MASKS[gap][idx as usize][piece];
                self.gap = idx as u8;
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

                // The piece that moves into the gap, which is 0 if the move isn't possible.
                let moved = self.pieces & Self::MULTI_MASKS[gap][dir][1];
                self.pieces ^= moved | moved.rotate_right(Self::ROTATIONS[dir]);

                let next_gap = Self::GAPS[gap][dir];
                self.gap = next_gap;

                next_gap != gap as u8
            }

            unsafe fn move_dir_unchecked(&mut self, dir: Direction) {
                self.move_dir(dir);
            }

            fn is_solved(&self) -> bool {
                // Each state has its own `pieces`, so this is the same as checking every piece.
                self.pieces == Self::SOLVED
            }

            fn is_solvable(&self) -> bool {
                // A state is solvable if and only if the parity of the permutation taking each
                // position to the solved position of its piece is the parity of the gap's
                // distance from the bottom right corner.
                //
                // The parity of the permutation is the parity of the number of inversions. For
                // each position, `seen >> solved` has a bit for each earlier piece that belongs
                // after this one, and the parity of the total of these bits is the parity of the
                // bits in their XOR.
                let mut seen = 0u16;
                let mut inversions = 0u16;
                for i in 0..Self::N {
                    let piece = ((self.pieces >> (4 * i)) & 0xF) as usize;
                    let solved = if piece == 0 { Self::N - 1 } else { piece - 1 };
                    inversions ^= seen >> solved;
                    seen |= 1 << solved;
                }

                let (gx, gy) = self.gap_position_xy();
                let distance = ($w - 1 - gx) + ($h - 1 - gy);

                (inversions.count_ones() as u64 + distance) % 2 == 0
            }

            fn can_apply_move(&self, mv: Move) -> bool {
                mv.amount <= u64::from(Self::DISTANCES[self.gap as usize][mv.direction as usize])
            }

            fn apply_move(&mut self, mv: Move) {
                let gap = self.gap as usize;
                let dir = mv.direction as usize;

                // Moving further than possible moves as far as possible.
                let amount = mv.amount.min(u64::from(Self::DISTANCES[gap][dir])) as usize;

                let mask = Self::MULTI_MASKS[gap][dir][amount];
                let moved = (self.pieces & mask).rotate_right(Self::ROTATIONS[dir]);
                self.pieces = (self.pieces & !mask) | moved;
                self.gap = Self::NEXT_GAPS[gap][dir][amount];
            }

            unsafe fn apply_move_unchecked(&mut self, mv: Move) {
                self.apply_move(mv);
            }
        }

        impl TryFrom<[u8; $w * $h]> for Puzzle<$w, $h> {
            type Error = PuzzleError;

            fn try_from(value: [u8; $w * $h]) -> Result<Self, Self::Error> {
                let mut seen = [false; $w * $h];
                for &piece in &value {
                    if piece >= $w * $h {
                        return Err(PuzzleError::PieceOutOfRange(piece as u64));
                    }

                    if seen[piece as usize] {
                        return Err(PuzzleError::DuplicatePiece(piece as u64));
                    }

                    seen[piece as usize] = true;
                }

                // SAFETY: We checked that all the pieces are distinct and within range, so `value`
                // contains the values 0 to `W * H - 1` once each.
                unsafe { Ok(Self::from_piece_array_unchecked(value)) }
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

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::puzzle::puzzle::Puzzle;

    #[test]
    fn test_conjugate_with_transpose() {
        let p = Puzzle::from_str("4 8 0 6 5 9/10 11 1 7 3 2").unwrap();
        let pt = Puzzle::from_str("7 8/4 10/0 1/11 2/9 5/6 3").unwrap();

        let mut p2 = Puzzle6x2::new();
        assert!(p2.try_set_state(&p));
        let p3 = p2.conjugate_with_transpose();
        assert!(pt.equals(&p3));
    }

    /// Checks that every [`SlidingPuzzle`] method of the small puzzle types agrees with the
    /// generic [`Puzzle`], over random sequences of operations on every size.
    mod equivalence {
        use rand::{RngExt as _, SeedableRng as _};
        use rand_xoshiro::Xoroshiro128StarStar;

        use super::super::{Puzzle as Small, SmallPuzzle};
        use crate::{
            algorithm::{algorithm::Algorithm, direction::Direction, r#move::r#move::Move},
            puzzle::{
                label::label::{FringeGrids, RowGrids, SpiralGrids},
                puzzle::Puzzle,
                scrambler::{RandomInvertibleState, RandomState, Scrambler as _},
                size::Size,
                sliding_puzzle::SlidingPuzzle as _,
            },
        };

        const DIRS: [Direction; 4] = [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ];

        fn assert_same<const W: usize, const H: usize>(s: &Small<W, H>, g: &Puzzle)
        where
            Small<W, H>: SmallPuzzle<TransposedPuzzle = Small<H, W>>,
            Small<H, W>: SmallPuzzle,
        {
            let n = (W * H) as u64;

            assert!(s.equals(g), "{s:?} {g}");
            assert!(g.equals(s), "{s:?} {g}");
            assert_eq!(s.size(), g.size());
            assert_eq!(s.gap_position(), g.gap_position());
            assert_eq!(s.gap_position_xy(), g.gap_position_xy());
            assert_eq!(s.try_gap_position(), g.try_gap_position());
            assert_eq!(s.is_solved(), g.is_solved(), "{g}");
            assert_eq!(s.is_solvable(), g.is_solvable(), "{g}");
            assert_eq!(s.invertible(), g.invertible(), "{g}");

            for i in 0..n + 2 {
                assert_eq!(
                    s.try_piece_at(i).map(u64::from),
                    g.try_piece_at(i),
                    "{g} {i}"
                );
                assert_eq!(
                    s.try_piece_position(i as u8),
                    g.try_piece_position(i),
                    "{g} {i}"
                );
                assert_eq!(
                    s.try_piece_position_xy(i as u8),
                    g.try_piece_position_xy(i),
                    "{g} {i}"
                );
                assert_eq!(s.try_solved_pos(i as u8), g.try_solved_pos(i), "{i}");
                assert_eq!(s.can_move_position(i), g.can_move_position(i), "{g} {i}");
            }
            for piece in 0..n {
                assert_eq!(s.piece_position(piece as u8), g.piece_position(piece));
            }

            for dir in DIRS {
                assert_eq!(s.can_move_dir(dir), g.can_move_dir(dir), "{g} {dir:?}");
                for amount in 0..=(W.max(H) as u64 + 1) {
                    let mv = Move::new(dir, amount);
                    assert_eq!(s.can_apply_move(mv), g.can_apply_move(mv), "{g} {mv:?}");
                }
            }

            // Every piece in `piece_array` and `pieces`, and the gap field.
            let (pieces, gap) = (s.pieces(), s.gap());
            assert_eq!(u64::from(gap), g.gap_position());
            for i in 0..n {
                assert_eq!((pieces >> (4 * i)) & 0xF, g.piece_at(i), "{g} {i}");
            }
            assert_eq!(
                pieces.checked_shr(4 * n as u32).unwrap_or(0),
                0,
                "{pieces:x}"
            );

            // The reference definition of `conjugate_with_transpose`.
            let t = s.conjugate_with_transpose();
            for y in 0..H as u64 {
                for x in 0..W as u64 {
                    let (sx, sy) = g.solved_pos_xy(g.piece_at_xy((x, y)));
                    let p = 1 + sy + H as u64 * sx;
                    let expected = if p == n { 0 } else { p };
                    assert_eq!(u64::from(t.piece_at(y + H as u64 * x)), expected, "{g}");
                }
            }
            assert_eq!(t.piece_at(u64::from(t.gap())), 0);
        }

        fn random_alg(rng: &mut Xoroshiro128StarStar, max: u64) -> Algorithm {
            let len = rng.random_range(0..6);
            std::iter::repeat_with(|| {
                Move::new(DIRS[rng.random_range(0..4)], rng.random_range(1..=max))
            })
            .take(len)
            .collect()
        }

        fn check<const W: usize, const H: usize>(seed: u64)
        where
            Small<W, H>: SmallPuzzle<TransposedPuzzle = Small<H, W>>,
            Small<H, W>: SmallPuzzle,
        {
            let size = Size::new(W as u64, H as u64).unwrap();
            let n = (W * H) as u64;
            let max = W.max(H) as u64;

            let mut rng = Xoroshiro128StarStar::seed_from_u64(seed);
            let mut s = Small::<W, H>::new();
            let mut g = Puzzle::new(size);
            assert_same(&s, &g);

            for _ in 0..3000 {
                match rng.random_range(0..17) {
                    0..=3 => {
                        let dir = DIRS[rng.random_range(0..4)];
                        assert_eq!(s.try_move_dir(dir), g.try_move_dir(dir));
                    }
                    4 => {
                        let dir = DIRS[rng.random_range(0..4)];
                        if g.can_move_dir(dir) {
                            s.move_dir(dir);
                            g.move_dir(dir);
                        }
                    }
                    5 => {
                        let mv = Move::new(DIRS[rng.random_range(0..4)], rng.random_range(0..=max));
                        assert_eq!(s.try_apply_move(mv), g.try_apply_move(mv), "{mv:?}");
                    }
                    6 => {
                        let mv = Move::new(DIRS[rng.random_range(0..4)], rng.random_range(0..=max));
                        if g.can_apply_move(mv) {
                            s.apply_move(mv);
                            g.apply_move(mv);
                        }
                    }
                    7 if rng.random() => {
                        // Any amount, including too far, matches moving one piece at a time.
                        let mv =
                            Move::new(DIRS[rng.random_range(0..4)], rng.random_range(0..=max + 2));
                        let mut expected = s;
                        for _ in 0..mv.amount {
                            expected.try_move_dir(mv.direction);
                        }
                        s.apply_move(mv);
                        assert_eq!((s.pieces(), s.gap()), (expected.pieces(), expected.gap()));
                        g.set_state(&s);
                    }
                    7 => {
                        let alg = random_alg(&mut rng, max);
                        assert_eq!(s.can_apply_alg(&alg), g.can_apply_alg(&alg));
                        assert_eq!(s.try_apply_alg(&alg), g.try_apply_alg(&alg), "{alg}");
                    }
                    8 => {
                        let idx = rng.random_range(0..n + 1);
                        assert_eq!(
                            s.try_swap_piece_with_gap(idx),
                            g.try_swap_piece_with_gap(idx)
                        );
                    }
                    9 => {
                        let (i, j) = (rng.random_range(0..n + 1), rng.random_range(0..n + 1));
                        assert_eq!(s.try_swap_pieces(i, j), g.try_swap_pieces(i, j));
                    }
                    10 => {
                        let (i, j) = (rng.random_range(0..n + 1), rng.random_range(0..n + 1));
                        assert_eq!(
                            s.try_swap_non_gap_pieces(i, j),
                            g.try_swap_non_gap_pieces(i, j)
                        );
                    }
                    11 => {
                        let idx = rng.random_range(0..n + 1);
                        assert_eq!(s.try_move_position(idx), g.try_move_position(idx));
                    }
                    12 => {
                        let piece = rng.random_range(0..n + 1);
                        assert_eq!(s.try_move_piece(piece as u8), g.try_move_piece(piece));
                    }
                    13 => {
                        let mut other = Puzzle::new(size);
                        RandomState.scramble_with_rng(&mut other, &mut rng);
                        s.set_state(&other);
                        g.set_state(&other);
                    }
                    14 => {
                        // The same scramble from the same seed.
                        let seed: u64 = rng.random();
                        let rng = || Xoroshiro128StarStar::seed_from_u64(seed);
                        if seed % 2 == 0 {
                            RandomState.scramble_with_rng(&mut s, &mut rng());
                            RandomState.scramble_with_rng(&mut g, &mut rng());
                        } else {
                            RandomInvertibleState.scramble_with_rng(&mut s, &mut rng());
                            RandomInvertibleState.scramble_with_rng(&mut g, &mut rng());
                        }
                    }
                    15 => match rng.random_range(0..4) {
                        0 => {
                            s.reset();
                            g.reset();
                        }
                        1 => {
                            s.reset_to_label(&RowGrids);
                            g.reset_to_label(&RowGrids);
                        }
                        2 => {
                            s.reset_to_label(&FringeGrids);
                            g.reset_to_label(&FringeGrids);
                        }
                        _ => {
                            s.reset_to_label(&SpiralGrids);
                            g.reset_to_label(&SpiralGrids);
                        }
                    },
                    _ => {
                        assert_eq!(s.try_invert(), g.try_invert());
                    }
                }

                assert_same(&s, &g);

                // Round trips through the raw representations.
                // SAFETY: `s` is in a valid state, so its pieces and gap are valid.
                let s2 =
                    unsafe { Small::<W, H>::with_pieces_and_gap_unchecked(s.pieces(), s.gap()) };
                assert_same(&s2, &g);
                // SAFETY: `piece_array` returns a permutation of the pieces.
                let s3 = unsafe { Small::<W, H>::from_piece_array_unchecked(s.piece_array()) };
                assert_same(&s3, &g);
                let mut g2 = Puzzle::new(size);
                g2.set_state(&s);
                assert_eq!(g, g2);
            }
        }

        macro_rules! check_sizes {
            ($($w:literal x $h:literal),* $(,)?) => {
                #[test]
                fn test_equivalence() {
                    $(check::<$w, $h>($w * 100 + $h);)*
                }

                #[test]
                fn test_try_from() {
                    let mut rng = Xoroshiro128StarStar::seed_from_u64(0);
                    $({
                        const N: usize = $w * $h;
                        let mut g = Puzzle::new(Size::new($w, $h).unwrap());
                        for _ in 0..200 {
                            RandomState.scramble_with_rng(&mut g, &mut rng);
                            let mut array: [u8; N] =
                                std::array::from_fn(|i| g.piece_at(i as u64) as u8);
                            let s = Small::<$w, $h>::try_from(array).unwrap();
                            assert!(s.equals(&g));
                            assert_eq!(s.gap_position(), g.gap_position());

                            // Out of range and duplicate pieces.
                            let i = rng.random_range(0..N);
                            let old = array[i];
                            array[i] = rng.random_range(0..20);
                            let result = Small::<$w, $h>::try_from(array);
                            if array[i] == old {
                                assert!(result.is_ok());
                            } else if usize::from(array[i]) >= N {
                                assert!(matches!(
                                    result,
                                    Err(PuzzleError::PieceOutOfRange(p)) if p == u64::from(array[i])
                                ));
                            } else {
                                // The first repeated piece in the array is reported.
                                let mut seen = [false; N];
                                let dup = array
                                    .iter()
                                    .find(|&&p| std::mem::replace(&mut seen[usize::from(p)], true))
                                    .unwrap();
                                assert!(matches!(
                                    result,
                                    Err(PuzzleError::DuplicatePiece(p)) if p == u64::from(*dup)
                                ));
                            }
                        }
                    })*
                }
            };
        }

        use crate::puzzle::puzzle::PuzzleError;

        check_sizes!(
            2 x 2, 2 x 3, 2 x 4, 2 x 5, 2 x 6, 2 x 7, 2 x 8, 3 x 2, 3 x 3, 3 x 4, 3 x 5, 4 x 2,
            4 x 3, 4 x 4, 5 x 2, 5 x 3, 6 x 2, 7 x 2, 8 x 2,
        );
    }
}
