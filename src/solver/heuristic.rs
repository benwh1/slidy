//! Defines the [`Heuristic`] trait which is used to compute a lower bound on the length of an
//! optimal solution of a puzzle.

pub mod manhattan;
pub mod mtm;

/// Provides a function returning a lower bound on the number of moves needed to solve a puzzle.
pub trait Heuristic<P, S, M> {
    /// Returns a lower bound on the number of moves needed to solve `puzzle`.
    #[must_use]
    fn bound(&self, puzzle: &P) -> u64;
}

impl<P, S, M> Heuristic<P, S, M> for () {
    fn bound(&self, _: &P) -> u64 {
        0
    }
}
