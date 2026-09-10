//! [`Mtm`]-specific implementation of the projection [`SolverBuilder`].

use crate::{
    algorithm::metric::Mtm,
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::projection::{
        builder::{ProjectionError, SolverBuilder},
        pdb::Pdb,
        solver::Solver,
    },
};

impl<P, Target, PruneTarget> SolverBuilder<'_, P, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    /// Builds the [`Solver`].
    ///
    /// See [`ProjectionError`] for possible errors.
    pub fn build(self) -> Result<Solver<P, Target, PruneTarget, Mtm>, ProjectionError> {
        let (target, prune_target, callback, size) = self.build_projecting()?;
        let pdb = Pdb::<Mtm>::new(&prune_target, size, callback);
        Ok(Solver::with_pdb(pdb, target, prune_target, size))
    }
}
