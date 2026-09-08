//! Stm-specific implementation of the projection [`SolverBuilder`].

use crate::{
    algorithm::metric::Stm,
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::projection::{
        builder::{ProjectionError, SolverBuilder},
        pdb::Pdb,
        solver::Solver,
    },
};

impl<P, Target, PruneTarget> SolverBuilder<'_, P, Target, PruneTarget, Stm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    /// Builds the [`Solver`], using default values for parameters that weren't set.
    ///
    /// Returns a [`ProjectionError::MissingSize`] if no size was provided, or a
    /// [`ProjectionError::InvalidProjection`] if the pruning label is not a projection of the
    /// target label.
    pub fn build(self) -> Result<Solver<P, Target, PruneTarget, Stm>, ProjectionError> {
        let (target, prune_target, callback, size) = self.build_projecting()?;
        let pdb = Pdb::<Stm>::new(&prune_target, size, callback);
        Ok(Solver::with_pdb(pdb, target, prune_target, size))
    }
}
