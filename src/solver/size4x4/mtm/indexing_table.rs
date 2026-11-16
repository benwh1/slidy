use std::array;

use crate::solver::{
    indexing,
    size4x4::mtm::consts::{SIZE, TALLY},
};

pub(super) struct IndexingTable {
    high: Box<[u32]>,
    low: Box<[u16]>,
}

impl IndexingTable {
    pub(super) fn new() -> Self {
        /// Calls `f` with the base-5 digits and the piece counts of every arrangement of 8 cells
        /// with at most `TALLY[i]` pieces labelled `i`, in lexicographic order.
        fn go(
            cell: usize,
            digits: usize,
            counts: &mut [u8; 5],
            f: &mut impl FnMut(usize, &[u8; 5]),
        ) {
            if cell == 8 {
                f(digits, counts);
                return;
            }

            for piece in 0..5 {
                if counts[piece] < TALLY[piece] {
                    counts[piece] += 1;
                    go(cell + 1, 5 * digits + piece, counts, f);
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
        let mut ranks = [0; 2 * 3 * 5 * 6 * 5];

        go(0, 0, &mut [0; 5], &mut |digits, counts| {
            high[digits] = start;
            // The states with this prefix have every arrangement of the remaining pieces as their
            // suffix.
            start +=
                indexing::multinomial(&array::from_fn::<_, 5, _>(|i| TALLY[i] - counts[i])) as u32;

            let id = (0..5).rev().fold(0, |id, i| {
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
        let high = base_5(u64::from(prefix)) as u32 as usize;

        // SAFETY: `base_5` returns halves less than `LEN`, the length of `self.high`.
        unsafe { *self.high.get_unchecked(high) }
    }

    /// Returns the offset of a state whose cells 8..16 are `suffix` within the run of indices
    /// given by [`Self::encode_prefix`].
    pub(super) fn encode_suffix(&self, suffix: u32) -> u32 {
        let low = base_5(u64::from(suffix)) as u32 as usize;

        // SAFETY: `base_5` returns halves less than `LEN`, the length of `self.low`.
        u32::from(unsafe { *self.low.get_unchecked(low) })
    }

    #[inline(always)]
    pub(super) fn encode(&self, puzzle: u64) -> u32 {
        let digits = base_5(puzzle);
        let high = digits as u32 as usize;
        let low = (digits >> 32) as usize;

        // SAFETY: `base_5` returns halves less than `LEN`, the length of `self.high` and
        // `self.low`.
        unsafe { *self.high.get_unchecked(high) + u32::from(*self.low.get_unchecked(low)) }
    }
}

/// The length of the tables: one more than the largest value of either half of [`base_5`], which
/// is reached when every cell is 7.
const LEN: usize = 7 * (5 * 5 * 5 * 5 * 5 * 5 * 5 * 5 - 1) / 4 + 1;

/// Reads cells 0..8 and cells 8..16 of `pieces` as two 8 digit base-5 numbers, with the lower
/// cells more significant, and returns them in the low and high 32 bits. Only the lowest 3 bits of
/// each cell are read, so each half is less than [`LEN`] even if a cell is more than 4.
#[inline(always)]
fn base_5(pieces: u64) -> u64 {
    const CELLS: u64 = 0x0707_0707_0707_0707;
    const PAIRS: u64 = 0x00FF_00FF_00FF_00FF;
    const QUADS: u64 = 0x0000_FFFF_0000_FFFF;

    // Each step combines neighbouring fields, and no field overflows into the next: a pair is at
    // most 7 * 5 + 7, a quad at most 42 * 25 + 42, and a half at most 1092 * 625 + 1092.
    let pairs = (pieces & CELLS) * 5 + ((pieces >> 4) & CELLS);
    let quads = (pairs & PAIRS) * 25 + ((pairs >> 8) & PAIRS);
    (quads & QUADS) * 625 + ((quads >> 16) & QUADS)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::solver::size4x4::mtm::consts::SIZE;

    #[test]
    fn test_indexing_table() {
        let table = IndexingTable::new();

        let max_counts = TALLY;
        let mut counts = [0, 0, 0, 0, 0];

        let mut index = 0;

        for p0 in 0..5 {
            counts[p0] += 1;
            for p1 in 0..5 {
                if counts[p1] >= max_counts[p1] {
                    continue;
                }
                counts[p1] += 1;
                for p2 in 0..5 {
                    if counts[p2] >= max_counts[p2] {
                        continue;
                    }
                    counts[p2] += 1;
                    for p3 in 0..5 {
                        if counts[p3] >= max_counts[p3] {
                            continue;
                        }
                        counts[p3] += 1;
                        for p4 in 0..5 {
                            if counts[p4] >= max_counts[p4] {
                                continue;
                            }
                            counts[p4] += 1;
                            for p5 in 0..5 {
                                if counts[p5] >= max_counts[p5] {
                                    continue;
                                }
                                counts[p5] += 1;
                                for p6 in 0..5 {
                                    if counts[p6] >= max_counts[p6] {
                                        continue;
                                    }
                                    counts[p6] += 1;
                                    for p7 in 0..5 {
                                        if counts[p7] >= max_counts[p7] {
                                            continue;
                                        }
                                        counts[p7] += 1;

                                        for p8 in 0..5 {
                                            if counts[p8] >= max_counts[p8] {
                                                continue;
                                            }
                                            counts[p8] += 1;

                                            for p9 in 0..5 {
                                                if counts[p9] >= max_counts[p9] {
                                                    continue;
                                                }
                                                counts[p9] += 1;

                                                for p10 in 0..5 {
                                                    if counts[p10] >= max_counts[p10] {
                                                        continue;
                                                    }
                                                    counts[p10] += 1;

                                                    for p11 in 0..5 {
                                                        if counts[p11] >= max_counts[p11] {
                                                            continue;
                                                        }
                                                        counts[p11] += 1;

                                                        for p12 in 0..5 {
                                                            if counts[p12] >= max_counts[p12] {
                                                                continue;
                                                            }
                                                            counts[p12] += 1;

                                                            for p13 in 0..5 {
                                                                if counts[p13] >= max_counts[p13] {
                                                                    continue;
                                                                }
                                                                counts[p13] += 1;

                                                                for p14 in 0..5 {
                                                                    if counts[p14]
                                                                        >= max_counts[p14]
                                                                    {
                                                                        continue;
                                                                    }
                                                                    counts[p14] += 1;

                                                                    for p15 in 0..5 {
                                                                        if counts[p15]
                                                                            >= max_counts[p15]
                                                                        {
                                                                            continue;
                                                                        }
                                                                        counts[p15] += 1;

                                                                        let puzzle = ((p15 as u64)
                                                                            << 60)
                                                                            | ((p14 as u64) << 56)
                                                                            | ((p13 as u64) << 52)
                                                                            | ((p12 as u64) << 48)
                                                                            | ((p11 as u64) << 44)
                                                                            | ((p10 as u64) << 40)
                                                                            | ((p9 as u64) << 36)
                                                                            | ((p8 as u64) << 32)
                                                                            | ((p7 as u64) << 28)
                                                                            | ((p6 as u64) << 24)
                                                                            | ((p5 as u64) << 20)
                                                                            | ((p4 as u64) << 16)
                                                                            | ((p3 as u64) << 12)
                                                                            | ((p2 as u64) << 8)
                                                                            | ((p1 as u64) << 4)
                                                                            | (p0 as u64);

                                                                        let encoded =
                                                                            table.encode(puzzle);

                                                                        assert_eq!(
                                                                            encoded, index,
                                                                            "puzzle {puzzle:x} encoded {encoded} index {index}"
                                                                        );

                                                                        index += 1;

                                                                        counts[p15] -= 1;
                                                                    }
                                                                    counts[p14] -= 1;
                                                                }
                                                                counts[p13] -= 1;
                                                            }
                                                            counts[p12] -= 1;
                                                        }
                                                        counts[p11] -= 1;
                                                    }
                                                    counts[p10] -= 1;
                                                }
                                                counts[p9] -= 1;
                                            }
                                            counts[p8] -= 1;
                                        }
                                        counts[p7] -= 1;
                                    }
                                    counts[p6] -= 1;
                                }
                                counts[p5] -= 1;
                            }
                            counts[p4] -= 1;
                        }
                        counts[p3] -= 1;
                    }
                    counts[p2] -= 1;
                }
                counts[p1] -= 1;
            }
            counts[p0] -= 1;
        }

        assert_eq!(index as usize, SIZE);
    }
}
