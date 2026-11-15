use std::marker::PhantomData;

use crate::{
    algorithm::metric::Stm,
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
    (2, 2, 0xbcd12cbfebf72525),
    (2, 3, 0xbdf02d4cbc3fb012),
    (2, 4, 0x6608274392d11a01),
    (2, 5, 0x7dd8055ed0ccb63a),
    (2, 6, 0x614aee89568d41ce),
    (3, 2, 0x898d80d6658b99ee),
    (3, 3, 0xb03284f7991700e2),
    (3, 4, 0x27741d91bdab73eb),
    (4, 2, 0xce58545dbae58122),
    (4, 3, 0xf7de3733261657e9),
    (5, 2, 0x6648cacc60d35c5c),
    (6, 2, 0x1f74ed8c37542528),
];

/// The state of the breadth first search that builds a [`Pdb`].
///
/// The bitmaps use [`Padded`] indices, so each 64-bit word holds 4 blocks of states, one in each
/// 16-bit lane, and a state's neighbours by horizontal moves are in the same lane.
struct Builder<const W: usize, const H: usize> {
    /// Which states have been reached.
    reached: Vec<Reached>,
    /// One bit per state: whether the state has been expanded.
    done: Vec<u64>,
    /// One bit per state: `(d >> 1) & 1`, where `d` is the depth of the state.
    bits: Vec<u64>,
    /// Masks of the states in each word with certain properties.
    masks: Masks,
}

/// One word of [`Builder::reached`]. The two bitmaps are interleaved because they're updated
/// together.
#[derive(Clone, Copy, Default)]
struct Reached {
    /// One bit per state: whether the state has been reached.
    any: u64,
    /// One bit per state: whether the state was reached by a vertical move, before it was reached
    /// by a horizontal move in the same layer or expanded.
    vertically: u64,
}

/// Masks of the states in each word of the bitmaps with certain properties, which depend only on
/// the gap.
struct Masks {
    /// `parity[k]` is the states whose depth has parity `k`.
    parity: [u64; 2],
    /// The states where the gap can move left (decreasing the index by 1).
    left: u64,
    /// The states where the gap can move right (increasing the index by 1).
    right: u64,
    /// The states where the gap can move up (decreasing the gap by `W`).
    up: u64,
    /// The states where the gap can move down (increasing the gap by `W`).
    down: u64,
    /// The states with the gap in the top or bottom row, which have only one vertical move.
    edge_rows: u64,
}

impl<const W: usize, const H: usize> Builder<W, H> {
    const N: usize = W * H;

    /// The number of blocks in each word.
    const LANES: u64 = 4;

    fn new() -> Self {
        let words = Padded::<W, H>::BLOCKS.div_ceil(Self::LANES) as usize;

        let mut masks = Masks {
            parity: [0; 2],
            left: 0,
            right: 0,
            up: 0,
            down: 0,
            edge_rows: 0,
        };
        for lane in 0..Self::LANES {
            for gap in 0..Self::N {
                let bit = 1 << (16 * lane + gap as u64);
                let (x, y) = (gap % W, gap / W);
                masks.parity[((W - 1 - x) + (H - 1 - y)) % 2] |= bit;
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
                if y == 0 || y == H - 1 {
                    masks.edge_rows |= bit;
                }
            }
        }

        let mut out = Self {
            reached: vec![Reached::default(); words],
            done: vec![0; words],
            bits: vec![0; words],
            masks,
        };

        // Mark the lanes past the last block as reached and expanded, so that they're never
        // treated as states.
        let lanes = Padded::<W, H>::BLOCKS % Self::LANES;
        if lanes != 0 {
            let unused = u64::MAX << (16 * lanes);
            out.reached[words - 1].any = unused;
            out.done[words - 1] = unused;
        }

        out
    }

    /// Returns the word of [`Self::reached`] containing the state at `index`, and the state's bit
    /// in it.
    #[inline(always)]
    fn reached_bit(&mut self, index: u64) -> (&mut Reached, u64) {
        let word = (index / 64) as usize;
        debug_assert!(word < self.reached.len());
        // SAFETY: `index` is the index of a state, so it's in bounds.
        let r = unsafe { self.reached.get_unchecked_mut(word) };
        (r, 1 << (index % 64))
    }

    /// Marks the state at `index` as reached, returning whether it was not already reached.
    #[inline(always)]
    fn claim(&mut self, index: u64) -> bool {
        let (r, bit) = self.reached_bit(index);
        let new = r.any & bit == 0;
        r.any |= bit;
        new
    }

    /// Like [`Self::claim`], for a state reached by a vertical move.
    #[inline(always)]
    fn claim_vertically(&mut self, index: u64) -> bool {
        let (r, bit) = self.reached_bit(index);
        r.vertically |= bit & !r.any;
        let new = r.any & bit == 0;
        r.any |= bit;
        new
    }

    /// Whether the state at `index` has been reached.
    #[inline(always)]
    fn is_visited(&mut self, index: u64) -> bool {
        let (r, bit) = self.reached_bit(index);
        r.any & bit != 0
    }

