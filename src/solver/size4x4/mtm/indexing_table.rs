use crate::solver::{
    indexing,
    size4x4::mtm::consts::{SIZE, TALLY},
};

/// The number of distinct labels, including the gap's.
const LABELS: usize = TALLY.len();

/// The number of values of a quarter: 4 cells read as base-8 digits by [`base_8`].
const QUARTER: usize = 1 << 12;

/// The place value of each label in an id of a multiset of pieces, which has a base-`TALLY[i] + 1`
/// digit for each label `i`.
const PLACES: [usize; LABELS] = {
    let mut places = [0; LABELS];
    let mut place = 1;
    let mut i = 0;
    while i < LABELS {
        places[i] = place;
        place *= TALLY[i] as usize + 1;
        i += 1;
    }
    places
};

/// Returns the number of pieces labeled `i` in the multiset with id `id`.
const fn count(id: usize, i: usize) -> usize {
    (id / PLACES[i]) % (TALLY[i] as usize + 1)
}

/// The number of multiset ids.
const IDS: usize = PLACES[LABELS - 1] * (TALLY[LABELS - 1] as usize + 1);

/// Indexes states by their rank in lexicographic order, with cell 0 most significant.
///
/// The cells are read as four quarters of 4 cells. The rank of a state is the number of states
/// with a smaller prefix (cells 0..8), plus the rank of its suffix (cells 8..16) among the
/// arrangements of the suffix's pieces. Each of these is split into one table lookup per quarter,
/// where the second lookup of each half depends on the pieces of one quarter, so every table is
/// small.
pub(super) struct IndexingTable {
    /// For each first quarter `a` (cells 0..4), the number of states whose first quarter is less
    /// than `a`, in the low 32 bits, and the row of [`Self::second`] for the pieces of `a`, in
    /// the high 32 bits.
    first: Box<[u64; QUARTER]>,
    /// For each multiset of 4 pieces, a row with an entry for each second quarter `b` (cells
    /// 4..8): the number of states whose first quarter uses those pieces and whose second quarter
    /// is less than `b`.
    second: Box<[u32]>,
    /// For each multiset of 4 pieces, a row with an entry for each third quarter `c` (cells
    /// 8..12): the number of arrangements of `c`'s pieces and those pieces, whose first 4 cells
    /// are less than `c`.
    third: Box<[u16]>,
    /// For each fourth quarter `d` (cells 12..16), the rank of `d` among the arrangements of its
    /// pieces, in the low 8 bits, and the row of [`Self::third`] for the pieces of `d`, above
    /// them.
    fourth: Box<[u32; QUARTER]>,
}

impl IndexingTable {
    pub(super) fn new() -> Self {
        // The number of arrangements of each multiset.
        let arrangements: Vec<u32> = (0..IDS)
            .map(|id| {
                let counts: [u8; LABELS] = std::array::from_fn(|i| count(id, i) as u8);
                indexing::multinomial(&counts) as u32
            })
            .collect();

        // Returns the number of arrangements of the multiset with id `id` whose first 4 cells are
        // less than the quarter `q`. The pieces of `q` must be in the multiset.
        let rank = |mut id: usize, q: usize| {
            let mut rank = 0;
            for i in 0..4 {
                let digit = (q >> (3 * (3 - i))) & 7;
                for d in 0..digit {
                    if count(id, d) != 0 {
                        rank += arrangements[id - PLACES[d]];
                    }
                }
                id -= PLACES[digit];
            }
            rank
        };

        // The quarters that use at most `TALLY[i]` pieces labeled `i`, with the ids of their
        // pieces.
        let quarters: Vec<(usize, usize)> = (0..QUARTER)
            .filter_map(|q| {
                let mut counts = [0; LABELS];
                for i in 0..4 {
                    let digit = (q >> (3 * i)) & 7;
                    if digit >= LABELS {
                        return None;
                    }
                    counts[digit] += 1;
                }
                (0..LABELS).all(|i| counts[i] <= TALLY[i]).then(|| {
                    let id = (0..LABELS)
                        .map(|i| usize::from(counts[i]) * PLACES[i])
                        .sum();
                    (q, id)
                })
            })
            .collect();

        // The multisets of 4 pieces, and the row of each.
        let mut multisets = Vec::new();
        let mut rows = vec![usize::MAX; IDS];
        for &(_, id) in &quarters {
            if rows[id] == usize::MAX {
                rows[id] = multisets.len() * QUARTER;
                multisets.push(id);
            }
        }

        // Returns whether every piece of the multiset `a` is in the multiset `b`.
        let contains = |b: usize, a: usize| (0..LABELS).all(|i| count(a, i) <= count(b, i));
        let all = IDS - 1;

        let mut first: Box<[u64; QUARTER]> = vec![0; QUARTER].try_into().unwrap();
        let mut second = vec![0; multisets.len() * QUARTER];
        let mut third = vec![0; multisets.len() * QUARTER];
        let mut fourth: Box<[u32; QUARTER]> = vec![0; QUARTER].try_into().unwrap();

        for &(q, id) in &quarters {
            first[q] = u64::from(rank(all, q)) | ((rows[id] as u64) << 32);
            fourth[q] = rank(id, q) | ((rows[id] as u32) << 8);
        }

        for &used in &multisets {
            let row = rows[used];

            // The other quarters can only use the pieces left over from `used`.
            for &(q, id) in quarters.iter().filter(|&&(_, id)| contains(all - used, id)) {
                // A first quarter using `used` leaves `all - used` for the rest of the state.
                second[row + q] = rank(all - used, q);
                // A third quarter `q` before a fourth quarter using `used`.
                third[row + q] = rank(id + used, q) as u16;
            }
        }

        Self {
            first,
            second: second.into_boxed_slice(),
            third: third.into_boxed_slice(),
            fourth,
        }
    }

