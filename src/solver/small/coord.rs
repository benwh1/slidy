//! An indexing of the solvable states of a small `WxH` puzzle that can be updated cheaply when a
//! move is made.
//!
//! The index of a state is built from the cells of its pieces, in the order: gap, then pieces
//! `1, 2, ..., N - 1`. The digit of the gap is its cell, and the digit of piece `p` is the number
//! of cells before it that are not occupied by the gap or by pieces less than `p` (an inverse
//! Lehmer code). The digits of pieces `N - 2` and `N - 1` are dropped, because the parity of the
//! permutation fixes them, which halves the number of states to `N! / 2`.
//!
//! The gap is the least significant digit, so horizontal moves change the index by exactly 1. The
//! index is `block * S + gap`, where `block` is the code of the pieces and the stride `S` is
//! either `N` (so the indices are exactly `0..N! / 2`) or 16 (so that each block fits in an
//! aligned 16-bit lane of a bitmap, which is faster to work with when building tables).
//!
//! A move changes the digits of the gap, the moved piece, and the pieces that it jumps over (when
//! it moves vertically), so the change in the index is a short sum of precomputed terms.

/// The largest number of cells supported. All the precomputed tables have this many entries.
const MAX_N: usize = 16;

/// Index computations for `WxH` puzzles, with stride `S`.
pub(super) struct Coord<const W: usize, const H: usize, const S: usize>;

/// [`Coord`] with stride `N`, so that the indices are exactly `0..N! / 2`.
pub(super) type Dense<const W: usize, const H: usize, const N: usize> = Coord<W, H, N>;

/// [`Coord`] with stride 16.
pub(super) type Padded<const W: usize, const H: usize> = Coord<W, H, MAX_N>;

const fn factorial(n: usize) -> u64 {
    let mut out = 1;
    let mut i = 2;
    while i <= n {
        out *= i as u64;
        i += 1;
    }
    out
}

impl<const W: usize, const H: usize, const S: usize> Coord<W, H, S> {
    pub(super) const N: usize = {
        assert!(W >= 2 && H >= 2 && W * H <= MAX_N && W * H <= S && S <= MAX_N);
        W * H
    };

    /// The number of solvable states, `N! / 2`.
    pub(super) const SIZE: u64 = factorial(Self::N) / 2;

    /// The number of blocks, `(N - 1)! / 2`.
    pub(super) const BLOCKS: u64 = Self::SIZE / Self::N as u64;

    /// The number of indices, including those of unused positions when `S > N`.
    #[cfg(test)]
    pub(super) const LEN: u64 = Self::BLOCKS * S as u64;

    /// `WEIGHT[p]` is the weight of the digit of piece `p` (0 is the gap).
    const WEIGHT: [u64; MAX_N] = {
        let mut out = [0; MAX_N];
        out[0] = 1;
        let mut p = 1;
        while p + 2 < Self::N {
            out[p] = S as u64 * (factorial(Self::N - 1 - p) / 2);
            p += 1;
        }
        out
    };

    /// `CONTRIB[m][p]` is the change in the index from piece `p` being jumped over when piece
    /// `m` moves up (so the gap moves down).
    const CONTRIB: [[u64; MAX_N]; MAX_N] = {
        let mut out = [[0; MAX_N]; MAX_N];
        let mut m = 1;
        while m < MAX_N {
            let mut p = 1;
            while p < m {
                out[m][p] = Self::WEIGHT[m].wrapping_add(Self::WEIGHT[p]);
                p += 1;
            }
            m += 1;
        }
        out
    };

    /// `VERT[m]` is the part of the change in the index when piece `m` moves up (so the gap moves
    /// down) that doesn't depend on the pieces jumped over.
    const VERT: [u64; MAX_N] = {
        let mut out = [0; MAX_N];
        let mut m = 1;
        while m < MAX_N {
            out[m] = (W as u64 * Self::WEIGHT[0])
                .wrapping_sub((W as u64 - 1).wrapping_mul(Self::WEIGHT[m]));
            m += 1;
        }
        out
    };

    /// `RADIX_WEIGHT[p]` is `WEIGHT[p] / S`, the weight of the digit of piece `p` in the block.
    const RADIX_WEIGHT: [u64; MAX_N] = {
        let mut out = [0; MAX_N];
        let mut p = 1;
        while p + 2 < Self::N {
            out[p] = Self::WEIGHT[p] / S as u64;
            p += 1;
        }
        out
    };

