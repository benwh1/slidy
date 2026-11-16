//! Defines the [`Pdb`] struct, which is a pattern database used to accelerate [`Solver`].
//!
//! [`Solver`]: crate::solver::size4x4::mtm::solver::Solver

use xxhash_rust::xxh3;

use crate::{
    algorithm::direction::Direction,
    solver::{
        config::PdbConfig,
        size4x4::mtm::{
            base_5_table::Base5Table,
            consts::{SIZE, TALLY},
            indexing_table::IndexingTable,
            puzzle::ReducedFourBitPuzzle,
        },
        statistics::PdbIterationStats,
    },
};

const HASH: u64 = 0x73b712151249d829;

const CHUNK: usize = 64;

/// Returns a mask with bit `k` set if `chunk[k] == depth`.
#[inline(always)]
fn depth_mask(chunk: &[u8; CHUNK], depth: u8) -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::{
            __m128i, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8,
        };

        // SAFETY: `sse2` is enabled on all x86_64 targets, and each load reads 16 bytes within
        // `chunk`, which has length 64.
        unsafe {
            let target = _mm_set1_epi8(depth as i8);
            let lane = |i: usize| {
                let v = _mm_loadu_si128(chunk.as_ptr().add(16 * i).cast::<__m128i>());
                u64::from(_mm_movemask_epi8(_mm_cmpeq_epi8(v, target)) as u16)
            };
            lane(0) | (lane(1) << 16) | (lane(2) << 32) | (lane(3) << 48)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let mut mask = 0;
        for (k, &x) in chunk.iter().enumerate() {
            mask |= u64::from(x == depth) << k;
        }
        mask
    }
}

/// Sets the entry at `idx` to `depth + 1` if it is unvisited, and returns whether it was.
#[inline(always)]
fn claim(pdb: &mut [u8], idx: usize, depth: u8) -> u64 {
    // Every visited entry is at most `depth + 1`, so this only changes unvisited ones.
    let entry = pdb[idx];
    pdb[idx] = entry.min(depth + 1);
    u64::from(entry == u8::MAX)
}

/// Returns whether `gap` is in cells 8..16.
#[inline(always)]
fn in_low_half(gap: u8) -> bool {
    gap >= 8
}

/// Returns the four states reached by moves that keep the gap in the same half of the puzzle
/// (cells 0..8 or 8..16): the three moves along its row, and the one move to the other row of
/// the half.
fn same_half_moves(pieces: u64) -> [u64; 4] {
    let state = ReducedFourBitPuzzle {
        pieces,
        gap: gap_position(pieces),
    };
    let mut moves = [0; 4];
    let mut n = 0;

    for mv in [Direction::Left, Direction::Right] {
        let mut puzzle = state;
        while puzzle.do_move(mv) {
            moves[n] = puzzle.pieces();
            n += 1;
        }
    }

    // `Up` moves the gap down a row, and `Down` moves it up.
    let mut puzzle = state;
    let mv = if (state.gap >> 2) & 1 == 0 {
        Direction::Up
    } else {
        Direction::Down
    };
    puzzle.do_move(mv);
    moves[n] = puzzle.pieces();
    n += 1;

    debug_assert_eq!(n, 4);
    moves
}

/// Calls `f` with the two states reached by moves that take the gap to the other half of the
/// puzzle.
#[inline(always)]
fn for_each_cross_move(pieces: u64, mut f: impl FnMut(u64)) {
    let gap = gap_position(pieces);
    let low = in_low_half(gap);

    // `Up` moves the gap down a row, and `Down` moves it up.
    let mv = if low { Direction::Down } else { Direction::Up };
    let mut puzzle = ReducedFourBitPuzzle { pieces, gap };

    while puzzle.do_move(mv) {
        if in_low_half(puzzle.gap) != low {
            f(puzzle.pieces());
        }
    }
}

/// Returns the position of the gap (the only zero nibble) in `pieces`.
fn gap_position(pieces: u64) -> u8 {
    const ONES: u64 = 0x1111_1111_1111_1111;
    const HIGHS: u64 = 0x8888_8888_8888_8888;

    // The lowest flagged nibble is exactly the lowest zero nibble.
    let zeros = pieces.wrapping_sub(ONES) & !pieces & HIGHS;
    (zeros.trailing_zeros() / 4) as u8
}

