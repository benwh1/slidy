//! [`Stm`]-specific implementation of the projection [`SolverBuilder`].

use crate::{
    algorithm::metric::Stm,
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::projection::{
        builder::{ProjectionError, SolverBuilder},
        pdb::Pdb,
        solver::Solver,
    },
};

impl<P, Target, PruneTarget> SolverBuilder<P, Target, PruneTarget, Stm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    /// Builds the [`Solver`], using default values for parameters that weren't set.
    ///
    /// See [`ProjectionError`] for possible errors.
    pub fn build(self) -> Result<Solver<P, Target, PruneTarget, Stm>, ProjectionError> {
        let (target, prune_target, pdb_config, size) = self.build_projecting()?;
        let pdb = Pdb::<Stm>::new(&prune_target, size, &pdb_config);
        Ok(Solver::with_pdb(pdb, target, prune_target, size))
    }
}
