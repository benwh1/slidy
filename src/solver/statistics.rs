//! Defines types holding statistics related to solvers.

/// Statistics about an iteration of a breadth-first search used to build a pattern database.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PdbIterationStats {
    /// The greatest depth of entries currently in the PDB.
    pub depth: u8,

    /// The number of entries at depth `depth`.
    pub new: u64,

    /// The number of entries at depth less than or equal to `depth`.
    pub total: u64,
}

/// Statistics about an iteration of a depth-first search used to solve a puzzle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SolverIterationStats {
    /// The depth of the search iteration that just finished.
    pub depth: u8,
}
