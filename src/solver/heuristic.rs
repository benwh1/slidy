//! Defines the [`Heuristic`] trait which is used to compute a lower bound on the length of an
//! optimal solution of a puzzle.

pub mod manhattan;

/// Provides a function returning a lower bound on the number of moves needed to solve a puzzle.
pub trait Heuristic<P, T, S, M> {
    /// Returns a lower bound on the number of moves needed to solve `puzzle`.
    #[must_use]
    fn bound(&self, puzzle: &P) -> T;
}
