use std::marker::PhantomData;

use crate::{
    algorithm::metric::Mtm,
    puzzle::small::{Puzzle, sealed::SmallPuzzle},
    solver::{
        config::PdbConfig,
        small::{
            coord::{BlockDecoder, Dense, Padded},
            pdb::Pdb,
        },
        statistics::PdbIterationStats,
    },
};

const HASHES: [(usize, usize, u64); 12] = [
    (2, 2, 0x120fc95229a80cba),
    (2, 3, 0x05e14c794c9d4a6e),
    (2, 4, 0xc243a7d8ff9cf87f),
    (2, 5, 0x8fa26d45af3f7647),
    (2, 6, 0xf546aef31edad546),
    (3, 2, 0xdb2d326033d3616a),
    (3, 3, 0xea0d873f776ff498),
    (3, 4, 0x712aff9703e58208),
    (4, 2, 0xd33671a1f26bcd8f),
    (4, 3, 0x6b313da9ba8f4d0f),
    (5, 2, 0x906d228f83a88fa0),
    (6, 2, 0x29d2d45cebe5f252),
];

/// The state of the breadth first search that builds a [`Pdb`].
///
/// The bitmaps use [`Padded`] indices, so each 64-bit word holds 4 blocks of states, one in each
/// 16-bit lane, and a state's neighbours by horizontal moves are in the same lane.
struct Builder<const W: usize, const H: usize> {
    words: Vec<Word>,
    /// Masks of the states in each word with certain properties.
    masks: Masks,
}

/// One word of each bitmap of [`Builder`]. They're interleaved because they're mostly accessed
/// together.
#[derive(Clone, Copy, Default)]
struct Word {
    /// Bits 0 and 1 of each state's entry: `d % 3`, where `d` is the depth of the state, or 3 if
    /// the state hasn't been reached. Positions that aren't states have entry 0.
    entry: [u64; 2],
    /// Whether the state has been expanded. Positions that aren't states are marked as expanded.
    done: u64,
    /// Whether all of the states that the state can reach by a vertical move have been reached,
    /// so that it doesn't need expanding vertically.
    column: u64,
}

/// Masks of the states in each word of the bitmaps with certain properties, which depend only on
/// the gap.
struct Masks {
    /// The states (with the gap in one of the `N` cells).
    valid: u64,
    /// The states where the gap can move left (decreasing the index by 1).
    left: u64,
    /// The states where the gap can move right (increasing the index by 1).
    right: u64,
    /// The states where the gap can move up (decreasing the gap by `W`).
    up: u64,
    /// The states where the gap can move down (increasing the gap by `W`).
    down: u64,
}

impl Word {
    /// The states with entry `r`.
    #[inline(always)]
    const fn with_entry(&self, r: u8) -> u64 {
        let lo = if r & 1 == 1 {
            self.entry[0]
        } else {
            !self.entry[0]
        };
        let hi = if r & 2 == 2 {
            self.entry[1]
        } else {
            !self.entry[1]
        };
        lo & hi
    }

    /// Sets the entries of `states`, which haven't been reached (so their entries are 3), to
    /// `r`.
    #[inline(always)]
    const fn set_entry(&mut self, states: u64, r: u8) {
        if r & 1 == 0 {
            self.entry[0] &= !states;
        }
        if r & 2 == 0 {
            self.entry[1] &= !states;
        }
    }
}

impl<const W: usize, const H: usize> Builder<W, H> {
    const N: usize = W * H;

    /// The number of blocks in each word.
    const LANES: u64 = 4;

    /// The number of states reached by vertical moves to collect before claiming them.
    const BATCH: usize = 64;