    /// Marks `states` in word `i`, which are at depth `depth`, as expanded.
    #[inline(always)]
    fn mark_expanded(&mut self, i: usize, states: u64, depth: u8) {
        self.done[i] |= states;
        if (depth >> 1) & 1 == 1 {
            self.bits[i] |= states;
        }
    }

    /// Returns the states in word `i` at depth `depth`, and marks them as expanded.
    #[inline(always)]
    fn take_frontier(&mut self, i: usize, depth: u8) -> u64 {
        let parity = self.masks.parity[usize::from(depth % 2)];
        let frontier = self.reached[i].any & !self.done[i] & parity;
        self.mark_expanded(i, frontier, depth);
        frontier
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
        let mut new = 0;
        let mut decoder = BlockDecoder::<W, H>::new();

        for i in 0..self.reached.len() {
            let frontier = self.take_frontier(i, depth);
            if frontier == 0 {
                continue;
            }

            // Horizontal moves stay in the same lane, so do them for the whole word at once. The
            // new states have the other parity, so they aren't mistaken for part of the frontier.
            let reached =
                ((frontier & self.masks.left) >> 1) | ((frontier & self.masks.right) << 1);
            new += u64::from((reached & !self.reached[i].any).count_ones());
            self.reached[i].any |= reached;

            // A state in the top or bottom row has only one vertical move, so if it was reached
            // by a vertical move, that move goes back to a state that has already been reached.
            let vertical = frontier & !(self.reached[i].vertically & self.masks.edge_rows);

            if vertical == 0 {
                continue;
            }

            // Do the moves up and down separately, so that there's no branch on the direction.
            let decoded = decoder.decode_word(i as u64);
            self.for_each_state(
                &decoded,
                i,
                vertical & self.masks.up,
                |s, index, pieces, gap| {
                    let delta = Padded::<W, H>::delta_up(pieces, gap);
                    new += u64::from(s.claim_vertically(index.wrapping_add(delta)));
                },
            );
            self.for_each_state(
                &decoded,
                i,
                vertical & self.masks.down,
                |s, index, pieces, gap| {
                    let delta = Padded::<W, H>::delta_down(pieces, gap);
                    new += u64::from(s.claim_vertically(index.wrapping_add(delta)));
                },
            );
        }

        new
    }

    /// Finds the states at depth `depth + 1` by checking, for every state not yet reached, whether
    /// it has a neighbour at depth `depth`, returning the number found.
    ///
    /// A neighbour of a state not yet reached has been reached if and only if it is at depth
    /// `depth`, because otherwise the state would have been reached already.
    #[inline(always)]
    fn pull(&mut self, depth: u8) -> u64 {
        let mut new = 0;
        let mut decoder = BlockDecoder::<W, H>::new();

        for i in 0..self.reached.len() {
            self.take_frontier(i, depth);

            let candidates = !self.reached[i].any & self.masks.parity[usize::from(depth % 2) ^ 1];
            if candidates == 0 {
                continue;
            }

            // Horizontal moves stay in the same lane, so check them for the whole word at once.
            let visited = self.reached[i].any;
            let mut found = candidates
                & ((self.masks.left & (visited << 1)) | (self.masks.right & (visited >> 1)));

            let remaining = candidates & !found;
            if remaining != 0 {
                let decoded = decoder.decode_word(i as u64);
                let base = 64 * i as u64;
                self.for_each_state(&decoded, i, remaining, |s, index, pieces, gap| {
                    let g = gap as usize;
                    let reached = (g >= W
                        && s.is_visited(index.wrapping_add(Padded::<W, H>::delta_up(pieces, gap))))
                        || (g + W < Self::N
                            && s.is_visited(
                                index.wrapping_add(Padded::<W, H>::delta_down(pieces, gap)),
                            ));
                    found |= u64::from(reached) << (index - base);
                });
            }

            self.reached[i].any |= found;
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
        let mut next = Vec::new();

        for &(index, pieces, gap) in queue {
            self.mark_expanded((index / 64) as usize, 1 << (index % 64), depth);

            let g = gap as usize;
            let mut visit = |s: &mut Self, child: u64, cell: usize| {
                if s.claim(child) {
                    let cell = cell as u8;
                    next.push((child, Padded::<W, H>::move_gap(pieces, gap, cell), cell));
                }
            };

            if g >= W {
                let delta = Padded::<W, H>::delta_up(pieces, gap);
                visit(self, index.wrapping_add(delta), g - W);
            }
            if g + W < Self::N {
                let delta = Padded::<W, H>::delta_down(pieces, gap);
                visit(self, index.wrapping_add(delta), g + W);
            }
            if !g.is_multiple_of(W) {
                visit(self, index - 1, g - 1);
            }
            if g % W + 1 < W {
                visit(self, index + 1, g + 1);
            }
        }

        next
    }

    /// Builds the table, one bit per state packed into bytes, with [`Dense`] indices.
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
        self.claim(Self::N as u64 - 1);

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        // The number of states not yet reached with each parity of depth.
        let mut remaining = self
            .masks
            .parity
            .map(|m| u64::from((m & 0xFFFF).count_ones()) * Padded::<W, H>::BLOCKS);
        remaining[0] -= 1;

        // While the frontier is small, keep it in a queue rather than scanning the bitmaps for it.
        let solved = (1..Self::N as u64).fold(0, |pieces, p| pieces | (p << (4 * (p - 1))));
        let mut queue = vec![(Self::N as u64 - 1, solved, Self::N as u8 - 1)];
        let max_queue = self.reached.len() / 32;

        loop {
            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
            if new == 0 {
                break;
            }

            let next = &mut remaining[usize::from(depth % 2) ^ 1];
            new = if !queue.is_empty() && queue.len() <= max_queue {
                queue = self.push_queue(depth, &queue);
                queue.len() as u64
            } else {
                queue.clear();

                // Expand the frontier while it's small compared to the number of states that
                // could be in the next layer, then switch to checking those states instead.
                if new * 3 < *next * 2 {
                    self.push(depth)
                } else {
                    self.pull(depth)
                }
            };
            *next -= new;
            total += new;
            depth += 1;
        }

        self.dense_bytes()
    }

