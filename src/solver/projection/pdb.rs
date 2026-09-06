use std::marker::PhantomData;

use crate::{
    puzzle::{label::label::Label, size::Size},
    solver::projection::{encoding, puzzle::ProjectedPuzzle},
};

pub(super) struct Pdb<Metric> {
    pub(super) pdb: Box<[u8]>,
    pub(super) tally: Box<[u8]>,
    pub(super) solved_state: Box<[u8]>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

impl<Metric> Pdb<Metric> {
    pub(super) fn get(&self, index: usize) -> u8 {
        self.pdb[index]
    }

    pub(super) fn solved_state(&self) -> &[u8] {
        &self.solved_state
    }

    pub(super) fn encode<const W: usize, const H: usize, const N: usize>(
        &self,
        puzzle: &ProjectedPuzzle<W, H, N>,
    ) -> usize {
        encoding::encode(&puzzle.pieces, &self.tally) as usize
    }
}

pub(super) fn compute_solved_state<const W: usize, const H: usize, const N: usize, L>(
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

pub(super) fn compute_tally(solved_state: &[u8]) -> Vec<u8> {
    let max_label = *solved_state.iter().max().unwrap() as usize;
    let mut tally = vec![0u8; max_label + 1];
    for &l in solved_state {
        tally[l as usize] += 1;
    }
    tally
}
