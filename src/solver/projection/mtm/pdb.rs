//! [`Mtm`]-specific implementation of the projection [`Pdb`].

use std::marker::PhantomData;

use crate::{
    algorithm::{direction::Direction, metric::Mtm},
    puzzle::{label::label::Label, size::Size},
    solver::{
        config::PdbConfig,
        indexing,
        projection::{
            pdb::{compute_solved_state, compute_tally, Pdb},
            puzzle::ProjectedPuzzle,
            LARGE, SMALL,
        },
        statistics::PdbIterationStats,
    },
};

impl Pdb<Mtm> {
    pub(super) fn new<L>(label: &L, size: Size, config: &PdbConfig) -> Self
    where
        L: Label,
    {
        if size.area() as usize <= SMALL {
            Self::new_impl::<SMALL, _>(label, size, config)
        } else {
            Self::new_impl::<LARGE, _>(label, size, config)
        }
    }

    fn new_impl<const N: usize, L>(label: &L, size: Size, config: &PdbConfig) -> Self
    where
        L: Label,
    {
        let solved_state = compute_solved_state(label, size);
        let tally = compute_tally(&solved_state);
        let gap = size.num_pieces() as u8;
        let width = size.width() as u8;
        let solved = ProjectedPuzzle::<N>::new(&solved_state, gap, width);
        let pdb_size = indexing::multinomial(&tally) as usize;

        let mut pdb = vec![u8::MAX; pdb_size];
        let solved_idx = solved.encode(&tally) as usize;
        pdb[solved_idx] = 0;

        let mut current = vec![solved];

        let mut depth = 0;
        let mut new = 1;
        let mut total = 1;

        if let Some(f) = &config.end_of_iter_callback {
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

            if let Some(f) = &config.end_of_iter_callback {
                f(PdbIterationStats { depth, new, total });
            }
        }

        Self {
            pdb: pdb.into_boxed_slice(),
            phantom_metric: PhantomData,
        }
    }
}
