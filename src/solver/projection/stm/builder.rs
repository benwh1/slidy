//! Stm-specific implementation of the projection [`SolverBuilder`].

use crate::{
    algorithm::metric::Stm,
    puzzle::{
        label::label::Label,
        size::Size,
        small::{sealed::SmallPuzzle, Puzzle},
        solved_state::SolvedState,
    },
    solver::projection::{
        builder::{ProjectionError, SolverBuilder},
        pdb::Pdb,
        solver::Solver,
    },
};

impl<const W: usize, const H: usize, const N: usize, Target, PruneTarget>
    SolverBuilder<'_, W, H, N, Target, PruneTarget, Stm>
where
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
    Puzzle<W, H>: SmallPuzzle<PieceArray = [u8; N]>,
{
    /// Builds the [`Solver`], using default values for parameters that weren't set.
    ///
    /// Returns a [`ProjectionError::InvalidProjection`] if the pruning label is not a projection
    /// of the target label.
    pub fn build(self) -> Result<Solver<W, H, N, Target, PruneTarget, Stm>, ProjectionError> {
        let prune_target = self.prune_target.unwrap_or_default();
        let target = self.target.unwrap_or_default();
        let size = Size::new(W as u64, H as u64).unwrap();

        if !prune_target.is_projection_of(size, &target) {
            return Err(ProjectionError::InvalidProjection);
        }

        let pdb = Pdb::new::<W, H, N, _>(&prune_target, self.pdb_iteration_callback);
        Ok(Solver::with_pdb(pdb, target, prune_target))
    }
}