    fn new() -> Self {
        let words = Padded::<W, H>::BLOCKS.div_ceil(Self::LANES) as usize;

        let mut masks = Masks {
            valid: 0,
            left: 0,
            right: 0,
            up: 0,
            down: 0,
        };
        for lane in 0..Self::LANES {
            for gap in 0..Self::N {
                let bit = 1 << (16 * lane + gap as u64);
                let (x, y) = (gap % W, gap / W);
                masks.valid |= bit;
                if x > 0 {
                    masks.left |= bit;
                }
                if x + 1 < W {
                    masks.right |= bit;
                }
                if y > 0 {
                    masks.up |= bit;
                }
                if y + 1 < H {
                    masks.down |= bit;
                }
            }
        }

        let word = Word {
            entry: [masks.valid; 2],
            done: !masks.valid,
            column: 0,
        };
        let mut out = Self {
            words: vec![word; words],
            masks,
        };

        // Mark the lanes past the last block as not states.
        let lanes = Padded::<W, H>::BLOCKS % Self::LANES;
        if lanes != 0 {
            let unused = u64::MAX << (16 * lanes);
            let last = &mut out.words[words - 1];
            last.entry[0] &= !unused;
            last.entry[1] &= !unused;
            last.done |= unused;
        }

        out
    }

    /// Returns the word containing the state at `index`, and the state's bit in it.
    #[inline(always)]
    fn word_bit(&mut self, index: u64) -> (&mut Word, u64) {
        let word = (index / 64) as usize;
        debug_assert!(word < self.words.len());
        // SAFETY: `index` is the index of a state, so it's in bounds.
        let w = unsafe { self.words.get_unchecked_mut(word) };
        (w, 1 << (index % 64))
    }

    /// Marks the state at `index` as reached at a depth with entry `r`, if it hasn't been reached,
    /// and as not needing expanding vertically. Returns whether it was not already reached.
    #[inline(always)]
    fn claim(&mut self, index: u64, r: u8) -> bool {
        let (w, bit) = self.word_bit(index);
        w.column |= bit;
        // Whether the state is new is unpredictable, so update the entry without branching.
        let new = w.entry[0] & w.entry[1] & bit;
        w.set_entry(new, r);
        new != 0
    }

    /// Whether the state at `index` has entry `r`.
    #[inline(always)]
    fn has_entry(&mut self, index: u64, r: u8) -> bool {
        let (w, bit) = self.word_bit(index);
        w.with_entry(r) & bit != 0
    }

    /// Returns the states in word `i` at depth `depth`, and marks them as expanded.
    #[inline(always)]
    fn take_frontier(&mut self, i: usize, depth: u8) -> u64 {
        let w = &mut self.words[i];
        let frontier = w.with_entry(depth % 3) & !w.done;
        w.done |= frontier;
        frontier
    }

    /// Returns the states in a word reached from `states` in the same word by moving the gap
    /// horizontally any number of cells.
    #[inline(always)]
    const fn horizontal(&self, states: u64) -> u64 {
        let (mut left, mut right) = (states, states);
        let mut out = 0;
        let mut k = 1;
        while k < W {
            left = (left & self.masks.left) >> 1;
            right = (right & self.masks.right) << 1;
            out |= left | right;
            k += 1;
        }
        out
    }

    /// Calls `f(cell, pieces, index)` for each state reached from the state with the given
    /// pieces, gap, and index by moving the gap vertically, nearest first in each direction,
    /// stopping when `f` returns `true`. Returns whether `f` returned `true`.
    #[inline(always)]
    fn for_each_vertical(
        &mut self,
        pieces: u64,
        gap: u8,
        index: u64,
        mut f: impl FnMut(&mut Self, u8, u64, u64) -> bool,
    ) -> bool {
        let (mut p, mut g, mut i) = (pieces, gap, index);
        while g as usize >= W {
            i = i.wrapping_add(Padded::<W, H>::delta_up(p, g));
            let cell = g - W as u8;
            p = Padded::<W, H>::move_gap(p, g, cell);
            g = cell;
            if f(self, g, p, i) {
                return true;
            }
        }

        let (mut p, mut g, mut i) = (pieces, gap, index);
        while (g as usize) + W < Self::N {
            i = i.wrapping_add(Padded::<W, H>::delta_down(p, g));
            let cell = g + W as u8;
            p = Padded::<W, H>::move_gap(p, g, cell);
            g = cell;
            if f(self, g, p, i) {
                return true;
            }
        }

        false
    }

