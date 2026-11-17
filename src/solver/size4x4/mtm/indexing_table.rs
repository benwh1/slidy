use std::array;

use crate::solver::{
    indexing,
    size4x4::mtm::consts::{SIZE, TALLY},
};

/// The number of distinct labels, including the gap's.
const LABELS: usize = TALLY.len();

pub(super) struct IndexingTable {
    high: Box<[u32]>,
    low: Box<[u16]>,
}

impl IndexingTable {
    pub(super) fn new() -> Self {
        /// Calls `f` with the base-8 digits and the piece counts of every arrangement of 8 cells
        /// with at most `TALLY[i]` pieces labeled `i`, in lexicographic order.
        fn go(
            cell: usize,
            digits: usize,
            counts: &mut [u8; LABELS],
            f: &mut impl FnMut(usize, &[u8; LABELS]),
        ) {
            if cell == 8 {
                f(digits, counts);
                return;
            }

            for piece in 0..LABELS {
                if counts[piece] < TALLY[piece] {
                    counts[piece] += 1;
                    go(cell + 1, 8 * digits + piece, counts, f);
                    counts[piece] -= 1;
                }
            }
        }

        let mut high = vec![0; LEN];
        let mut low = vec![0; LEN];

        // The next index for a prefix: every state with a lexicographically smaller prefix comes
        // first.
        let mut start = 0;
        // For each multiset of pieces, the rank of its next arrangement among all of its
        // arrangements.
        let mut ranks = vec![0; TALLY.iter().map(|&t| usize::from(t) + 1).product()];

        go(0, 0, &mut [0; LABELS], &mut |digits, counts| {
            high[digits] = start;
            // The states with this prefix have every arrangement of the remaining pieces as their
            // suffix.
            start +=
                indexing::multinomial(&array::from_fn::<_, LABELS, _>(|i| TALLY[i] - counts[i]))
                    as u32;

            let id = (0..LABELS).rev().fold(0, |id, i| {
                id * (usize::from(TALLY[i]) + 1) + usize::from(counts[i])
            });
            low[digits] = ranks[id];
            ranks[id] += 1;
        });

        debug_assert_eq!(start as usize, SIZE);

        Self {
            high: high.into_boxed_slice(),
            low: low.into_boxed_slice(),
        }
    }

    /// Returns the index of the first state whose cells 0..8 are `prefix`. Every state with these
    /// cells has an index in a run starting here.
    pub(super) fn encode_prefix(&self, prefix: u32) -> u32 {
        let high = base_8(u64::from(prefix)) as u32 as usize;

        // SAFETY: `base_8` returns halves less than `LEN`, the length of `self.high`.
        unsafe { *self.high.get_unchecked(high) }
    }

    /// Returns the offset of a state whose cells 8..16 are `suffix` within the run of indices
    /// given by [`Self::encode_prefix`].
    pub(super) fn encode_suffix(&self, suffix: u32) -> u32 {
        let low = base_8(u64::from(suffix)) as u32 as usize;

        // SAFETY: `base_8` returns halves less than `LEN`, the length of `self.low`.
        u32::from(unsafe { *self.low.get_unchecked(low) })
    }

    #[inline(always)]
    pub(super) fn encode(&self, puzzle: u64) -> u32 {
        let digits = base_8(puzzle);
        let high = digits as u32 as usize;
        let low = (digits >> 32) as usize;

        // SAFETY: `base_8` returns halves less than `LEN`, the length of `self.high` and
        // `self.low`.
        unsafe { *self.high.get_unchecked(high) + u32::from(*self.low.get_unchecked(low)) }
    }
}

/// The length of the tables: one more than the largest value of either half of [`base_8`].
const LEN: usize = 1 << 24;

/// Reads cells 0..8 and cells 8..16 of `pieces` as two 8 digit base-8 numbers, with the lower
/// cells more significant, and returns them in the low and high 32 bits. Only the lowest 3 bits of
/// each cell are read, so each half is less than [`LEN`].
#[inline(always)]
fn base_8(pieces: u64) -> u64 {
    const CELLS: u64 = 0x0707_0707_0707_0707;
    const PAIRS: u64 = 0x00FF_00FF_00FF_00FF;
    const QUADS: u64 = 0x0000_FFFF_0000_FFFF;

    // Each step combines neighboring fields, and no field overflows into the next: a pair is at
    // most 63, a quad at most 4095, and a half at most 8^8 - 1.
    let pairs = ((pieces & CELLS) << 3) | ((pieces >> 4) & CELLS);
    let quads = ((pairs & PAIRS) << 6) | ((pairs >> 8) & PAIRS);
    ((quads & QUADS) << 12) | ((quads >> 16) & QUADS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Calls `f` with every state, in lexicographic order with cell 0 most significant.
    fn for_each_state(
        cell: usize,
        pieces: u64,
        counts: &mut [u8; LABELS],
        f: &mut impl FnMut(u64),
    ) {
        if cell == 16 {
            f(pieces);
            return;
        }

        for piece in 0..LABELS {
            if counts[piece] < TALLY[piece] {
                counts[piece] += 1;
                for_each_state(cell + 1, pieces | ((piece as u64) << (4 * cell)), counts, f);
                counts[piece] -= 1;
            }
        }
    }

    #[test]
    fn test_indexing_table() {
        let table = IndexingTable::new();
        let mut index = 0;

        for_each_state(0, 0, &mut [0; LABELS], &mut |puzzle| {
            let encoded = table.encode(puzzle);
            assert_eq!(
                encoded, index,
                "puzzle {puzzle:x} encoded {encoded} index {index}"
            );
            index += 1;
        });

        assert_eq!(index as usize, SIZE);
    }
}
