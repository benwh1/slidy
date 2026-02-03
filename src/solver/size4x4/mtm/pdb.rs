use crate::{
    algorithm::direction::Direction,
    solver::{
        pdb_iteration::PdbIterationStats,
        size4x4::mtm::{
            base_5_table::Base5Table, consts::SIZE, indexing, indexing_table::IndexingTable,
            puzzle::ReducedFourBitPuzzle,
        },
    },
};

pub(super) struct Pdb {
    pdb: Box<[u8]>,
}

impl Pdb {
    pub(super) fn new(
        indexing_table: &IndexingTable,
        base_5_table: &Base5Table,
        end_of_iteration: Option<&dyn Fn(PdbIterationStats)>,
    ) -> Self {
        let mut pdb = vec![u8::MAX; SIZE];

        let puzzle = ReducedFourBitPuzzle::new();
        let solved_index = indexing_table.encode(puzzle.pieces, base_5_table) as usize;
        pdb[solved_index] = 0;

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = end_of_iteration {
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

            if let Some(f) = end_of_iteration {
                f(PdbIterationStats { depth, new, total });
            }
        }

        let pdb = pdb.into_boxed_slice();

        Self { pdb }
    }

    pub(crate) fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    pub(super) unsafe fn get_unchecked(&self, index: usize) -> u8 {
        *self.pdb.get_unchecked(index)
    }
}
