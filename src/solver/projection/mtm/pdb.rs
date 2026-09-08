//! Mtm-specific implementation of the projection PDB.

use std::marker::PhantomData;

use crate::{
    algorithm::{direction::Direction, metric::Mtm},
    puzzle::{label::label::Label, size::Size},
    solver::{
        indexing,
        projection::{
            pdb::{compute_solved_state, compute_tally, Pdb},
            puzzle::ProjectedPuzzle,
        },
        statistics::PdbIterationStats,
    },
};

impl Pdb<Mtm> {
    pub(super) fn new<L>(
        label: &L,
        size: Size,
        iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
    ) -> Self
    where
        L: Label,
    {
        let solved_state = compute_solved_state(label, size);
        let tally = compute_tally(&solved_state);
        let pdb_size = indexing::multinomial(&tally) as usize;

        let mut pdb = vec![u8::MAX; pdb_size];
        let solved =
            ProjectedPuzzle::new(&solved_state, (size.area() - 1) as u8, size.width() as u8);
        let solved_idx = solved.encode(&tally) as usize;
        pdb[solved_idx] = 0;

        let mut current = vec![solved];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = iteration_callback {
            f(PdbIterationStats { depth, new, total });
        }

        while !current.is_empty() {
            let mut next = Vec::with_capacity(current.len() * 4);

            for state in &current {
                for dir in [
                    Direction::Up,
                    Direction::Left,
                    Direction::Down,
                    Direction::Right,
                ] {
                    let mut puzzle = *state;
                    while puzzle.do_move(dir) {
                        let idx = puzzle.encode(&tally) as usize;
                        // `pdb` doubles as the visited set: BFS reaches every rank at its minimal
                        // depth, so an entry that is still `u8::MAX` has not been seen yet.
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

        Self {
            pdb: pdb.into_boxed_slice(),
            tally: tally.into_boxed_slice(),
            phantom_metric: PhantomData,
        }
    }
}
