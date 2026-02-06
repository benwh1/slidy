use xxhash_rust::xxh3;

use crate::{
    algorithm::direction::Direction,
    solver::{
        size4x4::mtm::{
            base_5_table::Base5Table, consts::SIZE, indexing, indexing_table::IndexingTable,
            puzzle::ReducedFourBitPuzzle,
        },
        statistics::PdbIterationStats,
    },
};

const HASH: u64 = 0x73b712151249d829;

pub(super) struct Pdb {
    pdb: Box<[u8]>,
}

impl Pdb {
    pub(super) fn new(
        indexing_table: &IndexingTable,
        base_5_table: &Base5Table,
        iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
    ) -> Self {
        let mut pdb = vec![u8::MAX; SIZE];

        let puzzle = ReducedFourBitPuzzle::new();
        let solved_index = indexing_table.encode(puzzle.pieces, base_5_table) as usize;
        pdb[solved_index] = 0;

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = iteration_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while new != 0 {
            new = 0;

            for i in 0..SIZE {
                if pdb[i] != depth {
                    continue;
                }

                for mv in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let piece_array = indexing::decode_multiset_16(i as u64);

                    // SAFETY: `decode_multiset_16` returns a valid permutation of the required
                    // nibbles.
                    let mut puzzle =
                        unsafe { ReducedFourBitPuzzle::from_piece_array_unchecked(piece_array) };

                    while puzzle.do_move(mv) {
                        let idx = indexing_table.encode(puzzle.pieces(), base_5_table) as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            new += 1;
                        }
                    }
                }
            }

            total += new;
            depth += 1;

            if let Some(f) = iteration_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        let pdb = pdb.into_boxed_slice();

        Self { pdb }
    }

    /// See [`crate::solver::small::pdb::Pdb::try_from_bytes`].
    pub(super) unsafe fn try_from_bytes(bytes: Box<[u8]>) -> Option<Self> {
        if bytes.len() != SIZE {
            return None;
        }

        let expected_hash = HASH;
        let actual_hash = xxh3::xxh3_64(&bytes);

        if actual_hash != expected_hash {
            return None;
        }

        // SAFETY: We checked above that the data is (almost certainly) correct.
        Some(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    /// See [`crate::solver::small::pdb::Pdb::from_bytes_unchecked`].
    pub(super) unsafe fn from_bytes_unchecked(bytes: Box<[u8]>) -> Self {
        Self { pdb: bytes }
    }

    pub(super) fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    pub(super) unsafe fn get_unchecked(&self, index: usize) -> u8 {
        *self.pdb.get_unchecked(index)
    }
}

impl AsRef<[u8]> for Pdb {
    fn as_ref(&self) -> &[u8] {
        &self.pdb
    }
}
