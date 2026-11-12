use crate::{
    algorithm::metric::Stm,
    puzzle::small::{Puzzle, sealed::SmallPuzzle},
    solver::{config::PdbConfig, small::pdb::Pdb},
};

const HASHES: [(usize, usize, u64); 12] = [
    (2, 2, 0x66b33be63e234245),
    (2, 3, 0x0f868220a4a06baf),
    (2, 4, 0x9b7a57ad2c6df83f),
    (2, 5, 0x4feabd468458775d),
    (2, 6, 0x84b6f795340a1b8a),
    (3, 2, 0x8275b13928b93c86),
    (3, 3, 0x8812534cd3f7d59f),
    (3, 4, 0x8bcdde83e8fb98b1),
    (4, 2, 0x0a649d41d893eae3),
    (4, 3, 0x835afc1a5551ae94),
    (5, 2, 0x44333aa439ea04fe),
    (6, 2, 0x05084fa633e32abf),
];

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
    /// Depending on the size of the puzzle, this may take several minutes to run.
    #[must_use]
    pub fn new(config: &PdbConfig) -> Self {
        Self::build::<false>(config)
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
        // SAFETY: The caller's requirements are the same.
        unsafe { Self::try_from_bytes_with_hashes(bytes, &HASHES) }
    }
}

#[cfg(test)]
mod tests {
    use xxhash_rust::xxh3;

    use super::*;
    use crate::solver::small::pdb::Pdb5x2Stm;

    #[test]
    fn test_5x2_tally() {
        let pdb = Pdb5x2Stm::default();
        let bytes = pdb.as_ref();

        let mut tally = [0u32; 64];
        for &b in bytes {
            tally[b as usize] += 1;
        }

        let expected = [
            1, 2, 3, 6, 11, 19, 30, 44, 68, 112, 176, 271, 411, 602, 851, 1232, 1783, 2530, 3567,
            4996, 6838, 9279, 12463, 16597, 21848, 28227, 35682, 44464, 54597, 65966, 78433, 91725,
            104896, 116966, 126335, 131998, 133107, 128720, 119332, 106335, 91545, 75742, 60119,
            45840, 33422, 23223, 15140, 9094, 5073, 2605, 1224, 528, 225, 75, 20, 2, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        assert_eq!(tally, expected);
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
            "{W}x{H} STM PDB hash mismatch, got {actual}, expected {expected}",
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
