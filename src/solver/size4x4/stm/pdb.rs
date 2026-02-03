use crate::{
    algorithm::direction::Direction,
    solver::{
        pdb_iteration::PdbIterationStats,
        size4x4::stm::{
            pattern::Pattern,
            puzzle::{MoveResult, Puzzle},
        },
    },
};

pub(super) struct Pdb {
    pattern: Pattern,
    transposition_table: Vec<[u32; 4]>,
    pdb: Vec<u8>,
}

impl Pdb {
    pub(super) fn new(
        pattern: Pattern,
        pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
    ) -> Self {
        let mut this = Self {
            pattern,
            transposition_table: Vec::new(),
            pdb: Vec::new(),
        };

        this.make_transposition_table();
        this.make_pdb(pdb_iteration_callback);

        this
    }

    fn make_transposition_table(&mut self) {
        let size = self.pattern.pdb_size();
        self.transposition_table = vec![[0; 4]; size];

        let mut puzzle = Puzzle::new();

        for i in 0..size {
            puzzle.decode(i, &self.pattern);

            let mut moves = [0; 4];

            for mv in [
                Direction::Up,
                Direction::Left,
                Direction::Down,
                Direction::Right,
            ] {
                match puzzle.do_move(mv) {
                    MoveResult::MovedPiece => {
                        let index = puzzle.encode(&self.pattern) | (1 << 24);
                        moves[mv as usize] = index as u32;
                        puzzle.do_move(mv.inverse());
                    }
                    MoveResult::MovedIgnoredPiece => {
                        let index = puzzle.encode(&self.pattern);
                        moves[mv as usize] = index as u32;
                        puzzle.do_move(mv.inverse());
                    }
                    MoveResult::CantMove => {
                        moves[mv as usize] = u32::MAX;
                    }
                }
            }

            self.transposition_table[i] = moves;
        }
    }

    fn pdb_bfs_pass(&mut self, depth: u8, base_depth: u8) -> u64 {
        let size = self.pattern.pdb_size();

        let mut new = 0;

        for i in 0..size {
            if self.pdb[i] == u8::MAX || self.pdb[i] < base_depth {
                continue;
            }

            let new_indexes = self.transposition_table[i];

            for dir in [
                Direction::Up,
                Direction::Left,
                Direction::Down,
                Direction::Right,
            ] {
                let entry = new_indexes[dir as usize];
                let new_index = entry & 0xffffff;
                let moved_piece = (entry >> 24) as u8;

                if entry != u32::MAX
                    && self.pdb[new_index as usize] == u8::MAX
                    && self.pdb[i] + moved_piece == depth
                {
                    self.pdb[new_index as usize] = depth;
                    new += 1;
                }
            }
        }

        new
    }

    fn make_pdb(&mut self, pdb_iteration_callback: Option<&dyn Fn(PdbIterationStats)>) {
        let size = self.pattern.pdb_size();
        self.pdb = vec![u8::MAX; size];

        self.pdb[Puzzle::new().encode(&self.pattern)] = 0;

        let mut depth = 0;
        let mut new;
        let mut total = 1;

        while total < size as u64 {
            new = 0;

            if depth != 0 {
                new = self.pdb_bfs_pass(depth, depth - 1);
            }

            loop {
                let n = self.pdb_bfs_pass(depth, depth);
                if n == 0 {
                    break;
                }
                new += n;
            }

            depth += 1;
            total += new;

            if let Some(callback) = &pdb_iteration_callback {
                callback(PdbIterationStats { depth, new, total });
            }
        }

        // Clean up transposition table
        for i in &mut self.transposition_table {
            for j in i {
                if *j != u32::MAX {
                    *j &= 0xffffff;
                }
            }
        }
    }

    pub(super) fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub(super) fn transposition_table(&self) -> &[[u32; 4]] {
        &self.transposition_table
    }

    pub(super) fn pdb(&self) -> &[u8] {
        &self.pdb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdb4_size() {
        let pattern = Pattern::new(&[1, 2, 5, 6, 0]);
        let pdb = Pdb::new(pattern, None);

        assert_eq!(pdb.transposition_table.len(), 524160);
        assert_eq!(pdb.pdb.len(), 524160);
    }

    #[test]
    fn test_pdb3_size() {
        let pattern = Pattern::new(&[11, 12, 15, 0]);
        let pdb = Pdb::new(pattern, None);

        assert_eq!(pdb.transposition_table.len(), 43680);
        assert_eq!(pdb.pdb.len(), 43680);
    }

    #[test]
    fn test_transposition_table_pdb4() {
        let pattern = Pattern::new(&[1, 2, 5, 6, 0]);
        let pdb = Pdb::new(pattern, None);

        for arr in pdb.transposition_table {
            for entry in arr {
                assert!(entry == u32::MAX || entry < 524160);
            }
        }
    }

    #[test]
    fn test_transposition_table_pdb3() {
        let pattern = Pattern::new(&[11, 12, 15, 0]);
        let pdb = Pdb::new(pattern, None);

        for arr in pdb.transposition_table {
            for entry in arr {
                assert!(entry == u32::MAX || entry < 43680);
            }
        }
    }
}
