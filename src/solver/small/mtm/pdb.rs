use std::marker::PhantomData;

use crate::{
    algorithm::metric::Mtm,
    puzzle::small::{Puzzle, sealed::SmallPuzzle},
    solver::{
        config::PdbConfig,
        small::{coord::Dense, moves::Moves, pdb::Pdb},
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
struct Builder<const W: usize, const H: usize, const N: usize> {
    /// Two bits per state with [`Dense`] indices: `d % 3`, where `d` is the depth of the state, or
    /// 3 if the state hasn't been reached.
    entries: Vec<u8>,
}

impl<const W: usize, const H: usize, const N: usize> Builder<W, H, N> {
    fn new() -> Self {
        Self {
            entries: vec![u8::MAX; Dense::<W, H, N>::SIZE.div_ceil(4) as usize],
        }
    }

    /// Marks the state at `index` as reached at depth `depth`, returning whether it was not
    /// already reached.
    #[inline(always)]
    fn claim(&mut self, index: u64, depth: u8) -> bool {
        let byte = &mut self.entries[(index / 4) as usize];
        let shift = 2 * (index % 4);
        let new = (*byte >> shift) & 3 == 3;
        if new {
            // The entry is 3, so this sets it to `depth % 3`.
            *byte ^= (3 ^ (depth % 3)) << shift;
        }
        new
    }

    /// Expands the states in `queue`, which are at depth `depth`, returning the new states at
    /// depth `depth + 1`. The states are `(index, pieces, gap)`.
    fn push_queue(&mut self, depth: u8, queue: &[(u64, u64, u8)]) -> Vec<(u64, u64, u8)> {
        let mut next = Vec::with_capacity(queue.len() * 2);

        for &(index, pieces, gap) in queue {
            let mut rest = Moves::<W, H, N>::MOVES[gap as usize].1;
            while rest != 0 {
                let d = rest.trailing_zeros();
                rest &= rest - 1;

                // Move one more piece at a time, each a single move from `(index, pieces, gap)`.
                let (mut index, mut pieces, mut gap) = (index, pieces, gap);
                loop {
                    let (cells, moves) = Moves::<W, H, N>::MOVES[gap as usize];
                    if moves & (1 << d) == 0 {
                        break;
                    }
                    index = Moves::<W, H, N>::child_index(pieces, gap, index, d);
                    let cell = cells[d as usize];
                    pieces = Dense::<W, H, N>::move_gap(pieces, gap, cell);
                    gap = cell;
                    if self.claim(index, depth + 1) {
                        next.push((index, pieces, gap));
                    }
                }
            }
        }

        next
    }

    /// Builds the table, two bits per state packed into bytes, with [`Dense`] indices.
    fn build(mut self, config: &PdbConfig) -> Box<[u8]> {
        // The solved state has the gap in the last cell and the pieces in order, so all digits
        // except the gap's are 0.
        let solved = (1..N as u64).fold(0, |pieces, p| pieces | (p << (4 * (p - 1))));
        self.claim(N as u64 - 1, 0);
        let mut queue = vec![(N as u64 - 1, solved, N as u8 - 1)];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        loop {
            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
            if new == 0 {
                break;
            }

            queue = self.push_queue(depth, &queue);
            new = queue.len() as u64;
            total += new;
            depth += 1;
        }

        self.entries.into_boxed_slice()
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
            pdb: Builder::<W, H, N>::new().build(config),
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