    /// The index of the state with the given pieces (4 bits per cell) and gap.
    pub(super) fn index(pieces: u64, gap: u8) -> u64 {
        let mut cell_of = [0u8; MAX_N];
        for cell in 0..Self::N {
            cell_of[((pieces >> (4 * cell)) & 0xF) as usize] = cell as u8;
        }

        let mut seen = 1u32 << gap;
        let mut index = u64::from(gap);
        for (&cell, &weight) in cell_of[1..Self::N - 2].iter().zip(&Self::WEIGHT[1..]) {
            let cell = cell as u32;
            let digit = cell - (seen & ((1 << cell) - 1)).count_ones();
            seen |= 1 << cell;
            index += digit as u64 * weight;
        }

        index
    }

    /// The change in the index when piece `m` moves vertically past the pieces in the low `W - 1`
    /// nibbles of `jumped`, with the gap moving down. If `m` is 0, the change is 0.
    #[inline(always)]
    fn vertical_delta(m: u64, jumped: u64) -> u64 {
        let m = m as usize;
        let mut delta = Self::VERT[m];
        for k in 0..W - 1 {
            let p = ((jumped >> (4 * k)) & 0xF) as usize;
            delta = delta.wrapping_add(Self::CONTRIB[m][p]);
        }
        delta
    }

    /// The change in the index when the gap moves down from `gap` to `gap + W`, where `pieces`
    /// is the state before the move, or 0 if the gap is in the bottom row.
    #[inline(always)]
    pub(super) fn delta_down(pieces: u64, gap: u8) -> u64 {
        // The cells `gap + 1..=gap + W`. If the gap is in the bottom row, the last is past the end
        // of the puzzle, so it's 0.
        let window = (pieces >> (4 * u32::from(gap))) >> 4;
        let m = (window >> (4 * (W - 1))) & 0xF;
        Self::vertical_delta(m, window)
    }

    /// The change in the index when the gap moves up from `gap` to `gap - W`, where `pieces` is
    /// the state before the move, or 0 if the gap is in the top row.
    #[inline(always)]
    pub(super) fn delta_up(pieces: u64, gap: u8) -> u64 {
        // The cells `gap - W..gap`. If the gap is in the top row, the first is before the start of
        // the puzzle, so it's 0.
        let window = ((u128::from(pieces) << (4 * W)) >> (4 * u32::from(gap))) as u64;
        let m = window & 0xF;
        Self::vertical_delta(m, window >> 4).wrapping_neg()
    }

    /// Whether the permutation of the pieces (in reading order, skipping the gap) of a solvable
    /// state with the gap at `gap` is odd.
    pub(super) const fn is_odd(gap: u8) -> bool {
        let rows_from_bottom = H - 1 - gap as usize / W;
        (W - 1) * rows_from_bottom % 2 == 1
    }

    /// Decodes `block = index / S` into the pieces of the state in reading order, skipping the
    /// gap (4 bits each).
    ///
    /// Returns the two possible orders of the pieces, which differ by swapping pieces `N - 2` and
    /// `N - 1`. The first is an even permutation and the second is odd.
    #[cfg(test)]
    pub(super) fn decode_block(mut block: u64) -> [u64; 2] {
        // The ranks (positions in reading order, skipping the gap) not yet taken, 4 bits each.
        let mut free = 0xFEDC_BA98_7654_3210u64;
        let mut ranked = 0u64;
        let mut parity = 0;

        for p in 1..Self::N - 2 {
            let w = Self::RADIX_WEIGHT[p];
            let digit = block / w;
            block %= w;
            parity ^= digit;

            let shift = 4 * digit;
            let rank = (free >> shift) & 0xF;
            free = (free & ((1 << shift) - 1)) | ((free >> (shift + 4)) << shift);
            ranked |= (p as u64) << (4 * rank);
        }

        let (r0, r1) = (free & 0xF, (free >> 4) & 0xF);
        let n = Self::N as u64;
        let a = ranked | ((n - 2) << (4 * r0)) | ((n - 1) << (4 * r1));
        let b = ranked | ((n - 1) << (4 * r0)) | ((n - 2) << (4 * r1));

        if parity & 1 == 0 { [a, b] } else { [b, a] }
    }

    /// The radix of the digit of piece `p`.
    const fn radix(p: usize) -> u64 {
        (Self::N - p) as u64
    }

