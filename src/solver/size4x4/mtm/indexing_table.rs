use crate::solver::size4x4::mtm::{base_5_table::Base5Table, consts::TALLY, indexing};

pub(super) struct IndexingTable {
    high: Box<[u32]>,
    low: Box<[u16]>,
}

impl IndexingTable {
    pub(super) fn new() -> Self {
        let max_counts = TALLY;
        let mut counts = [0, 0, 0, 0, 0];

        let mut high = vec![0; 5 * 5 * 5 * 5 * 5 * 5 * 5 * 5];
        let mut low = vec![0; 5 * 5 * 5 * 5 * 5 * 5 * 5 * 5];

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

                                        let index = p7
                                            + 5 * (p6
                                                + 5 * (p5
                                                    + 5 * (p4
                                                        + 5 * (p3
                                                            + 5 * (p2 + 5 * (p1 + 5 * p0))))));

                                        let mut pieces = [
                                            p0 as u8, p1 as u8, p2 as u8, p3 as u8, p4 as u8,
                                            p5 as u8, p6 as u8, p7 as u8, 0, 0, 0, 0, 0, 0, 0, 0,
                                        ];

                                        let mut piece_index = 8;
                                        for i in 0..5 {
                                            for _ in counts[i]..max_counts[i] {
                                                pieces[piece_index] = i as u8;
                                                piece_index += 1;
                                            }
                                        }
                                        high[index] =
                                            indexing::encode_multiset(pieces, max_counts) as u32;

                                        let pieces = [
                                            p0 as u8, p1 as u8, p2 as u8, p3 as u8, p4 as u8,
                                            p5 as u8, p6 as u8, p7 as u8,
                                        ];
                                        low[index] =
                                            indexing::encode_multiset(pieces, counts) as u16;

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

        Self {
            high: high.into_boxed_slice(),
            low: low.into_boxed_slice(),
        }
    }

    #[inline(always)]
    pub(super) fn encode(&self, puzzle: u64, base_5_table: &Base5Table) -> u32 {
        let t = |shift: u8| {
            // SAFETY: `base_5_table` has length 0xFFFF.
            unsafe { base_5_table.get_unchecked(((puzzle >> shift) & 0xFFFF) as usize) as usize }
        };

        let high = t(16) + 625 * t(0);
        let low = t(48) + 625 * t(32);

        // SAFETY: `high` and `low` are computed as 8 digit base-5 numbers. `self.high` and
        // `self.low` have length 5^8, so these are within bounds.
        unsafe { *self.high.get_unchecked(high) + *self.low.get_unchecked(low) as u32 }
    }
}

#[cfg(test)]
mod tests {

    use crate::solver::size4x4::mtm::consts::SIZE;

    use super::*;

    #[test]
    fn test_indexing_table() {
        let table = IndexingTable::new();
        let base_5_table = Base5Table::new();

        let max_counts = TALLY;
        let mut counts = [0u8, 0, 0, 0, 0];

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

                                                                        let encoded = table.encode(
                                                                            puzzle,
                                                                            &base_5_table,
                                                                        );

                                                                        assert_eq!(
                                                                            encoded,
                                                                            index,
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
