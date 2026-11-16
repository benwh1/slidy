use crate::{
    algorithm::metric::Mtm,
    puzzle::{
        sliding_puzzle::SlidingPuzzle as _,
        small::{Puzzle, sealed::SmallPuzzle},
    },
    solver::{config::PdbConfig, small::pdb::Pdb},
};

const HASHES: [(usize, usize, u64); 12] = [
    (2, 2, 0x66b33be63e234245),
    (2, 3, 0xa001670b2c0432ab),
    (2, 4, 0x64e76678662d6c49),
    (2, 5, 0xf7e3bbdb27bc8066),
    (2, 6, 0x1221756582872833),
    (3, 2, 0xfb4f384ea556c974),
    (3, 3, 0x2bc75b60a3361302),
    (3, 4, 0x61152679ea24a66a),
    (4, 2, 0x4e6df36030daed02),
    (4, 3, 0x4e91c8da54abdff8),
    (5, 2, 0xe6f36e4ac7284ada),
    (6, 2, 0xa824375d41fb2487),
];

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
        Self::build_mtm(config)
    }

    /// Returns the optimal solution length of the state at `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    /// Returns the optimal solution length of the state at `index`, without bounds checking.
    ///
    /// # Safety
    ///
    /// `index` must be a valid index into the PDB.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> u8 {
        // SAFETY: The caller guarantees that `index` is in bounds.
        unsafe { *self.pdb.get_unchecked(index) }
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
        let len = Puzzle::<W, H>::new()
            .size()
            .num_states()
            .and_then(|n| n.try_into().ok())?;

        // SAFETY: The caller's requirements are the same.
        unsafe { Self::try_from_bytes_with_hashes(bytes, len, &HASHES) }
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
        let _ = Pdb::<W, H, N, Mtm>::new(&config);
        let mut tally = tally.lock().unwrap().clone();
        assert_eq!(tally.pop(), Some(0));
        tally
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
            "{W}x{H} MTM PDB hash mismatch, got {actual}, expected {expected}",
        );
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
