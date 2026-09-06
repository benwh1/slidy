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
        size::Size,
        sliding_puzzle::SlidingPuzzle,
        small::{sealed::SmallPuzzle, Puzzle},
        solved_state::SolvedState,
    },
    solver::{
        projection::{
            pdb::{compute_solved_state, Pdb},
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
    pub(super) prune_target_solved_state: [u8; N],
    phantom_metric: PhantomData<Metric>,
}

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget, Metric>
    Solver<W, H, N, Target, PruneTarget, Metric>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    #[must_use]
    pub fn builder<'a>() -> SolverBuilder<'a, W, H, N, Target, PruneTarget, Metric> {
        SolverBuilder::new()
    }

    pub(super) fn cfg(&self) -> Ref<'_, SolverConfig> {
        Ref::map(self.config.borrow(), |b| b.as_ref().unwrap())
    }

    pub(super) fn with_pdb(pdb: Pdb<Metric>, target: Target, prune_target: PruneTarget) -> Self {
        let size = Size::new(W as u64, H as u64).unwrap();
        let prune_target_solved_state = compute_solved_state::<W, H, N, _>(&prune_target, size);

        Self {
            pdb,
            stack: Stack::default(),
            puzzle: Cell::new(Puzzle::<W, H>::new()),
            prune_target_solved_state,
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

    pub(super) fn check_solution(&self) -> bool {
        let mut p = self.puzzle.get();
        for dir in self.stack.iter() {
            p.try_move_dir(dir);
        }
        self.target.is_solved(&p)
    }
}
