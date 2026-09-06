//! Stm-specific implementation of the projection [`SolverBuilder`].

use crate::{
    algorithm::metric::Stm,
    puzzle::{
        label::label::Label,
        small::{sealed::SmallPuzzle, Puzzle},
        solved_state::SolvedState,
    },
    solver::projection::{builder::SolverBuilder, pdb::Pdb, solver::Solver},
};

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget>
    SolverBuilder<'_, W, H, N, Target, PruneTarget, Stm>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    #[must_use]
    /// Builds the [`Solver`], using default values for parameters that weren't set.
    pub fn build(self) -> Solver<W, H, N, Target, PruneTarget, Stm> {
        let prune_target = self.prune_target.unwrap_or_default();
        let target = self.target.unwrap_or_default();
        let pdb = Pdb::new::<W, H, N, _>(&prune_target, self.pdb_iteration_callback);
        Solver::with_pdb(pdb, target, prune_target)
    }
}