/// Calls `f` with every arrangement of `len` cells using at most `counts[i]` pieces labelled `i`,
/// in lexicographic order (cell 0 most significant). Cell `k` is stored in bits `4k..4k + 4`,
/// and `f` also receives the counts left over.
fn for_each_arrangement(len: usize, counts: [u8; 5], f: &mut impl FnMut(u32, [u8; 5])) {
    fn go(
        cell: usize,
        len: usize,
        nibbles: u32,
        counts: &mut [u8; 5],
        f: &mut impl FnMut(u32, [u8; 5]),
    ) {
        if cell == len {
            f(nibbles, *counts);
            return;
        }

        for piece in 0..5 {
            if counts[piece] == 0 {
                continue;
            }

            counts[piece] -= 1;
            go(
                cell + 1,
                len,
                nibbles | ((piece as u32) << (4 * cell)),
                counts,
                f,
            );
            counts[piece] += 1;
        }
    }

    let mut counts = counts;
    go(0, len, 0, &mut counts, f);
}

/// A run of consecutive indices that share the pieces in cells 0..8.
struct Block {
    /// Index of the first state in the block.
    start: u32,
    /// Pieces in cells 0..8.
    prefix: u32,
    /// Index into [`Blocks::suffixes`] of the possible pieces in cells 8..16.
    suffixes: u16,
    /// Whether the gap is in cells 0..8.
    gap_in_prefix: bool,
    /// If the gap is in cells 0..8, the starts of the blocks reached by [`same_half_moves`]. These
    /// moves leave cells 8..16 alone, so the state at offset `j` in this block moves to offset
    /// `j` in each of them.
    children: [u32; 4],
}

/// The possible pieces in cells 8..16 for a block, given the pieces left over from cells 0..8.
struct Suffixes {
    /// The pieces in cells 8..16, in index order.
    suffixes: Vec<u32>,
    /// If the gap is in cells 8..16, the offsets within the block reached from each suffix by
    /// [`same_half_moves`]. These moves leave cells 0..8 alone, so they stay in the block.
    moves: Vec<[u16; 4]>,
}

/// Every state's index split into blocks, so that the table can be walked in index order and the
/// state at each index read off directly.
struct Blocks {
    blocks: Vec<Block>,
    suffixes: Vec<Suffixes>,
}

