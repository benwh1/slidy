use std::marker::PhantomData;

use crate::{
    puzzle::{label::label::Label, size::Size},
    solver::projection::puzzle::ProjectedPuzzle,
};

pub(super) struct Pdb<Metric> {
    pub(super) pdb: Box<[u8]>,
    pub(super) tally: Box<[u8]>,
    pub(super) phantom_metric: PhantomData<Metric>,
}

impl<Metric> Pdb<Metric> {
    pub(super) fn get(&self, index: usize) -> u8 {
        // `index` is an encode rank, which is always `0 .. pdb.len()` by construction.
        debug_assert!(index < self.pdb.len());
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