    /// Inserts the gap at `gap` into pieces in reading order skipping the gap, giving the pieces
    /// of each cell.
    pub(super) const fn insert_gap(ranked: u64, gap: u8) -> u64 {
        let shift = 4 * gap as u64;
        let low = ranked & ((1 << shift) - 1);
        let high = (ranked >> shift) << shift << 4;
        low | high
    }

    /// Decodes an index into the pieces (4 bits per cell) and gap of the state.
    #[cfg(test)]
    pub(super) fn decode(index: u64) -> (u64, u8) {
        let gap = (index % S as u64) as u8;
        let [even, odd] = Self::decode_block(index / S as u64);
        let ranked = if Self::is_odd(gap) { odd } else { even };
        (Self::insert_gap(ranked, gap), gap)
    }
}

/// Decodes blocks (see [`Coord::decode_block`]) incrementally: decoding a block close after the
/// previous one only redoes the digits that changed.
pub(super) struct BlockDecoder<const W: usize, const H: usize> {
    block: u64,
    /// `digits[p]` is the digit of piece `p`.
    digits: [u64; MAX_N],
    /// `free[p]` is the ranks not taken by pieces less than `p`, 4 bits each.
    free: [u64; MAX_N],
    /// `ranked[p]` is the pieces less than `p` at their ranks, 4 bits each.
    ranked: [u64; MAX_N],
    /// `parity[p]` is the parity of the digits of pieces less than `p`.
    parity: [u64; MAX_N],
    decoded: [u64; 2],
}

impl<const W: usize, const H: usize> BlockDecoder<W, H> {
    const N: usize = Padded::<W, H>::N;

    /// How far ahead a block can be to step to it digit by digit instead of decoding it from
    /// scratch.
    const MAX_STEP: u64 = 64;

    pub(super) fn new() -> Self {
        let mut out = Self {
            block: 0,
            digits: [0; MAX_N],
            free: [0; MAX_N],
            ranked: [0; MAX_N],
            parity: [0; MAX_N],
            decoded: [0; 2],
        };
        out.free[1] = 0xFEDC_BA98_7654_3210;
        out.redo_from(1);
        out
    }

    /// Recomputes the decoding from the digit of piece `from` onwards.
    #[inline(always)]
    fn redo_from(&mut self, from: usize) {
        let mut free = self.free[from];
        let mut ranked = self.ranked[from];
        let mut parity = self.parity[from];

        for p in from..Self::N - 2 {
            self.free[p] = free;
            self.ranked[p] = ranked;
            self.parity[p] = parity;

            let digit = self.digits[p];
            parity ^= digit;

            let shift = 4 * digit;
            let rank = (free >> shift) & 0xF;
            free = (free & ((1 << shift) - 1)) | ((free >> (shift + 4)) << shift);
            ranked |= (p as u64) << (4 * rank);
        }

        let (r0, r1) = (free & 0xF, (free >> 4) & 0xF);
        let n = Self::N as u64;
        let a = ranked | ((n - 2) << (4 * r0)) | ((n - 1) << (4 * r1));
        let b = ranked | ((n - 1) << (4 * r0)) | ((n - 2) << (4 * r1));

        self.decoded = if parity & 1 == 0 { [a, b] } else { [b, a] };
    }

    /// Decodes `block`, after which [`Self::ranked`] returns the pieces of its states.
    #[inline(always)]
    pub(super) fn decode(&mut self, block: u64) {
        if block == self.block {
            return;
        }

        let from = if block > self.block && block - self.block <= Self::MAX_STEP {
            // Add the difference to the least significant digit and carry.
            let mut p = Self::N - 3;
            let mut carry = block - self.block;
            loop {
                let sum = self.digits[p] + carry;
                let radix = Padded::<W, H>::radix(p);
                self.digits[p] = sum % radix;
                carry = sum / radix;
                if carry == 0 {
                    break p;
                }
                p -= 1;
            }
        } else {
            let mut rest = block;
            let mut from = 0;
            for (p, d) in self.digits.iter_mut().enumerate().take(Self::N - 2).skip(1) {
                let w = Padded::<W, H>::RADIX_WEIGHT[p];
                let digit = rest / w;
                rest %= w;
                if from == 0 && digit != *d {
                    from = p;
                }
                *d = digit;
            }
            from
        };

        self.block = block;
        self.redo_from(from);
    }

    /// Returns the pieces of the states in the last decoded block, in reading order skipping the
    /// gap, for the states where that permutation is odd if `odd` is true, or even otherwise.
    #[inline(always)]
    pub(super) fn ranked(&self, odd: bool) -> u64 {
        self.decoded[usize::from(odd)]
    }
}

