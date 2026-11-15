//! Defines the [`Heuristic`] trait which is used to compute a lower bound on the length of an
//! optimal solution of a puzzle.

pub mod manhattan;
pub mod mtm;

/// Provides a function returning a lower bound on the number of moves needed to solve a puzzle.
pub trait Heuristic<P, S, M> {
    /// Returns a lower bound on the number of moves needed to solve `puzzle`.
    #[must_use]
    fn bound(&self, puzzle: &P) -> u64;

    /// Returns a lower bound on the number of moves needed to solve `puzzle`, which was reached by
    /// moving the piece at `from` into the gap at `to` (so the gap is now at `from`), from a
    /// state with bound `bound`.
    ///
    /// This is used when searching, so that the bound can be updated incrementally. The default
    /// implementation calls [`Heuristic::bound`].
    #[must_use]
    fn bound_after_move(&self, puzzle: &P, bound: u64, from: (u64, u64), to: (u64, u64)) -> u64 {
        let _ = (bound, from, to);
        self.bound(puzzle)
    }
}

impl<P, S, M> Heuristic<P, S, M> for () {
    fn bound(&self, _: &P) -> u64 {
        0
    }
}
