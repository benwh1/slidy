use xxhash_rust::xxh3;

use crate::{
    algorithm::direction::Direction,
    solver::{
        size4x4::mtm::{
            base_5_table::Base5Table, consts::SIZE, indexing_table::IndexingTable,
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

        let mut current = vec![puzzle];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = iteration_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while !current.is_empty() {
            let mut next = Vec::with_capacity(current.len() * 2);

            for state in current {
                for mv in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = state;

                    while puzzle.do_move(mv) {
                        let idx = indexing_table.encode(puzzle.pieces(), base_5_table) as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            next.push(puzzle);
                        }
                    }
                }
            }

            new = next.len() as u64;
            total += new;
            depth += 1;

            current = next;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb() {
        let indexing_table = IndexingTable::new();
        let base_5_table = Base5Table::new();
        let pdb = Pdb::new(&indexing_table, &base_5_table, None);
        let hash = xxh3::xxh3_64(pdb.as_ref());
        assert_eq!(hash, HASH);
    }
}
