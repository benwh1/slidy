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
    /// One bit per state: whether the state has been reached.
    visited: Vec<u64>,
    /// One bit per state: whether the state has been expanded.
    done: Vec<u64>,
    /// One bit per state: `(d >> 1) & 1`, where `d` is the depth of the state.
    bits: Vec<u64>,
    /// Masks of the states in each word with certain properties.
    masks: Masks,
}

/// Masks of the states in each word of the bitmaps with certain properties, which depend only on
/// the gap.
struct Masks {
    /// The states (with gap less than `N`).
    valid: u64,
    /// The states with odd depth.
    odd: u64,
    /// The states where the gap can move left (decreasing the index by 1).
    left: u64,
    /// The states where the gap can move right (increasing the index by 1).
    right: u64,
}

impl<const W: usize, const H: usize> Builder<W, H> {
    const N: usize = W * H;

    /// The number of blocks in each word.
    const LANES: u64 = 4;

    fn new() -> Self {
        let words = Padded::<W, H>::BLOCKS.div_ceil(Self::LANES) as usize;

        let mut masks = Masks {
            valid: 0,
            odd: 0,
            left: 0,
            right: 0,
        };
        for lane in 0..Self::LANES {
            for gap in 0..Self::N {
                let bit = 1 << (16 * lane + gap as u64);
                let (x, y) = (gap % W, gap / W);
                let dist = (W - 1 - x) + (H - 1 - y);
                masks.valid |= bit;
                if dist % 2 == 1 {
                    masks.odd |= bit;
                }
                if x > 0 {
                    masks.left |= bit;
                }
                if x + 1 < W {
                    masks.right |= bit;
                }
            }
        }

        Self {
            visited: vec![0; words],
            done: vec![0; words],
            bits: vec![0; words],
            masks,
        }
    }

    /// The mask of states in word `i` with the parity of depth `depth`.
    #[inline(always)]
    fn parity_mask(&self, i: usize, depth: u8) -> u64 {
        let masks = &self.masks;
        let parity = if depth % 2 == 1 {
            masks.odd
        } else {
            masks.valid & !masks.odd
        };

        // Lanes past the last block are unused.
        let lanes = Padded::<W, H>::BLOCKS - i as u64 * Self::LANES;
        if lanes < Self::LANES {
            parity & ((1 << (16 * lanes)) - 1)
        } else {
            parity
        }
    }

    /// Marks the state at `index` as reached, returning whether it was not already reached.
    #[inline(always)]
    fn claim(&mut self, index: u64) -> bool {
        let (word, mask) = ((index / 64) as usize, 1 << (index % 64));
        let v = &mut self.visited[word];
        let new = *v & mask == 0;
        *v |= mask;
        new
    }

    /// Whether the state at `index` has been reached.
    #[inline(always)]
    fn is_visited(&self, index: u64) -> bool {
        (self.visited[(index / 64) as usize] >> (index % 64)) & 1 == 1
    }

    /// Returns the states in word `i` at depth `depth`, and marks them as expanded.
    #[inline(always)]
    fn take_frontier(&mut self, i: usize, depth: u8) -> u64 {
        let frontier = self.visited[i] & !self.done[i] & self.parity_mask(i, depth);
        self.done[i] |= frontier;
        if (depth >> 1) & 1 == 1 {
            self.bits[i] |= frontier;
        }
        frontier
    }

    /// Calls `f(index, pieces, gap)` for each state in word `i` in `states`.
    #[inline(always)]
    fn for_each_state(
        &mut self,
        decoder: &mut BlockDecoder<W, H>,
        i: usize,
        mut states: u64,
        mut f: impl FnMut(&mut Self, u64, u64, u8),
    ) {
        while states != 0 {
            let lane = u64::from(states.trailing_zeros() / 16);
            let mut lane_states = (states >> (16 * lane)) & 0xFFFF;
            states &= !(0xFFFF << (16 * lane));

            let block = i as u64 * Self::LANES + lane;
            decoder.decode(block);
            let base = 16 * block;

            while lane_states != 0 {
                let gap = lane_states.trailing_zeros() as u8;
                lane_states &= lane_states - 1;

                let ranked = decoder.ranked(Padded::<W, H>::is_odd(gap));
                let pieces = Padded::<W, H>::insert_gap(ranked, gap);
                f(self, base + u64::from(gap), pieces, gap);
            }
        }
    }

