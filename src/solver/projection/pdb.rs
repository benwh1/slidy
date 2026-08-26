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
    n: usize,
}

impl Pdb {
    pub(super) fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    pub(super) fn solved_state(&self) -> &[u8] {
        &self.solved_state
    }

    pub(super) fn new_stm<const W: usize, const H: usize, const N: usize, L: Label>(
        label: &L,
        iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
    ) -> Self {
        let size = Size::new(W as u64, H as u64).unwrap();
        let solved_state = compute_solved_state::<W, H, N, L>(label, size);
        let tally = compute_tally::<W, H, N, L>(label, size);
        let pdb_size = encoding::pdb_size_with_gap(&tally, N) as usize;

        let mut pdb = vec![u8::MAX; pdb_size];
        let solved_gap = (N - 1) as u8;
        let solved_idx = encoding::encode_multiset(&solved_state, &tally) as usize * N
            + solved_gap as usize;
        pdb[solved_idx] = 0;

        let mut solved_arr = [0u8; N];
        solved_arr.copy_from_slice(&solved_state);
        let mut current: Vec<ProjectedPuzzle<N>> =
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
                    if puzzle.do_move::<W, H>(dir) {
                        let idx = encoding::encode_multiset(&puzzle.pieces, &tally) as usize * N
                            + puzzle.gap as usize;
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
            tally,
            solved_state,
            n: N,
        }
    }

    pub(super) fn new_mtm<const W: usize, const H: usize, const N: usize, L: Label>(
        label: &L,
        iteration_callback: Option<&dyn Fn(PdbIterationStats)>,
    ) -> Self {
        let size = Size::new(W as u64, H as u64).unwrap();
        let solved_state = compute_solved_state::<W, H, N, L>(label, size);
        let tally = compute_tally::<W, H, N, L>(label, size);
        let pdb_size = encoding::pdb_size_with_gap(&tally, N) as usize;

        let mut pdb = vec![u8::MAX; pdb_size];
        let solved_gap = (N - 1) as u8;
        let solved_idx = encoding::encode_multiset(&solved_state, &tally) as usize * N
            + solved_gap as usize;
        pdb[solved_idx] = 0;

        let mut solved_arr = [0u8; N];
        solved_arr.copy_from_slice(&solved_state);
        let mut current: Vec<ProjectedPuzzle<N>> =
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
                    while puzzle.do_move::<W, H>(dir) {
                        let idx = encoding::encode_multiset(&puzzle.pieces, &tally) as usize * N
                            + puzzle.gap as usize;
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
            tally,
            solved_state,
            n: N,
        }
    }

    pub(super) fn encode<const N: usize>(&self, puzzle: &ProjectedPuzzle<N>) -> usize {
        encoding::encode_multiset(&puzzle.pieces, &self.tally) as usize * self.n
            + puzzle.gap as usize
    }
}

fn compute_solved_state<const W: usize, const H: usize, const N: usize, L: Label>(
    label: &L,
    size: Size,
) -> Vec<u8> {
    let mut state = vec![0u8; N];
    for i in 0..N {
        let x = (i % W) as u64;
        let y = (i / W) as u64;
        state[i] = label.position_label(size, (x, y)) as u8;
    }
    state
}

fn compute_tally<const W: usize, const H: usize, const N: usize, L: Label>(
    label: &L,
    size: Size,
) -> Vec<u8> {
    let num_labels = label.num_labels(size) as usize;
    let mut tally = vec![0u8; num_labels];
    let solved_state = compute_solved_state::<W, H, N, L>(label, size);
    for &l in &solved_state {
        tally[l as usize] += 1;
    }
    tally
}
