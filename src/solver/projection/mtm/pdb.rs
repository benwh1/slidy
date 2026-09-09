//! [`Mtm`]-specific implementation of the projection PDB.

use std::marker::PhantomData;

use crate::{
    algorithm::{direction::Direction, metric::Mtm},
    puzzle::{label::label::Label, size::Size},
    solver::{
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
        let gap = size.num_pieces() as u8;
        let width = size.width() as u8;
        let area = size.area() as usize;

        let pdb = if area <= SMALL {
            bfs::<SMALL>(
                ProjectedPuzzle::<SMALL>::new(&solved_state, gap, width),
                &tally,
                iteration_callback,
            )
        } else {
            bfs::<LARGE>(
                ProjectedPuzzle::<LARGE>::new(&solved_state, gap, width),
                &tally,
                iteration_callback,
            )
        };

        Self {
            pdb,
            tally: tally.into_boxed_slice(),
            phantom_metric: PhantomData,
        }
    }
}

/// Breadth-first build of the mtm PDB. A move slides a tile as far as it can go, so every state
/// passed along the slide is visited. The `pdb` array doubles as the visited set: BFS reaches
/// every rank at its minimal depth, so an entry that is still `u8::MAX` has not been seen yet.
/// Projections of up to 16 cells use a `ProjectedPuzzle<16>`, halving the frontier's peak memory;
/// larger ones use the full `ProjectedPuzzle<32>`.
fn bfs<const N: usize>(
    solved: ProjectedPuzzle<N>,
    tally: &[u8],
    iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
) -> Box<[u8]> {
    let pdb_size = indexing::multinomial(tally) as usize;

    let mut pdb = vec![u8::MAX; pdb_size];
    let solved_idx = solved.encode(tally) as usize;
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
                    let idx = puzzle.encode(tally) as usize;
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

    pdb.into_boxed_slice()
}
