//! Stm-specific implementation of the projection [`Pdb`].
//!
//! [`Pdb`]: crate::solver::projection::pdb::Pdb

use std::{collections::HashSet, marker::PhantomData};

use crate::{
    algorithm::{direction::Direction, metric::Stm},
    puzzle::{label::label::Label, size::Size},
    solver::{
        projection::{
            encoding,
            pdb::{compute_solved_state, compute_tally, Pdb},
            puzzle::ProjectedPuzzle,
        },
        statistics::PdbIterationStats,
    },
};

impl Pdb<Stm> {
    pub(super) fn new<const W: usize, const H: usize, const N: usize, L>(
        label: &L,
        iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
    ) -> Self
    where
        L: Label,
    {
        let size = Size::new(W as u64, H as u64).unwrap();
        let solved_state = compute_solved_state::<W, H, N, L>(label, size);
        let tally = compute_tally(&solved_state);
        let pdb_size = encoding::multinomial(&tally) as usize;

        let mut pdb = vec![u8::MAX; pdb_size];
        let solved_gap = (N - 1) as u8;
        let solved_idx = encoding::encode_multiset(&solved_state, &tally) as usize;
        pdb[solved_idx] = 0;

        let mut visited: HashSet<(Vec<u8>, u8)> = HashSet::new();
        visited.insert((solved_state.clone(), solved_gap));

        let mut solved_arr = [0u8; N];
        solved_arr.copy_from_slice(&solved_state);
        let mut current: Vec<ProjectedPuzzle<W, H, N>> =
            vec![ProjectedPuzzle::new(solved_arr, solved_gap)];

        let mut depth = 0u8;
        let mut new = 1u64;
        let mut total = 1u64;

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
                    if puzzle.do_move(dir) {
                        let key = (puzzle.pieces.to_vec(), puzzle.gap);
                        if visited.insert(key) {
                            let idx = encoding::encode_multiset(&puzzle.pieces, &tally) as usize;
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
            solved_state: solved_state.into_boxed_slice(),
            phantom_metric: PhantomData,
        }
    }
}
