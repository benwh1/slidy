//! Defines the [`Pdb`] struct, which is a pattern database used to accelerate [`Solver`].
//!
//! [`Solver`]: crate::solver::size4x4::mtm::solver::Solver

use xxhash_rust::xxh3;

use crate::{
    algorithm::direction::Direction,
    puzzle::small::find_nibble,
    solver::{
        config::PdbConfig,
        size4x4::mtm::{
            consts::{SIZE, TALLY},
            indexing_table::IndexingTable,
            puzzle::ReducedFourBitPuzzle,
        },
        statistics::PdbIterationStats,
    },
};

// Tuning constants. These only affect how fast the table is built, not its contents.

/// Layers are expanded from a queue while they have fewer states than this. `1 << 17` to
/// `1 << 19` build in the same time, and `1 << 20` is ~1.5% slower.
const QUEUE_LIMIT: usize = 1 << 18;

/// Pull passes list the states they don't reach once at most this many states are unvisited, so
/// that the next pass checks only those instead of scanning the table. `1 << 24` builds in the
/// same time, using more memory.
const LIST_LIMIT: u64 = 1 << 22;

const HASH: u64 = 0x29c23b6ab750f72c;

/// The number of table entries compared at once when scanning for a layer. This is the width of
/// the mask returned by [`depth_mask`].
const CHUNK: usize = 64;

/// Extra entries at the end of the table while it is being built. Every child's index is a start
/// below [`SIZE`] plus an offset of [`OFFSET_BITS`] bits, so with this padding it is always in
/// bounds.
const PADDING: usize = 1 << 16;

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
        const ONES: u64 = 0x0101_0101_0101_0101;
        const LOWS: u64 = 0x7F7F_7F7F_7F7F_7F7F;
        // Bit `56 + k` of the product is bit `8k` of the multiplicand, and no other bits carry
        // into the top byte.
        const GATHER: u64 = 0x0102_0408_1020_4080;

        let target = ONES * u64::from(depth);
        chunk
            .as_chunks::<8>()
            .0
            .iter()
            .enumerate()
            .fold(0, |mask, (i, bytes)| {
                let x = u64::from_le_bytes(*bytes) ^ target;
                // The high bit of each byte is set exactly when the byte is zero.
                let zeros = !(((x & LOWS) + LOWS) | x | LOWS);
                mask | (((zeros >> 7).wrapping_mul(GATHER) >> 56) << (8 * i))
            })
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

/// [`claim`] without bounds checking.
///
/// # Safety
///
/// `idx` must be less than `pdb.len()`.
#[inline(always)]
unsafe fn claim_unchecked(pdb: &mut [u8], idx: usize, depth: u8) -> u64 {
    // SAFETY: The caller guarantees that `idx` is in bounds.
    let entry = unsafe { pdb.get_unchecked_mut(idx) };
    let old = *entry;
    *entry = old.min(depth + 1);
    u64::from(old == u8::MAX)
}

/// Returns the piece in cell `cell` of `nibbles`.
fn get(nibbles: u32, cell: u32) -> u32 {
    (nibbles >> (4 * cell)) & 0xF
}

/// Returns `nibbles` with the piece in cell `cell` replaced by `piece`.
fn set(nibbles: u32, cell: u32, piece: u32) -> u32 {
    (nibbles & !(0xF << (4 * cell))) | (piece << (4 * cell))
}

