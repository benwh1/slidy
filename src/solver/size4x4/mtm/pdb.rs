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

/// Layers are expanded from a queue while they have fewer states than this.
const QUEUE_LIMIT: usize = 1 << 18;

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
    const ONES: u64 = 0x1111_1111_1111_1111;
    const HIGHS: u64 = 0x8888_8888_8888_8888;

    // The lowest flagged nibble is exactly the lowest zero nibble.
    let zeros = pieces.wrapping_sub(ONES) & !pieces & HIGHS;
    (zeros.trailing_zeros() / 4) as u8
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
///   start `cross[p]`, and offsets from [`Suffixes::cross`] at `2 * c` and `2 * c + 1`.
///
/// If the gap is in the suffix:
/// - the four moves within the suffix keep the prefix, so the children stay in this block, at
///   offsets from [`Suffixes::moves`];
/// - the two moves into the prefix (to rows 1 and 0) push out the piece `q` from cell `4 + c`. The
///   children have starts `cross[2 * c]` and `cross[2 * c + 1]`, and the same offset from
///   [`Suffixes::cross`] at `q`.
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

/// The possible suffixes of a block, which depend only on the pieces left over from the prefix.
struct Suffixes {
    /// The pieces in cells 8..16, in index order.
    suffixes: Vec<u32>,
    /// If the gap is in the suffix, the offsets reached from each suffix by [`same_half_moves`].
    moves: Vec<[u16; 4]>,
    /// The offsets reached from each suffix by moves that take the gap to the other half. See
    /// [`Block`].
    cross: Vec<[u16; 8]>,
}

/// Every state's index split into blocks, so that the table can be walked in index order and the
/// state at each index read off directly.
struct Blocks {
    blocks: Vec<Block>,
    suffixes: Vec<Suffixes>,
}

impl Blocks {
    fn new(indexing_table: &IndexingTable, base_5_table: &Base5Table) -> Self {
        let encode_prefix = |prefix| indexing_table.encode_prefix(prefix, base_5_table);
        let encode_suffix = |suffix| indexing_table.encode_suffix(suffix, base_5_table) as u16;

        let mut blocks = Vec::new();
        let mut suffixes: Vec<Suffixes> = Vec::new();
        let mut suffix_ids = std::collections::HashMap::new();
        let mut start = 0;

        for_each_arrangement(8, TALLY, &mut |prefix, remaining| {
            let gap_in_prefix = remaining[0] == 0;

            let id = *suffix_ids.entry(remaining).or_insert_with(|| {
                let mut list = Vec::new();
                for_each_arrangement(8, remaining, &mut |suffix, _| list.push(suffix));

                let (moves, cross) = if gap_in_prefix {
                    let cross = list
                        .iter()
                        .map(|&s| {
                            let mut cross = [0; 8];
                            for c in 0..4 {
                                // The gap moves to cell `8 + c`, or on to cell `12 + c`.
                                cross[2 * c as usize] = encode_suffix(set(s, c, 0));
                                cross[2 * c as usize + 1] =
                                    encode_suffix(set(set(s, c, get(s, c + 4)), c + 4, 0));
                            }
                            cross
                        })
                        .collect();
                    (Vec::new(), cross)
                } else {
                    let moves = list
                        .iter()
                        .map(|&s| {
                            let pieces = u64::from(prefix) | (u64::from(s) << 32);
                            same_half_moves(pieces).map(|p| encode_suffix((p >> 32) as u32))
                        })
                        .collect();
                    let cross = list
                        .iter()
                        .map(|&s| {
                            // The suffix's gap is its lowest zero nibble.
                            let h = u32::from(gap_position(u64::from(s)));
                            let c = h % 4;
                            let mut cross = [0; 8];
                            for q in 1..5 {
                                let child = if h < 4 {
                                    set(s, c, q)
                                } else {
                                    set(set(s, c + 4, get(s, c)), c, q)
                                };
                                cross[q as usize] = encode_suffix(child);
                            }
                            cross
                        })
                        .collect();
                    (moves, cross)
                };

                suffixes.push(Suffixes {
                    suffixes: list,
                    moves,
                    cross,
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
                for p in 1..5 {
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
                        let idx = indexing_table.encode(puzzle.pieces(), base_5_table) as usize;
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
                f(PdbIterationStats { depth, new, total });
            }
        }

        drop(frontier);

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

                        let suffix = suffixes.suffixes[offset];
                        let cross = &suffixes.cross[offset];

                        if block.gap_in_prefix {
                            for child in block.children {
                                new += claim(&mut pdb, child as usize + offset, depth);
                            }

                            let c = u32::from(block.gap_col);
                            let child = block.cross[get(suffix, c) as usize] as usize;
                            new += claim(&mut pdb, child + cross[2 * c as usize] as usize, depth);
                            new +=
                                claim(&mut pdb, child + cross[2 * c as usize + 1] as usize, depth);
                        } else {
                            for child in suffixes.moves[offset] {
                                new += claim(&mut pdb, start + child as usize, depth);
                            }

                            let c = u32::from(gap_position(u64::from(suffix))) % 4;
                            let child = cross[get(block.prefix, c + 4) as usize] as usize;
                            new += claim(
                                &mut pdb,
                                block.cross[2 * c as usize] as usize + child,
                                depth,
                            );
                            new += claim(
                                &mut pdb,
                                block.cross[2 * c as usize + 1] as usize + child,
                                depth,
                            );
                        }
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
