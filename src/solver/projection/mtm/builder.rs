//! [`Mtm`]-specific implementation of the projection [`SolverBuilder`].

use crate::{
    algorithm::metric::Mtm,
    puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle, solved_state::SolvedState},
    solver::projection::{
        builder::{PdbAction, ProjectionError, SolverBuilder},
        pdb::Pdb,
        solver::Solver,
    },
};

impl<P, Target, PruneTarget> SolverBuilder<P, Target, PruneTarget, Mtm>
where
    P: SlidingPuzzle + Clone,
    Target: Label + SolvedState + Default,
    PruneTarget: Label + SolvedState + Default,
{
    /// Builds the [`Solver`].
    ///
    /// See [`ProjectionError`] for possible errors.
    pub fn build(self) -> Result<Solver<P, Target, PruneTarget, Mtm>, ProjectionError> {
        let prune_target = self.prune_target.unwrap_or_default();
        let target = self.target.unwrap_or_default();
        let size = self.size.ok_or(ProjectionError::MissingSize)?;

        if !prune_target.is_projection_of(size, &target) {
            return Err(ProjectionError::InvalidProjection);
        }

        let pdb = match self.pdb_action.ok_or(ProjectionError::MissingPdbAction)? {
            PdbAction::Build { config } => Pdb::new(&prune_target, size, &config),
            PdbAction::UseExisting { pdb } => pdb,
        };

        Ok(Solver::with_pdb(pdb, target, prune_target, size))
    }
}