/// Returns the position of the gap (the only zero nibble) in `pieces`.
fn gap_position(pieces: u64) -> u8 {
    find_nibble(pieces, 0) as u8
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

/// Calls `f` with every arrangement of `len` cells using at most `counts[i]` pieces labeled `i`,
/// in lexicographic order (cell 0 most significant). Cell `k` is stored in bits `4k..4k + 4`,
/// and `f` also receives the counts left over.
fn for_each_arrangement(len: usize, counts: [u8; LABELS], f: &mut impl FnMut(u32, [u8; LABELS])) {
    fn go(
        cell: usize,
        len: usize,
        nibbles: u32,
        counts: &mut [u8; LABELS],
        f: &mut impl FnMut(u32, [u8; LABELS]),
    ) {
        if cell == len {
            f(nibbles, *counts);
            return;
        }

        for piece in 0..LABELS {
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

/// A run of consecutive indices that share the pieces in cells 0..8 (the prefix). The states in
/// the run are ordered by the pieces in cells 8..16 (the suffix).
///
/// The index of a state is the start of its block plus the offset of its suffix, and each of
/// these depends on only one half of the puzzle. A move changes the prefix in a way that depends
/// on the prefix and at most one piece of the suffix, and vice versa, so a child's index is a
/// start looked up from the parent's [`Block`] plus an offset looked up from its suffix.
///
/// Write `c` for the gap's column.
///
/// If the gap is in the prefix:
/// - the four moves within the prefix keep the suffix, so the child at `children[k]` has the same
///   offset;
/// - the two moves into the suffix pull in the piece `p` from cell `8 + c`. Both children have the
///   start `cross[p]`, and offsets from [`Suffixes::prefix_gap`].
///
/// If the gap is in the suffix:
/// - the four moves within the suffix keep the prefix, so the children stay in this block, at
///   offsets from [`Suffixes::suffix_gap`];
/// - the two moves into the prefix (to rows 1 and 0) push out the piece `q` from cell `4 + c`. The
///   children have starts `cross[2 * c]` and `cross[2 * c + 1]`, and the same offset from
///   [`Suffixes::suffix_gap`].
struct Block {
    /// Index of the first state in the block.
    start: u32,
    /// Pieces in cells 0..8.
    prefix: u32,
    /// Index into [`Blocks::suffixes`] of the possible suffixes.
    suffixes: u16,
    /// Whether the gap is in the prefix.
    gap_in_prefix: bool,
    /// If the gap is in the prefix, its column.
    gap_col: u8,
    /// If the gap is in the prefix, the starts of the blocks reached by [`same_half_moves`].
    children: [u32; 4],
    /// The starts of the blocks reached by moves that take the gap to the other half.
    cross: [u32; 8],
}

/// The number of distinct labels, including the gap's.
const LABELS: usize = TALLY.len();

/// The number of bits of an offset within a block in [`Suffixes`]. A block has at most 8! = 40320
/// suffixes, when the suffix has the gap, labels 3 to 7, and one each of labels 1 and 2, so this
/// is enough.
const OFFSET_BITS: u32 = 16;

/// The low [`OFFSET_BITS`] bits.
const OFFSET_MASK: u64 = (1 << OFFSET_BITS) - 1;

/// The possible suffixes of a block, which depend only on the pieces left over from the prefix.
/// The tables are packed so that the build loop reads as little as possible for each state.
struct Suffixes {
    /// The number of suffixes.
    len: usize,
    /// If the gap is in the prefix: for each column `c` of the gap, and each suffix in index
    /// order, the piece `p` in cell `8 + c` and the offsets `e` and `f` reached by moving the gap
    /// to cell `8 + c` and on to `12 + c`, packed as `e | (f << OFFSET_BITS) | (p << 2 *
    /// OFFSET_BITS)`.
    prefix_gap: [Vec<u64>; 4],
    /// If the gap is in the suffix: for each suffix in index order, the offsets reached by
    /// [`same_half_moves`], then the offsets reached by moves that push out the piece `q` into the
    /// prefix, at entry `3 + q` for `q` in `1..LABELS`. The last entry is the gap's column.
    suffix_gap: Vec<[u16; 12]>,
}

/// Every state's index split into blocks, so that the table can be walked in index order and the
/// state at each index read off directly.
struct Blocks {
    blocks: Vec<Block>,
    suffixes: Vec<Suffixes>,
}

impl Blocks {
    fn new(indexing_table: &IndexingTable) -> Self {
        let encode_prefix = |prefix| indexing_table.encode_prefix(prefix);
        let encode_suffix = |suffix| indexing_table.encode_suffix(suffix) as u16;

        let mut blocks = Vec::new();
        let mut suffixes: Vec<Suffixes> = Vec::new();
        let mut suffix_ids = std::collections::HashMap::new();
        let mut start = 0;

        for_each_arrangement(8, TALLY, &mut |prefix, remaining| {
            let gap_in_prefix = remaining[0] == 0;

            let id = *suffix_ids.entry(remaining).or_insert_with(|| {
                let mut list = Vec::new();
                for_each_arrangement(8, remaining, &mut |suffix, _| list.push(suffix));

                assert!(list.len() <= 1 << OFFSET_BITS);

                let (prefix_gap, suffix_gap) = if gap_in_prefix {
                    let prefix_gap = std::array::from_fn(|c| {
                        let c = c as u32;
                        list.iter()
                            .map(|&s| {
                                // The gap moves to cell `8 + c`, or on to cell `12 + c`.
                                let e = u64::from(encode_suffix(set(s, c, 0)));
                                let f = u64::from(encode_suffix(set(
                                    set(s, c, get(s, c + 4)),
                                    c + 4,
                                    0,
                                )));
                                e | (f << OFFSET_BITS) | (u64::from(get(s, c)) << (2 * OFFSET_BITS))
                            })
                            .collect()
                    });
                    (prefix_gap, Vec::new())
                } else {
                    let suffix_gap = list
                        .iter()
                        .map(|&s| {
                            let pieces = u64::from(prefix) | (u64::from(s) << 32);
                            let moves =
                                same_half_moves(pieces).map(|p| encode_suffix((p >> 32) as u32));

                            // The suffix's gap is its lowest zero nibble.
                            let h = u32::from(gap_position(u64::from(s)));
                            let c = h % 4;

                            let mut entry = [0; 12];
                            entry[..4].copy_from_slice(&moves);
                            for q in 1..LABELS as u32 {
                                let child = if h < 4 {
                                    set(s, c, q)
                                } else {
                                    set(set(s, c + 4, get(s, c)), c, q)
                                };
                                entry[3 + q as usize] = encode_suffix(child);
                            }
                            entry[11] = c as u16;
                            entry
                        })
                        .collect();
                    (Default::default(), suffix_gap)
                };

                suffixes.push(Suffixes {
                    len: list.len(),
                    prefix_gap,
                    suffix_gap,
                });
                suffixes.len() - 1
            });

            let mut children = [0; 4];
            let mut cross = [0; 8];
            let mut gap_col = 0;

            if gap_in_prefix {
                let g = u32::from(gap_position(u64::from(prefix)));
                let c = g % 4;
                gap_col = c as u8;

                children = same_half_moves(u64::from(prefix)).map(|p| encode_prefix(p as u32));

                // The piece in cell `4 + c` moves up to the gap if it's in row 0, and `p` moves up
                // into cell `4 + c`.
                let shifted = if g < 4 {
                    set(prefix, c, get(prefix, c + 4))
                } else {
                    prefix
                };
                for p in 1..LABELS as u32 {
                    cross[p as usize] = encode_prefix(set(shifted, c + 4, p));
                }
            } else {
                for c in 0..4 {
                    // The gap moves to cell `4 + c`, or on to cell `c`.
                    cross[2 * c as usize] = encode_prefix(set(prefix, c + 4, 0));
                    cross[2 * c as usize + 1] =
                        encode_prefix(set(set(prefix, c + 4, get(prefix, c)), c, 0));
                }
            }

            blocks.push(Block {
                start,
                prefix,
                suffixes: id as u16,
                gap_in_prefix,
                gap_col,
                children,
                cross,
            });
            start += suffixes[id].len as u32;
        });

        assert_eq!(start as usize, SIZE);
        // The building loop relies on this to skip bounds checks; see `PADDING`.
        assert!(blocks.iter().all(|block| {
            block
                .children
                .iter()
                .chain(&block.cross)
                .all(|&start| (start as usize) < SIZE)
        }));

        Self { blocks, suffixes }
    }

    /// Returns the indices of the six neighbors of the state at `offset` in `block`. These are
    /// all less than `SIZE + PADDING`.
    ///
    /// # Safety
    ///
    /// `offset` must be less than the number of suffixes of `block`.
    #[inline(always)]
    unsafe fn neighbors(&self, block: &Block, offset: usize) -> [usize; 6] {
        let suffixes = &self.suffixes[block.suffixes as usize];

        // Every index below is a start below `SIZE` (checked in `Blocks::new`) plus an offset below
        // `1 << OFFSET_BITS`, which is `PADDING`, so it is within `SIZE + PADDING`.
        if block.gap_in_prefix {
            let [a, b, c, d] = block.children.map(|child| child as usize + offset);

            let col = usize::from(block.gap_col & 3);

            // SAFETY: The caller guarantees that `offset` is less than the number of suffixes,
            // which is the length of each table.
            let packed = unsafe { *suffixes.prefix_gap[col].get_unchecked(offset) };
            let child = block.cross[(packed >> (2 * OFFSET_BITS)) as usize & 7] as usize;
            let e = child + (packed & OFFSET_MASK) as usize;
            let f = child + ((packed >> OFFSET_BITS) & OFFSET_MASK) as usize;

            [a, b, c, d, e, f]
        } else {
            let start = block.start as usize;

            // SAFETY: as above.
            let entry = unsafe { suffixes.suffix_gap.get_unchecked(offset) };
            let [a, b, c, d] =
                [entry[0], entry[1], entry[2], entry[3]].map(|child| start + usize::from(child));

            let col = u32::from(entry[11] & 3);
            // `q` is at most 7, so the entry is in bounds.
            let q = get(block.prefix, col + 4) & 7;
            let child = usize::from(entry[q as usize + 3]);
            let e = block.cross[(2 * col) as usize] as usize + child;
            let f = block.cross[(2 * col + 1) as usize] as usize + child;

            [a, b, c, d, e, f]
        }
    }

    /// Sets the entry of the state at `offset` in block `b` to `depth + 1` if the state has a
    /// neighbor at `depth`, and otherwise adds it to `unreached`, if that is `Some`. Returns
    /// whether it has such a neighbor.
    ///
    /// # Safety
    ///
    /// `offset` must be less than the number of suffixes of block `b`, and `pdb` must have length
    /// `SIZE + PADDING`.
    #[inline(always)]
    unsafe fn pull(
        &self,
        pdb: &mut [u8],
        b: usize,
        offset: usize,
        depth: u8,
        unreached: &mut Option<Vec<(u32, u16)>>,
    ) -> u64 {
        let block = &self.blocks[b];

        // SAFETY: The caller guarantees that `offset` is in range.
        let neighbors = unsafe { self.neighbors(block, offset) };
        let found = neighbors.iter().any(|&idx| {
            // SAFETY: `neighbors` returns indices within `SIZE + PADDING`, which is the length of
            // `pdb`.
            unsafe { *pdb.get_unchecked(idx) == depth }
        });

        if found {
            pdb[block.start as usize + offset] = depth + 1;
        } else if let Some(unreached) = unreached {
            unreached.push((b as u32, offset as u16));
        }
        u64::from(found)
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
    pub(super) fn new(indexing_table: &IndexingTable, config: &PdbConfig) -> Self {
        let blocks = Blocks::new(indexing_table);

        let mut pdb = vec![u8::MAX; SIZE + PADDING];

        let puzzle = ReducedFourBitPuzzle::new();
        let solved_index = indexing_table.encode(puzzle.pieces) as usize;
        pdb[solved_index] = 0;

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
            f(PdbIterationStats {
                depth: depth.into(),
                new,
                total,
            });
        }

        // Scanning the whole table costs about as much as expanding a couple of hundred thousand
        // states from a queue, so expand the first few layers from a queue instead.
        let mut frontier = vec![puzzle.pieces];

        while new != 0 && frontier.len() < QUEUE_LIMIT {
            let mut next = Vec::new();

            for &pieces in &frontier {
                let state = ReducedFourBitPuzzle {
                    pieces,
                    gap: gap_position(pieces),
                };

                for mv in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = state;

                    while puzzle.do_move(mv) {
                        let idx = indexing_table.encode(puzzle.pieces()) as usize;
                        if claim(&mut pdb, idx, depth) == 1 {
                            next.push(puzzle.pieces());
                        }
                    }
                }
            }

            new = next.len() as u64;
            total += new;
            depth += 1;
            frontier = next;

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats {
                    depth: depth.into(),
                    new,
                    total,
                });
            }
        }

        drop(frontier);

        // The states that the last pull pass didn't reach, as (block, offset) pairs, if it listed
        // them.
        let mut unreached: Option<Vec<(u32, u16)>> = None;

        while new != 0 {
            let unvisited = SIZE as u64 - total;

            // Once fewer states are unvisited than are in the current layer, it's cheaper to check
            // each unvisited state for a neighbor in the layer than to expand the layer.
            let pull = unvisited < new;
            let target = if pull { u8::MAX } else { depth };

            // Once few states are unvisited, a pull pass lists the states it doesn't reach, so that
            // the next pass only checks those instead of scanning the whole table.
            let previous = unreached.take();
            let mut next = (pull && unvisited <= LIST_LIMIT).then(Vec::new);
            new = 0;

            if unvisited == 0 {
                // Every state has been visited, so the next layer is empty.
            } else if let (true, Some(previous)) = (pull, previous) {
                for (b, offset) in previous {
                    // SAFETY: Every listed offset came from a scan of its block, so it is in range.
                    new += unsafe {
                        blocks.pull(&mut pdb, b as usize, offset.into(), depth, &mut next)
                    };
                }
            } else {
                for (b, block) in blocks.blocks.iter().enumerate() {
                    let suffixes = &blocks.suffixes[block.suffixes as usize];
                    let start = block.start as usize;

                    let len = suffixes.len;
                    let mut j = 0;

                    while j < len {
                        let n = (len - j).min(CHUNK);
                        let mut mask = if n == CHUNK {
                            let chunk = pdb[start + j..start + j + CHUNK].try_into().unwrap();
                            depth_mask(chunk, target)
                        } else {
                            pdb[start + j..start + j + n]
                                .iter()
                                .enumerate()
                                .fold(0, |m, (k, &x)| m | (u64::from(x == target) << k))
                        };

                        while mask != 0 {
                            let offset = j + mask.trailing_zeros() as usize;
                            mask &= mask - 1;

                            if pull {
                                // SAFETY: The mask only has bits for offsets less than `len`.
                                new +=
                                    unsafe { blocks.pull(&mut pdb, b, offset, depth, &mut next) };
                            } else {
                                // SAFETY: The mask only has bits for offsets less than `len`.
                                let neighbors = unsafe { blocks.neighbors(block, offset) };
                                for idx in neighbors {
                                    // SAFETY: `neighbors` returns indices within `SIZE + PADDING`.
                                    new += unsafe { claim_unchecked(&mut pdb, idx, depth) };
                                }
                            }
                        }

                        j += CHUNK;
                    }
                }
            }

            unreached = next;
            total += new;
            depth += 1;

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats {
                    depth: depth.into(),
                    new,
                    total,
                });
            }
        }

        pdb.truncate(SIZE);
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
    fn test_depth_mask() {
        use rand::{RngExt as _, SeedableRng as _};

        let mut rng = rand_xoshiro::Xoshiro256StarStar::seed_from_u64(0);
        for _ in 0..10000 {
            // A random number of distinct values, so that chunks have anywhere from no matches to
            // all matches.
            let range = rng.random_range(1..=255);
            let chunk: [u8; CHUNK] = std::array::from_fn(|_| rng.random_range(0..range));
            let depth = rng.random_range(0..range);
            let expected = chunk
                .iter()
                .enumerate()
                .fold(0, |m, (k, &x)| m | (u64::from(x == depth) << k));
            assert_eq!(depth_mask(&chunk, depth), expected);
        }
    }

    #[test]
    fn test_pdb() {
        use std::sync::{Arc, Mutex};

        // The number of states at each distance, ending with the empty layer that stops the
        // search.
        const LAYERS: [u64; 35] = [
            1, 6, 18, 42, 106, 258, 637, 1594, 3701, 8773, 19995, 45077, 96900, 206786, 419189,
            833302, 1569248, 2868931, 4945258, 8171129, 12678971, 18480920, 25029510, 31216644,
            35536278, 36020069, 32283810, 24411593, 15109886, 6956267, 2156058, 363146, 24670, 427,
            0,
        ];

        let stats = Arc::new(Mutex::new(Vec::new()));
        let config = PdbConfig {
            end_of_iter_callback: Some(Box::new({
                let stats = Arc::clone(&stats);
                move |s| stats.lock().unwrap().push(s)
            })),
        };

        let indexing_table = IndexingTable::new();
        let pdb = Pdb::new(&indexing_table, &config);
        let hash = xxh3::xxh3_64(pdb.as_ref());
        assert_eq!(hash, HASH);

        let stats = stats.lock().unwrap();
        let mut total = 0;
        for (depth, (s, &new)) in stats.iter().zip(&LAYERS).enumerate() {
            total += new;
            assert_eq!((s.depth, s.new, s.total), (depth as u64, new, total));
        }
        assert_eq!(stats.len(), LAYERS.len());
    }
}
