//! An indexing of the solvable states of a small `WxH` puzzle that can be updated cheaply when a
//! move is made.
//!
//! The index of a state is built from the cells of its pieces, in the order: gap, then pieces
//! `1, 2, ..., N - 1`. The digit of the gap is its cell, and the digit of piece `p` is the number
//! of cells before it that are not occupied by the gap or by pieces less than `p` (an inverse
//! Lehmer code). The gap is the least significant digit, so horizontal moves change the index by
//! exactly 1. The digits of pieces `N - 2` and `N - 1` are dropped, because the parity of the
//! permutation fixes them, which halves the index range to the `N! / 2` solvable states.
//!
//! A move changes the digits of the gap, the moved piece, and the pieces that it jumps over (when
//! it moves vertically), so the change in the index is a short sum of precomputed terms.

/// The largest number of cells supported. All the precomputed tables have this many entries.
const MAX_N: usize = 16;

/// Index computations for `WxH` puzzles.
pub(super) struct Coord<const W: usize, const H: usize>;

const fn factorial(n: usize) -> u64 {
    let mut out = 1;
    let mut i = 2;
    while i <= n {
        out *= i as u64;
        i += 1;
    }
    out
}

impl<const W: usize, const H: usize> Coord<W, H> {
    const N: usize = {
        assert!(W >= 2 && H >= 2 && W * H <= MAX_N);
        W * H
    };

    /// The number of solvable states, `N! / 2`.
    pub(super) const SIZE: u64 = factorial(Self::N) / 2;

    /// `WEIGHT[p]` is the weight of the digit of piece `p` (0 is the gap).
    const WEIGHT: [u64; MAX_N] = {
        let mut out = [0; MAX_N];
        out[0] = 1;
        let mut p = 1;
        while p + 2 < Self::N {
            out[p] = Self::N as u64 * (factorial(Self::N - 1 - p) / 2);
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

    /// `RADIX_WEIGHT[p]` is `WEIGHT[p] / N`, the weight of the digit of piece `p` in `index / N`.
    const RADIX_WEIGHT: [u64; MAX_N] = {
        let mut out = [0; MAX_N];
        let mut p = 1;
        while p + 2 < Self::N {
            out[p] = Self::WEIGHT[p] / Self::N as u64;
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
        let mut index = gap as u64;
        for (&cell, &weight) in cell_of[1..Self::N - 2].iter().zip(&Self::WEIGHT[1..]) {
            let cell = cell as u32;
            let digit = cell - (seen & ((1 << cell) - 1)).count_ones();
            seen |= 1 << cell;
            index += digit as u64 * weight;
        }

        index
    }

    /// The change in the index when the gap moves down from `gap` to `gap + W`, where `pieces`
    /// is the state before the move.
    #[inline(always)]
    pub(super) fn delta_down(pieces: u64, gap: u8) -> u64 {
        let gap = gap as usize;
        let m = ((pieces >> (4 * (gap + W))) & 0xF) as usize;
        let mut delta = Self::VERT[m];
        for cell in gap + 1..gap + W {
            let p = ((pieces >> (4 * cell)) & 0xF) as usize;
            delta = delta.wrapping_add(Self::CONTRIB[m][p]);
        }
        delta
    }

    /// The change in the index when the gap moves up from `gap` to `gap - W`, where `pieces` is
    /// the state before the move.
    #[inline(always)]
    pub(super) fn delta_up(pieces: u64, gap: u8) -> u64 {
        let gap = gap as usize;
        let m = ((pieces >> (4 * (gap - W))) & 0xF) as usize;
        let mut delta = Self::VERT[m];
        for cell in gap + 1 - W..gap {
            let p = ((pieces >> (4 * cell)) & 0xF) as usize;
            delta = delta.wrapping_add(Self::CONTRIB[m][p]);
        }
        delta.wrapping_neg()
    }

    /// Whether the permutation of the pieces (in reading order, skipping the gap) of a solvable
    /// state with the gap at `gap` is odd.
    pub(super) const fn is_odd(gap: u8) -> bool {
        let rows_from_bottom = H - 1 - gap as usize / W;
        (W - 1) * rows_from_bottom % 2 == 1
    }

    /// Decodes `block = index / N` into the pieces of the state in reading order, skipping the
    /// gap (4 bits each).
    ///
    /// Returns the two possible orders of the pieces, which differ by swapping pieces `N - 2` and
    /// `N - 1`. The first is an even permutation and the second is odd.
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
        let n = Self::N as u64;
        let gap = (index % n) as u8;
        let [even, odd] = Self::decode_block(index / n);
        let ranked = if Self::is_odd(gap) { odd } else { even };
        (Self::insert_gap(ranked, gap), gap)
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

    fn check<const W: usize, const H: usize>()
    where
        Puzzle<W, H>: SmallPuzzle,
    {
        let size = Coord::<W, H>::SIZE;
        let mut rng = Xoshiro256StarStar::seed_from_u64(1);
        let indices: Vec<u64> = if size <= 2_000_000 {
            (0..size).collect()
        } else {
            std::iter::repeat_with(|| rng.random_range(0..size))
                .take(200_000)
                .collect()
        };

        let solved = Puzzle::<W, H>::new();
        assert_eq!(
            Coord::<W, H>::index(solved.pieces(), solved.gap()),
            W as u64 * H as u64 - 1
        );

        for index in indices {
            let (pieces, gap) = Coord::<W, H>::decode(index);
            assert_eq!(Coord::<W, H>::index(pieces, gap), index);

            // SAFETY: `decode` returns a valid state.
            let puzzle = unsafe { Puzzle::<W, H>::with_pieces_and_gap_unchecked(pieces, gap) };
            assert!(puzzle.is_solvable());

            let g = gap as usize;
            if g + W < W * H {
                let mut p = puzzle;
                p.swap_piece_with_gap((g + W) as u64);
                let expected = Coord::<W, H>::index(p.pieces(), p.gap());
                let delta = Coord::<W, H>::delta_down(pieces, gap);
                assert_eq!(index.wrapping_add(delta), expected);
            }
            if g >= W {
                let mut p = puzzle;
                p.swap_piece_with_gap((g - W) as u64);
                let expected = Coord::<W, H>::index(p.pieces(), p.gap());
                let delta = Coord::<W, H>::delta_up(pieces, gap);
                assert_eq!(index.wrapping_add(delta), expected);
            }
            if g % W + 1 < W {
                let mut p = puzzle;
                p.swap_piece_with_gap((g + 1) as u64);
                assert_eq!(Coord::<W, H>::index(p.pieces(), p.gap()), index + 1);
            }
        }
    }

    macro_rules! test {
        ($w:literal, $h:literal) => {
            ::paste::paste! {
                #[test]
                fn [< test_coord_ $w x $h >]() {
                    check::<$w, $h>();
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
