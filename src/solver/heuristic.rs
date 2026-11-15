//! Defines the [`Heuristic`] trait which is used to compute a lower bound on the length of an
//! optimal solution of a puzzle.

pub mod manhattan;
pub mod mtm;

/// Provides a function returning a lower bound on the number of moves needed to solve a puzzle.
///
/// When searching, the bound can also be updated incrementally as moves are made and undone, using
/// a [`Heuristic::State`] that is created at the start of the search.
pub trait Heuristic<P, S, M> {
    /// Information kept during a search, so that the bound can be updated incrementally when a
    /// move is made instead of being computed from scratch. This can be `()` if it isn't needed.
    type State;

    /// Returns a lower bound on the number of moves needed to solve `puzzle`.
    #[must_use]
    fn bound(&self, puzzle: &P) -> u64;

    /// Returns the state for a search starting from `puzzle`.
    #[must_use]
    fn init(&self, puzzle: &P) -> Self::State;

    /// Updates `state` after the piece at `from` is moved into the gap at `to` (so the gap is now
    /// at `from`), and returns a lower bound on the number of moves needed to solve `puzzle`,
    /// which is the state after the move.
    fn bound_after_move(
        &self,
        state: &mut Self::State,
        puzzle: &P,
        from: (u64, u64),
        to: (u64, u64),
    ) -> u64;

    /// Undoes [`Heuristic::bound_after_move`] with the same `from` and `to`, where `puzzle` is
    /// the state after the move has been undone (so the piece is back at `from`).
    ///
    /// Moves are undone in the opposite order to the order they were made in.
    fn undo_move(&self, state: &mut Self::State, puzzle: &P, from: (u64, u64), to: (u64, u64));
}

impl<P, S, M> Heuristic<P, S, M> for () {
    type State = ();

    fn bound(&self, _: &P) -> u64 {
        0
    }

    fn init(&self, _: &P) {}

    fn bound_after_move(&self, (): &mut (), _: &P, _: (u64, u64), _: (u64, u64)) -> u64 {
        0
    }

    fn undo_move(&self, (): &mut (), _: &P, _: (u64, u64), _: (u64, u64)) {}
}

#[cfg(test)]
mod tests {
    use rand::{RngExt as _, SeedableRng as _};
    use rand_xoshiro::Xoshiro256StarStar;

    use super::*;
    use crate::{
        algorithm::{
            direction::Direction,
            metric::{Mtm, Stm},
        },
        puzzle::{
            label::label::{
                Checkerboard, Diagonals, Fringe, RowGrids, Rows, SplitFringe, SplitSquareFringe,
                SquareFringe, Trivial,
            },
            puzzle::Puzzle,
            size::Size,
            sliding_puzzle::SlidingPuzzle as _,
        },
        solver::heuristic::{manhattan::ManhattanDistance, mtm::MtmHeuristic},
    };

    /// Checks that the bounds from [`Heuristic::bound_after_move`] match [`Heuristic::bound`]
    /// while making and undoing random moves.
    fn check_incremental<S, M, H>(heuristic: &H)
    where
        H: Heuristic<Puzzle, S, M>,
    {
        let dirs = [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ];
        let mut rng = Xoshiro256StarStar::seed_from_u64(3);

        for (w, h) in [(3, 3), (4, 4), (5, 3), (2, 6), (6, 2)] {
            let mut puzzle = Puzzle::new(Size::new(w, h).unwrap());
            let mut state = heuristic.init(&puzzle);
            let mut moves = Vec::new();

            for _ in 0..3000 {
                if !moves.is_empty() && rng.random_range(0..3) == 0 {
                    let (dir, from, to): (Direction, _, _) = moves.pop().unwrap();
                    puzzle.move_dir(dir.inverse());
                    heuristic.undo_move(&mut state, &puzzle, from, to);
                } else {
                    let dir = dirs[rng.random_range(0..4)];
                    let to = puzzle.gap_position_xy();
                    if puzzle.try_move_dir(dir) {
                        let from = puzzle.gap_position_xy();
                        let bound = heuristic.bound_after_move(&mut state, &puzzle, from, to);
                        assert_eq!(bound, heuristic.bound(&puzzle));
                        moves.push((dir, from, to));
                    }
                }
            }
        }
    }

    #[test]
    fn test_incremental_bounds() {
        fn check<S>(label: S)
        where
            S: Clone,
            ManhattanDistance<S>: Heuristic<Puzzle, S, Stm>,
        {
            check_incremental::<S, Stm, _>(&ManhattanDistance(label.clone()));
            check_incremental::<S, Mtm, _>(&MtmHeuristic(ManhattanDistance(label)));
        }

        check(Trivial);
        check(RowGrids);
        check(Rows);
        check(Fringe);
        check(SquareFringe);
        check(SplitFringe);
        check(SplitSquareFringe);
        check(Diagonals);
        check(Checkerboard);
        check_incremental::<RowGrids, Stm, _>(&());
    }
}
