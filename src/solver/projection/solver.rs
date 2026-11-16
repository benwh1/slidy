//! Projection [`Solver`] and its metric-specific implementations.
//!
//! The [`Solver`] struct and the implementation shared by all metrics live here; the
//! metric-specific implementations live in the `stm` and `mtm` submodules.

use std::marker::PhantomData;

use crate::{
    puzzle::{
        label::label::Label, size::Size, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState,
    },
    solver::{
        projection::{
            LARGE,
            builder::SolverBuilder,
            pdb::{Pdb, compute_solved_state, compute_tally},
            puzzle::{Moves, ProjectedPuzzle, project_puzzle},
        },
        search::Search,
    },
};

/// An iterative deepening solver that uses a pattern database of projected puzzle states as its
/// heuristic.
///
/// The solver is generic over the puzzle type `P`, which can be any [`SlidingPuzzle`]; the size
/// of the puzzle must match the [`Size`] provided to the [`builder`](Solver::builder).
pub struct Solver<P, Target, PruneTarget, Metric> {
    pub(super) pdb: Pdb<Metric>,
    pub(super) tally: Box<[u8]>,
    pub(super) search: Search<128>,
    pub(super) size: Size,
    pub(super) target: Target,
    prune_target: PruneTarget,
    pub(super) prune_target_solved_index: u64,
    pub(super) moves: Moves,
    /// The solved state projected onto `target`.
    target_solved: ProjectedPuzzle<LARGE>,
    /// The puzzle being solved projected onto `target`.
    target_start: ProjectedPuzzle<LARGE>,
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
    pub fn builder() -> SolverBuilder<P, Target, PruneTarget, Metric> {
        SolverBuilder::new()
    }

    pub(super) fn with_pdb(
        pdb: Pdb<Metric>,
        target: Target,
        prune_target: PruneTarget,
        size: Size,
    ) -> Self {
        let prune_target_solved_state = compute_solved_state(&prune_target, size);
        let tally = compute_tally(&prune_target_solved_state).into_boxed_slice();
        let solved_projected =
            ProjectedPuzzle::<LARGE>::new(&prune_target_solved_state, size.num_pieces() as u8);
        let prune_target_solved_index = solved_projected.encode(&tally);
        let target_solved = ProjectedPuzzle::new(
            &compute_solved_state(&target, size),
            size.num_pieces() as u8,
        );

        Self {
            pdb,
            moves: Moves::new(size, &tally),
            tally,
            search: Search::new(),
            size,
            prune_target_solved_index,
            target_solved,
            target_start: target_solved,
            target,
            prune_target,
            phantom_p: PhantomData,
            phantom_metric: PhantomData,
        }
    }

    /// Returns a reference to the inner [`Pdb`].
    pub fn pdb(&self) -> &Pdb<Metric> {
        &self.pdb
    }

    /// Returns `puzzle` projected onto `prune_target`, and remembers it projected onto `target`
    /// for [`Self::check_solution`].
    pub(super) fn initial_projected<const N: usize>(&mut self, puzzle: &P) -> ProjectedPuzzle<N> {
        self.target_start = project_puzzle::<LARGE, P, Target>(puzzle, &self.target);
        project_puzzle::<N, P, PruneTarget>(puzzle, &self.prune_target)
    }

    /// Returns whether the moves on the stack solve the puzzle passed to
    /// [`Self::initial_projected`] into `target`.
    pub(super) fn check_solution(&self) -> bool {
        // `target` is solved exactly when the pieces in each cell have the labels of the cell.
        let mut state = self.target_start;
        for dir in self.search.stack.iter() {
            state = self.moves.apply_dir(&state, dir);
        }
        state == self.target_solved
    }
}