    /// Calls `f(index, pieces, gap)` for each state in word `i` in `states`, where `decoded` is
    /// the decoding of the word (see [`BlockDecoder::decode_word`]).
    #[inline(always)]
    fn for_each_state(
        &mut self,
        decoded: &[[u64; 2]; 4],
        i: usize,
        mut states: u64,
        mut f: impl FnMut(&mut Self, u64, u64, u8),
    ) {
        let base = 64 * i as u64;
        while states != 0 {
            let bit = states.trailing_zeros();
            states &= states - 1;

            let gap = (bit % 16) as u8;
            let ranked = decoded[(bit / 16) as usize][usize::from(Padded::<W, H>::is_odd(gap))];
            let pieces = Padded::<W, H>::insert_gap(ranked, gap);
            f(self, base + u64::from(bit), pieces, gap);
        }
    }

    /// Expands all states at depth `depth`, returning the number of new states at depth
    /// `depth + 1`.
    #[inline(always)]
    fn push(&mut self, depth: u8) -> u64 {
        let next = (depth + 1) % 3;
        let mut new = 0;
        let mut decoder = BlockDecoder::<W, H>::new();

        // The states reached by vertical moves, which are claimed in batches so that their words
        // can be loaded in parallel.
        let mut pending = Vec::with_capacity(2 * Self::BATCH);

        for i in 0..self.words.len() {
            let frontier = self.take_frontier(i, depth);
            if frontier == 0 {
                continue;
            }

            // Horizontal moves stay in the same lane, so do them for the whole word at once. The
            // new states have a different entry, so they aren't mistaken for part of the
            // frontier.
            let reached = self.horizontal(frontier) & self.words[i].with_entry(3);
            self.words[i].set_entry(reached, next);
            new += u64::from(reached.count_ones());

            // A state whose column has been expanded doesn't need expanding vertically.
            let vertical = frontier & !self.words[i].column & (self.masks.up | self.masks.down);
            if vertical == 0 {
                continue;
            }

            let decoded = decoder.decode_word(i as u64);
            self.for_each_state(&decoded, i, vertical, |s, index, pieces, gap| {
                s.for_each_vertical(pieces, gap, index, |s, _, _, child| {
                    s.prefetch(child);
                    pending.push(child);
                    false
                });
            });

            if pending.len() >= Self::BATCH {
                new += self.claim_all(&pending, next);
                pending.clear();
            }
        }

        new + self.claim_all(&pending, next)
    }

    /// Claims each state in `states` (see [`Self::claim`]), returning the number that were not
    /// already reached.
    #[inline(always)]
    fn claim_all(&mut self, states: &[u64], r: u8) -> u64 {
        states
            .iter()
            .map(|&index| u64::from(self.claim(index, r)))
            .sum()
    }