    /// Expands all states at depth `depth`, returning the number of new states at depth
    /// `depth + 1`.
    fn push(&mut self, depth: u8) -> u64 {
        let mut new = 0;
        let mut decoder = BlockDecoder::<W, H>::new();

        for i in 0..self.visited.len() {
            let frontier = self.take_frontier(i, depth);
            if frontier == 0 {
                continue;
            }

            // Horizontal moves stay in the same lane, so do them for the whole word at once. The
            // new states have the other parity, so they aren't mistaken for part of the frontier.
            let reached =
                ((frontier & self.masks.left) >> 1) | ((frontier & self.masks.right) << 1);
            new += u64::from((reached & !self.visited[i]).count_ones());
            self.visited[i] |= reached;

            // Every state has a vertical move, since `H >= 2`.
            self.for_each_state(&mut decoder, i, frontier, |s, index, pieces, gap| {
                let g = gap as usize;
                if g >= W {
                    let delta = Padded::<W, H>::delta_up(pieces, gap);
                    new += u64::from(s.claim(index.wrapping_add(delta)));
                }
                if g + W < Self::N {
                    let delta = Padded::<W, H>::delta_down(pieces, gap);
                    new += u64::from(s.claim(index.wrapping_add(delta)));
                }
            });
        }

        new
    }

    /// Finds the states at depth `depth + 1` by checking, for every state not yet reached, whether
    /// it has a neighbour at depth `depth`, returning the number found.
    ///
    /// A neighbour of a state not yet reached has been reached if and only if it is at depth
    /// `depth`, because otherwise the state would have been reached already.
    fn pull(&mut self, depth: u8) -> u64 {
        let mut new = 0;
        let mut decoder = BlockDecoder::<W, H>::new();

        for i in 0..self.visited.len() {
            self.take_frontier(i, depth);

            let candidates = !self.visited[i] & self.parity_mask(i, depth + 1);
            if candidates == 0 {
                continue;
            }

            // Horizontal moves stay in the same lane, so check them for the whole word at once.
            let visited = self.visited[i];
            let mut found = candidates
                & ((self.masks.left & (visited << 1)) | (self.masks.right & (visited >> 1)));

            let remaining = candidates & !found;
            let base = 64 * i as u64;
            self.for_each_state(&mut decoder, i, remaining, |s, index, pieces, gap| {
                let g = gap as usize;
                let reached = (g >= W
                    && s.is_visited(index.wrapping_add(Padded::<W, H>::delta_up(pieces, gap))))
                    || (g + W < Self::N
                        && s.is_visited(
                            index.wrapping_add(Padded::<W, H>::delta_down(pieces, gap)),
                        ));
                found |= u64::from(reached) << (index - base);
            });

            self.visited[i] |= found;
            new += u64::from(found.count_ones());
        }

        new
    }

    /// Builds the table, one bit per state packed into bytes, with [`Dense`] indices.
    fn build(mut self, config: &PdbConfig) -> Box<[u8]> {
        // The solved state has the gap in the last cell and the pieces in order, so all digits
        // except the gap's are 0.
        self.claim(Self::N as u64 - 1);

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
            f(PdbIterationStats { depth, new, total });
        }

        // The number of states not yet reached with each parity of depth.
        let mut remaining = [0; 2];
        for gap in 0..Self::N {
            let (x, y) = (gap % W, gap / W);
            let dist = (W - 1 - x) + (H - 1 - y);
            remaining[dist % 2] += Padded::<W, H>::BLOCKS;
        }
        remaining[0] -= 1;

        while new != 0 {
            // Expand the frontier while it's small compared to the number of states that could be
            // in the next layer, then switch to checking those states instead.
            let next = &mut remaining[usize::from(depth % 2 == 0)];
            new = if new * 2 < *next {
                self.push(depth)
            } else {
                self.pull(depth)
            };
            *next -= new;
            total += new;
            depth += 1;

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        self.dense_bytes()
    }

    /// Returns `bits` with [`Dense`] indices, packed into bytes.
    fn dense_bytes(&self) -> Box<[u8]> {
        let n = Self::N as u64;
        let lane_mask = (1 << n) - 1;

        let mut out = Vec::with_capacity(Padded::<W, H>::SIZE.div_ceil(64) as usize);
        let mut acc = 0u64;
        let mut filled = 0;

        for block in 0..Padded::<W, H>::BLOCKS {
            let word = self.bits[(block / Self::LANES) as usize];
            let lane = (word >> (16 * (block % Self::LANES))) & lane_mask;

            acc |= lane << filled;
            filled += n;
            if filled >= 64 {
                out.push(acc);
                filled -= 64;
                acc = if filled == 0 { 0 } else { lane >> (n - filled) };
            }
        }
        if filled > 0 {
            out.push(acc);
        }

        let len = Padded::<W, H>::SIZE.div_ceil(8) as usize;
        out.iter().flat_map(|w| w.to_le_bytes()).take(len).collect()
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
