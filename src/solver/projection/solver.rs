//! Projection [`Solver`] and its metric-specific implementations.
//!
//! The [`Solver`] struct and the implementation shared by all metrics live here; the
//! metric-specific implementations live in the [`stm`] and [`mtm`] submodules.

use std::{
    cell::{Cell, Ref, RefCell},
    marker::PhantomData,
};

use super::builder::SolverBuilder;
use crate::{
    puzzle::{
        label::label::Label,
        sliding_puzzle::SlidingPuzzle as _,
        small::{sealed::SmallPuzzle, Puzzle},
        solved_state::SolvedState,
    },
    solver::{
        projection::{
            pdb::Pdb,
            puzzle::{project_puzzle, ProjectedPuzzle},
        },
        solver::SolverConfig,
        stack::Stack,
    },
};

pub struct Solver<const W: usize, const H: usize, const N: usize, Target, PruneTarget, Metric> {
    pub(super) pdb: Pdb<Metric>,
    pub(super) stack: Stack<128>,
    pub(super) puzzle: Cell<Puzzle<W, H>>,
    pub(super) solutions_found: Cell<u64>,
    pub(super) config: RefCell<Option<SolverConfig>>,
    target: Target,
    prune_target: PruneTarget,
    phantom_metric: PhantomData<Metric>,
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget, MetricTag>
    Solver<W, H, N, Target, PruneTarget, MetricTag>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    #[must_use]
    pub fn builder<'a>() -> SolverBuilder<'a, W, H, N, Target, PruneTarget, MetricTag> {
        SolverBuilder::new()
    }

    pub(super) fn cfg(&self) -> Ref<'_, SolverConfig> {
        Ref::map(self.config.borrow(), |b| b.as_ref().unwrap())
    }

    pub(super) fn with_pdb(pdb: Pdb<MetricTag>, target: Target, prune_target: PruneTarget) -> Self {
        Self {
            pdb,
            stack: Stack::default(),
            puzzle: Cell::new(Puzzle::<W, H>::new()),
            solutions_found: Cell::new(0),
            config: RefCell::new(None),
            target,
            prune_target,
            phantom_metric: PhantomData,
        }
    }

    pub(super) fn initial_projected(&self) -> ProjectedPuzzle<W, H, N> {
        project_puzzle::<W, H, N, Puzzle<W, H>, PruneTarget>(&self.puzzle.get(), &self.prune_target)
    }

    pub(super) fn solved_state_arr(&self) -> [u8; N] {
        let mut arr = [0u8; N];
        arr.copy_from_slice(self.pdb.solved_state());
        arr
    }

    pub(super) fn check_solution(&self) -> bool {
        let mut p = self.puzzle.get();
        for dir in self.stack.iter() {
            p.try_move_dir(dir);
        }
        self.target.is_solved(&p)
    }
}