impl Blocks {
    fn new(indexing_table: &IndexingTable, base_5_table: &Base5Table) -> Self {
        let encode = |pieces| indexing_table.encode(pieces, base_5_table);

        let mut blocks = Vec::new();
        let mut suffixes: Vec<Suffixes> = Vec::new();
        let mut suffix_ids = std::collections::HashMap::new();
        let mut start = 0;

        for_each_arrangement(8, TALLY, &mut |prefix, remaining| {
            let id = *suffix_ids.entry(remaining).or_insert_with(|| {
                let mut list = Vec::new();
                for_each_arrangement(8, remaining, &mut |suffix, _| list.push(suffix));

                // The offsets don't depend on the prefix, so compute them with this block's.
                let moves = if remaining[0] == 1 {
                    list.iter()
                        .map(|&suffix| {
                            let pieces = u64::from(prefix) | (u64::from(suffix) << 32);
                            same_half_moves(pieces).map(|p| (encode(p) - start) as u16)
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                suffixes.push(Suffixes {
                    suffixes: list,
                    moves,
                });
                suffixes.len() - 1
            });

            let first = suffixes[id].suffixes[0];
            let gap_in_prefix = remaining[0] == 0;
            let children = if gap_in_prefix {
                let pieces = u64::from(prefix) | (u64::from(first) << 32);
                same_half_moves(pieces).map(encode)
            } else {
                [0; 4]
            };

            blocks.push(Block {
                start,
                prefix,
                suffixes: id as u16,
                gap_in_prefix,
                children,
            });
            start += suffixes[id].suffixes.len() as u32;
        });

        debug_assert_eq!(start as usize, SIZE);

        Self { blocks, suffixes }
    }
}

/// Pattern database used by [`Solver`].
///
/// [`Solver`]: crate::solver::size4x4::mtm::solver::Solver
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pdb {
    pdb: Box<[u8]>,
}

impl Pdb {
    pub(super) fn new(
        indexing_table: &IndexingTable,
        base_5_table: &Base5Table,
        config: &PdbConfig,
    ) -> Self {
        let blocks = Blocks::new(indexing_table, base_5_table);

        let mut pdb = vec![u8::MAX; SIZE];

        let puzzle = ReducedFourBitPuzzle::new();
        let solved_index = indexing_table.encode(puzzle.pieces, base_5_table) as usize;
        pdb[solved_index] = 0;

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while new != 0 {
            new = 0;

            for block in &blocks.blocks {
                let suffixes = &blocks.suffixes[block.suffixes as usize];
                let start = block.start as usize;

                let len = suffixes.suffixes.len();
                let mut j = 0;

                while j < len {
                    let n = (len - j).min(CHUNK);
                    let mut mask = if n == CHUNK {
                        let chunk = pdb[start + j..start + j + CHUNK].try_into().unwrap();
                        depth_mask(chunk, depth)
                    } else {
                        pdb[start + j..start + j + n]
                            .iter()
                            .enumerate()
                            .fold(0, |m, (k, &x)| m | (u64::from(x == depth) << k))
                    };

                    while mask != 0 {
                        let offset = j + mask.trailing_zeros() as usize;
                        mask &= mask - 1;

                        if block.gap_in_prefix {
                            for child in block.children {
                                new += claim(&mut pdb, child as usize + offset, depth);
                            }
                        } else {
                            for child in suffixes.moves[offset] {
                                new += claim(&mut pdb, start + child as usize, depth);
                            }
                        }

                        let suffix = suffixes.suffixes[offset];
                        let pieces = u64::from(block.prefix) | (u64::from(suffix) << 32);
                        for_each_cross_move(pieces, |p| {
                            let idx = indexing_table.encode(p, base_5_table) as usize;
                            new += claim(&mut pdb, idx, depth);
                        });
                    }

                    j += CHUNK;
                }
            }

            total += new;
            depth += 1;

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        let pdb = pdb.into_boxed_slice();

        Self { pdb }
    }

    /// Initializes a [`Pdb`] from a boxed byte slice containing the pre-computed data.
    ///
    /// The length of the data is checked, and the [`xxh3`] hash is computed and checked against a
    /// known value to verify integrity.
    ///
    /// # Safety
    ///
    /// Despite the correctness checks described above, this function is unsafe because it is
    /// still technically possible for `bytes` to contain incorrect data in the event of a hash
    /// collision.
    ///
    /// If the data is incorrect, then using the resulting [`Pdb`] in [`Solver`] can cause undefined
    /// behavior.
    ///
    /// [`Solver`]: crate::solver::size4x4::mtm::solver::Solver
    #[must_use]
    pub unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.len() != SIZE || xxh3::xxh3_64(&bytes) != HASH {
            return None;
        }

        // SAFETY: We checked above that the data is (almost certainly) correct.
        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    /// Initializes the [`Pdb`] with `bytes`.
    ///
    /// # Safety
    ///
    /// The caller is responsible for the correctness of the data contained in `bytes`. No
    /// correctness checks are performed.
    ///
    /// If incorrect data is used, then use of the [`Pdb`] in [`Solver`] could lead to incorrect
    /// results or undefined behavior.
    ///
    /// [`Solver`]: crate::solver::size4x4::mtm::solver::Solver
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        Self { pdb: bytes }
    }

    /// Returns the entry for the state at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    /// Returns the entry for the state at `index`, without bounds checking.
    ///
    /// # Safety
    ///
    /// `index` must be within bounds.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
        // SAFETY: The caller guarantees that `index` is in bounds.
        unsafe { *self.pdb.get_unchecked(index) }
    }
}

impl AsRef<[u8]> for Pdb {
    fn as_ref(&self) -> &[u8] {
        &self.pdb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb() {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();
        let pdb = Pdb::new(&indexing_table, &base_5_table, &PdbConfig::default());
        let hash = xxh3::xxh3_64(pdb.as_ref());
        assert_eq!(hash, HASH);
    }
}
