//! Mtm-specific implementation of the projection PDB.

use std::marker::PhantomData;

use crate::{
    algorithm::metric::Mtm,
    puzzle::{label::label::Label, size::Size},
    solver::{
        projection::{
            encoding,
            pdb::{build_pdb, compute_solved_state, compute_tally, Pdb},
            puzzle::{CompactProjectedPuzzle, ProjectedPuzzle},
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
        let gap = (size.area() - 1) as u8;
        let width = size.width() as u8;

        let (pdb, tally) = if solved_state.len() <= encoding::SMALL {
            let solved = CompactProjectedPuzzle::new(&solved_state, gap, width);
            build_pdb::<CompactProjectedPuzzle, true>(solved, &tally, iteration_callback)
        } else {
            let solved = ProjectedPuzzle::new(&solved_state, gap, width);
            build_pdb::<ProjectedPuzzle, true>(solved, &tally, iteration_callback)
        };

        Self {
            pdb,
            tally: tally.into_boxed_slice(),
            phantom_metric: PhantomData,
        }
    }
}