    /// Hints that the word containing the state at `index` will be accessed soon.
    #[inline(always)]
    fn prefetch(&self, index: u64) {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            let word = self.words.as_ptr().wrapping_add((index / 64) as usize);
            // SAFETY: `sse` is enabled on all x86_64 targets, and a prefetch doesn't access
            // memory, so the pointer doesn't need to be valid.
            unsafe {
                _mm_prefetch::<_MM_HINT_T0>(word.cast());
            }
        }
    }

    /// Finds the states at depth `depth + 1` by checking, for every state not yet reached, whether
    /// it has a neighbour at depth `depth`, returning the number found.
    ///
    /// A neighbour of a state not yet reached is at depth at least `depth`, so it's at depth
    /// `depth` exactly when its entry is `depth % 3`. States found in this pass have a different
    /// entry, so they aren't mistaken for neighbours at depth `depth`.
    #[inline(always)]
    fn pull(&mut self, depth: u8) -> u64 {
        let (current, next) = (depth % 3, (depth + 1) % 3);
        let mut new = 0;
        let mut decoder = BlockDecoder::<W, H>::new();

        for i in 0..self.words.len() {
            self.take_frontier(i, depth);

            let w = self.words[i];
            let candidates = w.with_entry(3);
            if candidates == 0 {
                continue;
            }

            // Horizontal moves stay in the same lane, so check them for the whole word at once.
            let mut found = candidates & self.horizontal(w.with_entry(current));

            let remaining = candidates & !found & (self.masks.up | self.masks.down);
            if remaining != 0 {
                let decoded = decoder.decode_word(i as u64);
                let base = 64 * i as u64;
                self.for_each_state(&decoded, i, remaining, |s, index, pieces, gap| {
                    let reached = s.for_each_vertical(pieces, gap, index, |s, _, _, child| {
                        s.has_entry(child, current)
                    });
                    found |= u64::from(reached) << (index - base);
                });
            }

            self.words[i].set_entry(found, next);
            new += u64::from(found.count_ones());
        }

        new
    }

    /// Expands the states in `queue`, which are at depth `depth`, returning the new states at
    /// depth `depth + 1`.
    ///
    /// This is used while the frontier is small, instead of scanning the whole bitmap. The states
    /// are `(index, pieces, gap)`.
    fn push_queue(&mut self, depth: u8, queue: &[(u64, u64, u8)]) -> Vec<(u64, u64, u8)> {
        let next_entry = (depth + 1) % 3;
        let mut next = Vec::new();

        for &(index, pieces, gap) in queue {
            let (w, bit) = self.word_bit(index);
            w.done |= bit;
            let column = w.column & bit != 0;

            // Moving the gap left or right by one cell changes the index by 1.
            let x = gap as usize % W;
            for (steps, sign) in [(x, u64::MAX), (W - 1 - x, 1)] {
                let (mut p, mut g, mut i) = (pieces, gap, index);
                for _ in 0..steps {
                    i = i.wrapping_add(sign);
                    let cell = g.wrapping_add(sign as u8);
                    p = Padded::<W, H>::move_gap(p, g, cell);
                    g = cell;
                    let (w, bit) = self.word_bit(i);
                    if w.entry[0] & w.entry[1] & bit != 0 {
                        w.set_entry(bit, next_entry);
                        next.push((i, p, g));
                    }
                }
            }

            if !column {
                self.for_each_vertical(pieces, gap, index, |s, cell, child_pieces, child| {
                    if s.claim(child, next_entry) {
                        next.push((child, child_pieces, cell));
                    }
                    false
                });
            }
        }

        next
    }

    /// Builds the table, two bits per state packed into bytes, with [`Dense`] indices.
    fn build(self, config: &PdbConfig) -> Box<[u8]> {
        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("popcnt")
            && is_x86_feature_detected!("bmi1")
            && is_x86_feature_detected!("bmi2")
        {
            // SAFETY: The target features are available.
            return unsafe { self.build_bmi2(config) };
        }

        self.build_impl(config)
    }

    /// [`Self::build`], compiled with instructions that make the bit manipulation cheaper:
    /// `popcnt`, and shifts by a variable amount that are a single instruction.
    ///
    /// # Safety
    ///
    /// The CPU must support the `popcnt`, `bmi1`, and `bmi2` target features.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "popcnt,bmi1,bmi2")]
    unsafe fn build_bmi2(self, config: &PdbConfig) -> Box<[u8]> {
        self.build_impl(config)
    }

    #[inline(always)]
    fn build_impl(mut self, config: &PdbConfig) -> Box<[u8]> {
        // The solved state has the gap in the last cell and the pieces in order, so all digits
        // except the gap's are 0.
        let solved_index = Self::N as u64 - 1;
        let (w, bit) = self.word_bit(solved_index);
        w.set_entry(bit, 0);

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        // The number of states not yet reached.
        let mut remaining = Padded::<W, H>::SIZE - 1;

        // While the frontier is small, keep it in a queue rather than scanning the bitmaps for it.
        let solved = (1..Self::N as u64).fold(0, |pieces, p| pieces | (p << (4 * (p - 1))));
        let mut queue = vec![(solved_index, solved, Self::N as u8 - 1)];
        let max_queue = self.words.len() / 32;

        loop {
            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
            if new == 0 {
                break;
            }

            new = if !queue.is_empty() && queue.len() <= max_queue {
                queue = self.push_queue(depth, &queue);
                queue.len() as u64
            } else {
                queue.clear();

                // Expand the frontier while it's small compared to the number of states not yet
                // reached, then switch to checking those states instead.
                if new * 2 < remaining {
                    self.push(depth)
                } else {
                    self.pull(depth)
                }
            };
            remaining -= new;
            total += new;
            depth += 1;
        }

        self.dense_bytes()
    }

    /// Returns the entries with [`Dense`] indices, two bits each, packed into bytes.
    fn dense_bytes(&self) -> Box<[u8]> {
        /// Spreads the low 16 bits of `x` to the even bits of the result.
        const fn spread(mut x: u64) -> u64 {
            x = (x | (x << 8)) & 0x00FF_00FF;
            x = (x | (x << 4)) & 0x0F0F_0F0F;
            x = (x | (x << 2)) & 0x3333_3333;
            (x | (x << 1)) & 0x5555_5555
        }

        let n = Self::N as u32;
        let len = Padded::<W, H>::SIZE.div_ceil(4) as usize;
        let mut out = Vec::with_capacity(len + 16);
        let mut acc = 0u128;
        let mut filled = 0;

        for block in 0..Padded::<W, H>::BLOCKS {
            let word = &self.words[(block / Self::LANES) as usize];
            let shift = 16 * (block % Self::LANES);
            let lo = (word.entry[0] >> shift) & ((1 << n) - 1);
            let hi = (word.entry[1] >> shift) & ((1 << n) - 1);
            acc |= u128::from(spread(lo) | (spread(hi) << 1)) << filled;
            filled += 2 * n;
            if filled >= 64 {
                out.extend_from_slice(&(acc as u64).to_le_bytes());
                acc >>= 64;
                filled -= 64;
            }
        }
        out.extend_from_slice(&(acc as u64).to_le_bytes());

        out.truncate(len);
        out.into_boxed_slice()
    }
}

