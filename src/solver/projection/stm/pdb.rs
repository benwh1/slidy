//! Stm-specific implementation of the projection [`Pdb`].
//!
//! [`Pdb`]: crate::solver::projection::pdb::Pdb

use std::{collections::HashSet, marker::PhantomData};

use crate::{
    algorithm::{direction::Direction, metric::Stm},
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

impl Pdb<Stm> {
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
            ProjectedPuzzle::new(solved_state, (size.area() - 1) as u8, size.width() as u8);
        let solved_idx = solved.encode(&tally) as usize;
        pdb[solved_idx] = 0;

        let mut visited = HashSet::new();
        visited.insert(solved_idx);

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
                    let mut puzzle = state.clone();
                    if puzzle.do_move(dir) {
                        let idx = puzzle.encode(&tally) as usize;
                        if visited.insert(idx) {
                            if pdb[idx] == u8::MAX {
                                pdb[idx] = depth + 1;
                            }
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
