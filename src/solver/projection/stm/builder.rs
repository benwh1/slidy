//! [`Stm`]-specific implementation of the projection [`SolverBuilder`].

use crate::{
    algorithm::metric::Stm,
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::projection::{
        builder::{PdbAction, SolverBuilder, SolverBuilderError},
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
    /// See [`SolverBuilderError`] for possible errors.
    pub fn build(self) -> Result<Solver<P, Target, PruneTarget, Stm>, SolverBuilderError> {
        let prune_target = self.prune_target.unwrap_or_default();
        let target = self.target.unwrap_or_default();
        let size = self.size.ok_or(SolverBuilderError::MissingSize)?;

        if !prune_target.is_projection_of(size, &target) {
            return Err(SolverBuilderError::InvalidProjection);
        }

        let pdb = match self.pdb_action {
            PdbAction::Build { config } => Pdb::<Stm>::new(&prune_target, size, &config),
            PdbAction::UseExisting { pdb } => pdb,
        };

        Ok(Solver::with_pdb(pdb, target, prune_target, size))
    }
}