impl<const W: usize, const H: usize, const N: usize> Default for Pdb<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<const W: usize, const H: usize, const N: usize> Pdb<W, H, N, Mtm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    /// Creates and builds a new pattern database for a `WxH` puzzle in the [`Mtm`] metric.
    ///
    /// Depending on the size of the puzzle, this may take several minutes to run.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        Self {
            pdb: Builder::<W, H>::new().build(config),
            phantom_metric: PhantomData,
        }
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
    /// [`Solver`]: crate::solver::small::solver::Solver
    /// [`xxh3`]: xxhash_rust::xxh3
    #[must_use]
    pub unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        let len = Dense::<W, H, N>::SIZE.div_ceil(4) as usize;

        // SAFETY: The caller's requirements are the same.
        unsafe { Self::try_from_bytes_with_hashes(bytes, len, &HASHES) }
    }

    /// Returns `d % 3`, where `d` is the optimal solution length of the state at `index`.
    ///
    /// # Safety
    ///
    /// `index` must be less than `N! / 2`.
    #[inline(always)]
    pub(in crate::solver::small) unsafe fn entry_unchecked(&self, index: u64) -> u8 {
        // SAFETY: The table has `ceil((N! / 2) / 4)` bytes, and `index < N! / 2`.
        let byte = unsafe { *self.pdb.get_unchecked((index / 4) as usize) };
        (byte >> (2 * (index % 4))) & 3
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use xxhash_rust::xxh3;

    use super::*;
    use crate::{algorithm::direction::Direction, puzzle::sliding_puzzle::SlidingPuzzle as _};

    fn tally<const W: usize, const H: usize, const N: usize>() -> Vec<u64>
    where
        Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
    {
        let tally = Arc::new(Mutex::new(Vec::new()));
        let t = Arc::clone(&tally);
        let config = PdbConfig {
            end_of_iter_callback: Some(Box::new(move |stats| {
                t.lock().unwrap().push(stats.new);
            })),
        };
        let _ = Pdb::<W, H, N, Mtm>::new(&config);
        let mut tally = tally.lock().unwrap().clone();
        assert_eq!(tally.pop(), Some(0));
        tally
    }

    /// Checks every entry against a breadth first search using the puzzle's own moves.
    fn check_entries<const W: usize, const H: usize, const N: usize>()
    where
        Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
    {
        let pdb = Pdb::<W, H, N, Mtm>::default();
        let mut distance = vec![u8::MAX; Dense::<W, H, N>::SIZE as usize];
        let solved = Puzzle::<W, H>::new();
        distance[Dense::<W, H, N>::index(solved.pieces(), solved.gap()) as usize] = 0;

        let mut current = vec![solved];
        let mut depth = 0;
        while !current.is_empty() {
            depth += 1;
            let mut next = Vec::new();
            for state in current {
                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = state;
                    while puzzle.try_move_dir(dir) {
                        let index = Dense::<W, H, N>::index(puzzle.pieces(), puzzle.gap());
                        if distance[index as usize] == u8::MAX {
                            distance[index as usize] = depth;
                            next.push(puzzle);
                        }
                    }
                }
            }
            current = next;
        }

        for (index, &d) in distance.iter().enumerate() {
            assert_ne!(d, u8::MAX);
            // SAFETY: `index < N! / 2`.
            let entry = unsafe { pdb.entry_unchecked(index as u64) };
            assert_eq!(entry, d % 3, "{W}x{H} index {index}");
        }
    }

    #[test]
    fn test_entries() {
        check_entries::<2, 2, 4>();
        check_entries::<2, 3, 6>();
        check_entries::<3, 2, 6>();
        check_entries::<2, 4, 8>();
        check_entries::<4, 2, 8>();
        check_entries::<3, 3, 9>();
        check_entries::<2, 5, 10>();
        check_entries::<5, 2, 10>();
    }

    #[test]
    fn test_5x2_tally() {
        let expected = [
            1, 5, 8, 20, 32, 80, 127, 300, 458, 960, 1458, 3055, 4540, 8780, 12694, 23520, 33370,
            57200, 77373, 119315, 151791, 204225, 235617, 256305, 246642, 178775, 120853, 49655,
            20885, 4530, 1229, 440, 113, 30, 8, 5, 1,
        ];

        assert_eq!(tally::<5, 2, 10>(), expected);
        assert_eq!(expected.iter().sum::<u64>(), 1_814_400);
    }

    #[test]
    fn test_3x3_tally() {
        let expected = [
            1, 4, 8, 16, 32, 64, 127, 244, 454, 856, 1576, 2854, 5117, 8588, 13466, 19739, 26558,
            31485, 30985, 23494, 11751, 3390, 589, 41, 1,
        ];

        assert_eq!(tally::<3, 3, 9>(), expected);
        assert_eq!(expected.iter().sum::<u64>(), 181_440);
    }

    fn check_pdb_hash<const W: usize, const H: usize, const N: usize>()
    where
        Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
    {
        let pdb = Pdb::<W, H, N, Mtm>::default();
        let actual = xxh3::xxh3_64(pdb.as_ref());
        let expected = HASHES
            .iter()
            .find(|(w, h, _)| *w == W && *h == H)
            .unwrap()
            .2;
        assert_eq!(
            actual, expected,
            "{W}x{H} MTM PDB hash mismatch, got {actual:#018x}, expected {expected:#018x}",
        );

        // SAFETY: The data was just built.
        let pdb = unsafe { Pdb::<W, H, N, Mtm>::try_from_bytes(pdb.as_ref().into()) };
        assert!(pdb.is_some());
    }

    #[test]
    fn test_small_hashes() {
        check_pdb_hash::<2, 2, 4>();
        check_pdb_hash::<2, 3, 6>();
        check_pdb_hash::<2, 4, 8>();
        check_pdb_hash::<2, 5, 10>();
        check_pdb_hash::<3, 2, 6>();
        check_pdb_hash::<3, 3, 9>();
        check_pdb_hash::<4, 2, 8>();
        check_pdb_hash::<5, 2, 10>();
    }
}
