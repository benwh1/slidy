use std::marker::PhantomData;

use crate::{
    algorithm::direction::Direction,
    puzzle::{label::label::Label, size::Size},
    solver::{
        indexing,
        projection::puzzle::{FrontierState, ProjectedPuzzle},
        statistics::PdbIterationStats,
    },
};

pub(super) struct Pdb<Metric> {
    pub(super) pdb: Box<[u8]>,
    pub(super) tally: Box<[u8]>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

impl<Metric> Pdb<Metric> {
    pub(super) unsafe fn get_unchecked(&self, index: usize) -> u8 {
        debug_assert!(index < self.pdb.len());
        // SAFETY: `index` is an encode rank, which is always `0 .. pdb.len()` by construction.
        unsafe { *self.pdb.get_unchecked(index) }
    }

    pub(super) fn encode(&self, puzzle: &ProjectedPuzzle) -> usize {
        puzzle.encode(&self.tally) as usize
    }
}

pub(super) fn compute_solved_state<L>(label: &L, size: Size) -> Vec<u8>
where
    L: Label,
{
    let n = size.area() as usize;
    let mut state = vec![0; n];
    for (i, s) in state.iter_mut().enumerate() {
        let x = (i as u64) % size.width();
        let y = (i as u64) / size.width();
        *s = label.position_label(size, (x, y)) as u8 + 1;
    }
    state[n - 1] = 0;
    state
}

pub(super) fn compute_tally(solved_state: &[u8]) -> Vec<u8> {
    let max_label = *solved_state.iter().max().unwrap() as usize;
    let mut tally = vec![0; max_label + 1];
    for &l in solved_state {
        tally[l as usize] += 1;
    }
    tally
}

/// Breadth-first build of a PDB for the projection metric. The `pdb` array doubles as the visited
/// set: BFS reaches every rank at its minimal depth, so an entry that is still `u8::MAX` has not
/// been seen yet. `SLIDING` selects the move semantics: false visits each state reached by a
/// single step (stm), true visits every state passed while sliding for the wall (mtm).
pub(super) fn build_pdb<S, const SLIDING: bool>(
    solved: S,
    tally: &[u8],
    iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
) -> (Box<[u8]>, Vec<u8>)
where
    S: FrontierState,
{
    let pdb_size = indexing::multinomial(tally) as usize;

    let mut pdb = vec![u8::MAX; pdb_size];
    let solved_idx = solved.rank(tally) as usize;
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
                if SLIDING {
                    while puzzle.step(dir) {
                        let idx = puzzle.rank(tally) as usize;
                        if pdb[idx] == u8::MAX {
                            pdb[idx] = depth + 1;
                            next.push(puzzle);
                        }
                    }
                } else if puzzle.step(dir) {
                    let idx = puzzle.rank(tally) as usize;
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

    (pdb.into_boxed_slice(), tally.to_vec())
}
