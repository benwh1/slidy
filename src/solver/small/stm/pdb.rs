use std::marker::PhantomData;

use crate::{
    algorithm::metric::Stm,
    puzzle::small::{Puzzle, sealed::SmallPuzzle},
    solver::{
        config::PdbConfig,
        small::{coord::Coord, pdb::Pdb},
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
struct Builder<const W: usize, const H: usize> {
    /// One bit per state: whether the state has been reached.
    visited: Vec<u64>,
    /// One bit per state: whether the state has been expanded.
    done: Vec<u64>,
    /// One bit per state: `(d >> 1) & 1`, where `d` is the depth of the state.
    bits: Vec<u64>,
    /// `odd_masks[k]` has bit `i` set if the state at index `64 * j + i` has odd depth, for any
    /// `j` with `64 * j % N == k`.
    odd_masks: [u64; 16],
}

impl<const W: usize, const H: usize> Builder<W, H> {
    const N: usize = W * H;

    fn new() -> Self {
        let words = Coord::<W, H>::SIZE.div_ceil(64) as usize;

        let mut odd_masks = [0; 16];
        for (k, mask) in odd_masks.iter_mut().enumerate().take(Self::N) {
            for i in 0..64 {
                let gap = (k + i) % Self::N;
                let (x, y) = (gap % W, gap / W);
                let dist = (W - 1 - x) + (H - 1 - y);
                *mask |= ((dist % 2) as u64) << i;
            }
        }

        Self {
            visited: vec![0; words],
            done: vec![0; words],
            bits: vec![0; words],
            odd_masks,
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

    /// Expands all states at depth `depth`, returning the number of new states at depth
    /// `depth + 1`.
    fn expand(&mut self, depth: u8) -> u64 {
        let n = Self::N as u64;
        let odd = depth % 2 == 1;
        let bit = (depth >> 1) & 1 == 1;

        let mut new = 0;
        let mut cached_block = u64::MAX;
        let mut decoded = [0; 2];
        let mut offset = 0;

        for i in 0..self.visited.len() {
            let odd_mask = self.odd_masks[offset];
            offset = (offset + 64) % Self::N;

            let parity_mask = if odd { odd_mask } else { !odd_mask };
            let mut frontier = self.visited[i] & !self.done[i] & parity_mask;
            if frontier == 0 {
                continue;
            }

            self.done[i] |= frontier;
            if bit {
                self.bits[i] |= frontier;
            }

            while frontier != 0 {
                let index = 64 * i as u64 + frontier.trailing_zeros() as u64;
                frontier &= frontier - 1;

                let block = index / n;
                let gap = (index % n) as u8;
                if block != cached_block {
                    cached_block = block;
                    decoded = Coord::<W, H>::decode_block(block);
                }
                let ranked = decoded[usize::from(Coord::<W, H>::is_odd(gap))];
                let pieces = Coord::<W, H>::insert_gap(ranked, gap);

                let g = gap as usize;
                if !g.is_multiple_of(W) {
                    new += u64::from(self.claim(index - 1));
                }
                if g % W + 1 < W {
                    new += u64::from(self.claim(index + 1));
                }
                if g >= W {
                    let delta = Coord::<W, H>::delta_up(pieces, gap);
                    new += u64::from(self.claim(index.wrapping_add(delta)));
                }
                if g + W < Self::N {
                    let delta = Coord::<W, H>::delta_down(pieces, gap);
                    new += u64::from(self.claim(index.wrapping_add(delta)));
                }
            }
        }

        new
    }

    /// Builds the table, one bit per state packed into bytes.
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

        while new != 0 {
            new = self.expand(depth);
            total += new;
            depth += 1;

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        let len = Coord::<W, H>::SIZE.div_ceil(8) as usize;
        self.bits
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .take(len)
            .collect()
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
        let len = Coord::<W, H>::SIZE.div_ceil(8) as usize;

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
