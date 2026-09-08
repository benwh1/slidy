//! Projection [`Solver`] and its metric-specific implementations.
//!
//! The [`Solver`] struct and the implementation shared by all metrics live here; the
//! metric-specific implementations live in the `stm` and `mtm` submodules.

use std::{
    cell::{Cell, Ref, RefCell},
    marker::PhantomData,
};

use crate::{
    puzzle::{
        label::label::Label, size::Size, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState,
    },
    solver::{
        projection::{
            builder::SolverBuilder,
            pdb::{compute_solved_state, Pdb},
            puzzle::{project_puzzle, ProjectedPuzzle},
        },
        solver::SolverConfig,
        stack::Stack,
    },
};

/// An iterative deepening solver that uses a pattern database of projected puzzle states as its
/// heuristic.
///
/// The solver is generic over the puzzle type `P`, which can be any [`SlidingPuzzle`]; the size
/// of the puzzle must match the [`Size`] provided to the [`builder`](Solver::builder).
pub struct Solver<P, Target, PruneTarget, Metric> {
    pub(super) pdb: Pdb<Metric>,
    pub(super) stack: Stack<128>,
    pub(super) size: Size,
    pub(super) solutions_found: Cell<u64>,
    pub(super) config: RefCell<Option<SolverConfig>>,
    target: Target,
    prune_target: PruneTarget,
    pub(super) prune_target_solved_idx: usize,
    phantom_p: PhantomData<P>,
    phantom_metric: PhantomData<Metric>,
}

impl<P, Target, PruneTarget, Metric> Solver<P, Target, PruneTarget, Metric>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    #[must_use]
    /// Creates a [`SolverBuilder`] for constructing a [`Solver`].
    pub fn builder<'a>() -> SolverBuilder<'a, P, Target, PruneTarget, Metric> {
        SolverBuilder::new()
    }

    pub(super) fn cfg(&self) -> Ref<'_, SolverConfig> {
        Ref::map(self.config.borrow(), |b| b.as_ref().unwrap())
    }

    pub(super) fn with_pdb(
        pdb: Pdb<Metric>,
        target: Target,
        prune_target: PruneTarget,
        size: Size,
    ) -> Self {
        let prune_target_solved_state = compute_solved_state(&prune_target, size);
        let solved_projected = ProjectedPuzzle::new(
            &prune_target_solved_state,
            (size.area() - 1) as u8,
            size.width() as u8,
        );
        let prune_target_solved_idx = solved_projected.encode(&pdb.tally) as usize;

        Self {
            pdb,
            stack: Stack::default(),
            size,
            prune_target_solved_idx,
            solutions_found: Cell::new(0),
            config: RefCell::new(None),
            target,
            prune_target,
            phantom_p: PhantomData,
            phantom_metric: PhantomData,
        }
    }

    pub(super) fn initial_projected(&self, puzzle: &P) -> ProjectedPuzzle {
        project_puzzle(puzzle, &self.prune_target)
    }

    pub(super) fn check_solution(&self, puzzle: &P) -> bool {
        let mut p = puzzle.clone();
        for dir in self.stack.iter() {
            p.try_move_dir(dir);
        }
        self.target.is_solved(&p)
    }
}