    /// Returns `bits` with [`Dense`] indices, packed into bytes.
    fn dense_bytes(&self) -> Box<[u8]> {
        let n = Self::N as u32;
        let len = Padded::<W, H>::SIZE.div_ceil(8) as usize;
        let mut out = Vec::with_capacity(len + 16);
        let mut acc = 0u128;
        let mut filled = 0;

        for block in 0..Padded::<W, H>::BLOCKS {
            let word = self.bits[(block / Self::LANES) as usize];
            let lane = (word >> (16 * (block % Self::LANES))) & ((1 << n) - 1);
            acc |= u128::from(lane) << filled;
            filled += n;
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

impl<const W: usize, const H: usize, const N: usize> Default for Pdb<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    fn default() -> Self {
        Self::new(&PdbConfig::default())
    }
}

impl<const W: usize, const H: usize, const N: usize> Pdb<W, H, N, Stm>
where
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    /// Creates and builds a new pattern database for a `WxH` puzzle in the [`Stm`] metric.
    ///
    /// Depending on the size of the puzzle, this may take several seconds to run.
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
        let len = Dense::<W, H, N>::SIZE.div_ceil(8) as usize;

        // SAFETY: The caller's requirements are the same.
        unsafe { Self::try_from_bytes_with_hashes(bytes, len, &HASHES) }
    }

    /// Returns `(d >> 1) & 1`, where `d` is the optimal solution length of the state at `index`.
    ///
    /// # Safety
    ///
    /// `index` must be less than `N! / 2`.
    #[inline(always)]
    pub(in crate::solver::small) unsafe fn bit_unchecked(&self, index: u64) -> u8 {
        // SAFETY: The table has `ceil((N! / 2) / 8)` bytes, and `index < N! / 2`.
        let byte = unsafe { *self.pdb.get_unchecked((index / 8) as usize) };
        (byte >> (index % 8)) & 1
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use xxhash_rust::xxh3;

    use super::*;

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
        let _ = Pdb::<W, H, N, Stm>::new(&config);
        let mut tally = tally.lock().unwrap().clone();
        assert_eq!(tally.pop(), Some(0));
        tally
    }

    #[test]
    fn test_5x2_tally() {
        let expected = [
            1, 2, 3, 6, 11, 19, 30, 44, 68, 112, 176, 271, 411, 602, 851, 1232, 1783, 2530, 3567,
            4996, 6838, 9279, 12463, 16597, 21848, 28227, 35682, 44464, 54597, 65966, 78433, 91725,
            104896, 116966, 126335, 131998, 133107, 128720, 119332, 106335, 91545, 75742, 60119,
            45840, 33422, 23223, 15140, 9094, 5073, 2605, 1224, 528, 225, 75, 20, 2,
        ];

        assert_eq!(tally::<5, 2, 10>(), expected);
        assert_eq!(tally::<2, 5, 10>(), expected);
    }

    #[test]
    fn test_3x3_tally() {
        let expected = [
            1, 2, 4, 8, 16, 20, 39, 62, 116, 152, 286, 396, 748, 1024, 1893, 2512, 4485, 5638,
            9529, 10878, 16993, 17110, 23952, 20224, 24047, 15578, 14560, 6274, 3910, 760, 221, 2,
        ];

        assert_eq!(tally::<3, 3, 9>(), expected);
    }

    fn check_pdb_hash<const W: usize, const H: usize, const N: usize>()
    where
        Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
    {
        let pdb = Pdb::<W, H, N, Stm>::default();
        let actual = xxh3::xxh3_64(pdb.as_ref());
        let expected = HASHES
            .iter()
            .find(|(w, h, _)| *w == W && *h == H)
            .unwrap()
            .2;
        assert_eq!(
            actual, expected,
            "{W}x{H} STM PDB hash mismatch, got {actual:#018x}, expected {expected:#018x}",
        );

        // SAFETY: The data was just built.
        let pdb = unsafe { Pdb::<W, H, N, Stm>::try_from_bytes(pdb.as_ref().into()) };
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