#[cfg(test)]
mod tests {
    use rand::{RngExt as _, SeedableRng as _};
    use rand_xoshiro::Xoshiro256StarStar;

    use super::*;
    use crate::puzzle::{
        sliding_puzzle::SlidingPuzzle as _,
        small::{Puzzle, sealed::SmallPuzzle},
    };

    fn check<const W: usize, const H: usize, const S: usize>()
    where
        Puzzle<W, H>: SmallPuzzle,
    {
        let n = (W * H) as u64;
        let len = Coord::<W, H, S>::LEN;
        let mut rng = Xoshiro256StarStar::seed_from_u64(1);
        let indices: Vec<u64> = if len <= 2_000_000 {
            (0..len).filter(|i| i % (S as u64) < n).collect()
        } else {
            std::iter::repeat_with(|| rng.random_range(0..len))
                .filter(|i| i % (S as u64) < n)
                .take(200_000)
                .collect()
        };

        let solved = Puzzle::<W, H>::new();
        assert_eq!(
            Coord::<W, H, S>::index(solved.pieces(), solved.gap()),
            W as u64 * H as u64 - 1
        );

        for index in indices {
            let (pieces, gap) = Coord::<W, H, S>::decode(index);
            assert_eq!(Coord::<W, H, S>::index(pieces, gap), index);

            // SAFETY: `decode` returns a valid state.
            let puzzle = unsafe { Puzzle::<W, H>::with_pieces_and_gap_unchecked(pieces, gap) };
            assert!(puzzle.is_solvable());

            let g = gap as usize;
            if g < W {
                assert_eq!(Coord::<W, H, S>::delta_up(pieces, gap), 0);
            }
            if g + W >= W * H {
                assert_eq!(Coord::<W, H, S>::delta_down(pieces, gap), 0);
            }
            if g + W < W * H {
                let mut p = puzzle;
                p.swap_piece_with_gap((g + W) as u64);
                let expected = Coord::<W, H, S>::index(p.pieces(), p.gap());
                let delta = Coord::<W, H, S>::delta_down(pieces, gap);
                assert_eq!(index.wrapping_add(delta), expected);
            }
            if g >= W {
                let mut p = puzzle;
                p.swap_piece_with_gap((g - W) as u64);
                let expected = Coord::<W, H, S>::index(p.pieces(), p.gap());
                let delta = Coord::<W, H, S>::delta_up(pieces, gap);
                assert_eq!(index.wrapping_add(delta), expected);
            }
            if g % W + 1 < W {
                let mut p = puzzle;
                p.swap_piece_with_gap((g + 1) as u64);
                assert_eq!(Coord::<W, H, S>::index(p.pieces(), p.gap()), index + 1);
            }
        }
    }

    fn check_block_decoder<const W: usize, const H: usize>() {
        let blocks = Padded::<W, H>::BLOCKS;
        let mut rng = Xoshiro256StarStar::seed_from_u64(2);
        let mut decoder = BlockDecoder::<W, H>::new();
        let mut block = 0;

        for _ in 0..100_000 {
            decoder.decode(block);
            let [even, odd] = Padded::<W, H>::decode_block(block);
            assert_eq!(decoder.ranked(false), even);
            assert_eq!(decoder.ranked(true), odd);

            // Mostly small steps forward, sometimes large ones or backwards.
            block = match rng.random_range(0..10) {
                0 => rng.random_range(0..blocks),
                1..5 => block + 1,
                _ => block + rng.random_range(0..100),
            };
            if block >= blocks {
                block = rng.random_range(0..blocks);
            }
        }
    }

    macro_rules! test {
        ($w:literal, $h:literal) => {
            ::paste::paste! {
                #[test]
                fn [< test_coord_ $w x $h >]() {
                    check::<$w, $h, { $w * $h }>();
                    check::<$w, $h, 16>();
                }

                #[test]
                fn [< test_block_decoder_ $w x $h >]() {
                    check_block_decoder::<$w, $h>();
                }
            }
        };
    }

    test!(2, 2);
    test!(2, 3);
    test!(2, 4);
    test!(2, 5);
    test!(2, 6);
    test!(3, 2);
    test!(3, 3);
    test!(3, 4);
    test!(4, 2);
    test!(4, 3);
    test!(5, 2);
    test!(6, 2);
}