    /// Returns the number of states with a smaller prefix than the 8 base-8 digits `digits`.
    #[inline(always)]
    fn prefix_rank(&self, digits: u32) -> u32 {
        let first = self.first[(digits >> 12) as usize & (QUARTER - 1)];
        let row = (first >> 32) as usize;

        // SAFETY: `row` is the start of a row of `self.second`, or 0, and each row has `QUARTER`
        // entries.
        first as u32
            + unsafe {
                *self
                    .second
                    .get_unchecked(row + (digits as usize & (QUARTER - 1)))
            }
    }

    /// Returns the rank of a suffix with the 8 base-8 digits `digits` among the arrangements of
    /// its pieces.
    #[inline(always)]
    fn suffix_rank(&self, digits: u32) -> u32 {
        let fourth = self.fourth[digits as usize & (QUARTER - 1)];
        let row = (fourth >> 8) as usize;

        // SAFETY: `row` is the start of a row of `self.third`, or 0, and each row has `QUARTER`
        // entries.
        (fourth & 0xFF)
            + u32::from(unsafe {
                *self
                    .third
                    .get_unchecked(row + ((digits >> 12) as usize & (QUARTER - 1)))
            })
    }

    /// Returns the index of the first state whose cells 0..8 are `prefix`. Every state with these
    /// cells has an index in a run starting here.
    pub(super) fn encode_prefix(&self, prefix: u32) -> u32 {
        self.prefix_rank(base_8(u64::from(prefix)) as u32)
    }

    /// Returns the offset of a state whose cells 8..16 are `suffix` within the run of indices
    /// given by [`Self::encode_prefix`].
    pub(super) fn encode_suffix(&self, suffix: u32) -> u32 {
        self.suffix_rank(base_8(u64::from(suffix)) as u32)
    }

    #[inline(always)]
    pub(super) fn encode(&self, puzzle: u64) -> u32 {
        let digits = base_8(puzzle);
        self.prefix_rank(digits as u32) + self.suffix_rank((digits >> 32) as u32)
    }
}

const _: () = assert!(SIZE <= u32::MAX as usize);

/// Reads cells 0..8 and cells 8..16 of `pieces` as two 8 digit base-8 numbers, with the lower
/// cells more significant, and returns them in the low and high 32 bits. Only the lowest 3 bits of
/// each cell are read, so each half is less than `1 << 24`, and its high and low 12 bits are its
/// first and second quarters.
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
            assert_eq!(
                table.encode_prefix(puzzle as u32) + table.encode_suffix((puzzle >> 32) as u32),
                index
            );
            index += 1;
        });

        assert_eq!(index as usize, SIZE);
    }
}
