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

/// Marks every unvisited neighbour of the state `pieces` with `depth + 1` and returns how many
/// there were.
#[inline(always)]
fn expand(
    pdb: &mut [u8],
    pieces: u64,
    depth: u8,
    indexing_table: &IndexingTable,
    base_5_table: &Base5Table,
) -> u64 {
    let state = ReducedFourBitPuzzle {
        pieces,
        gap: gap_position(pieces),
    };
    let mut new = 0;

    for mv in [
        Direction::Up,
        Direction::Left,
        Direction::Down,
        Direction::Right,
    ] {
        let mut puzzle = state;

        while puzzle.do_move(mv) {
            let idx = indexing_table.encode(puzzle.pieces(), base_5_table) as usize;
            if pdb[idx] == u8::MAX {
                pdb[idx] = depth + 1;
                new += 1;
            }
        }
    }

    new
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
    /// Index into [`Blocks::suffixes`] of the possible pieces in cells 8..16, in index order.
    suffixes: u16,
}

/// Every state's index split into blocks, so that the table can be walked in index order and the
/// state at each index read off directly.
struct Blocks {
    blocks: Vec<Block>,
    suffixes: Vec<Vec<u32>>,
}

impl Blocks {
    fn new() -> Self {
        let mut blocks = Vec::new();
        let mut suffixes: Vec<Vec<u32>> = Vec::new();
        let mut suffix_ids = std::collections::HashMap::new();
        let mut start = 0;

        for_each_arrangement(8, TALLY, &mut |prefix, remaining| {
            let id = *suffix_ids.entry(remaining).or_insert_with(|| {
                let mut list = Vec::new();
                for_each_arrangement(8, remaining, &mut |suffix, _| list.push(suffix));
                suffixes.push(list);
                suffixes.len() - 1
            });

            blocks.push(Block {
                start,
                prefix,
                suffixes: id as u16,
            });
            start += suffixes[id].len() as u32;
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
        let blocks = Blocks::new();

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

                let len = suffixes.len();
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
                        let k = mask.trailing_zeros() as usize;
                        mask &= mask - 1;

                        let suffix = suffixes[j + k];
                        let pieces = u64::from(block.prefix) | (u64::from(suffix) << 32);
                        new += expand(&mut pdb, pieces, depth, indexing_table, base_5_table);
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
