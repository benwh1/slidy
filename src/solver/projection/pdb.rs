use std::collections::HashSet;

use crate::{
    algorithm::direction::Direction,
    puzzle::{label::label::Label, size::Size},
    solver::{
        projection::{encoding, puzzle::ProjectedPuzzle},
        statistics::PdbIterationStats,
    },
};

pub(super) struct Pdb {
    pdb: Box<[u8]>,
    tally: Vec<u8>,
    solved_state: Vec<u8>,
}

impl Pdb {
    pub(super) fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    pub(super) fn solved_state(&self) -> &[u8] {
        &self.solved_state
    }

    pub(super) fn new_stm<const W: usize, const H: usize, const N: usize, L>(
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
            tally,
            solved_state,
        }
    }

    pub(super) fn new_mtm<const W: usize, const H: usize, const N: usize, L>(
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
                    while puzzle.do_move(dir) {
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
            tally,
            solved_state,
        }
    }

    pub(super) fn encode<const W: usize, const H: usize, const N: usize>(
        &self,
        puzzle: &ProjectedPuzzle<W, H, N>,
    ) -> usize {
        encoding::encode_multiset(&puzzle.pieces, &self.tally) as usize
    }
}

fn compute_solved_state<const W: usize, const H: usize, const N: usize, L>(
    label: &L,
    size: Size,
) -> Vec<u8>
where
    L: Label,
{
    let mut state = vec![0u8; N];
    for (i, s) in state.iter_mut().enumerate() {
        let x = (i % W) as u64;
        let y = (i / W) as u64;
        *s = label.position_label(size, (x, y)) as u8 + 1;
    }
    state[N - 1] = 0;
    state
}

fn compute_tally(solved_state: &[u8]) -> Vec<u8> {
    let max_label = *solved_state.iter().max().unwrap() as usize;
    let mut tally = vec![0u8; max_label + 1];
    for &l in solved_state {
        tally[l as usize] += 1;
    }
    tally
}
