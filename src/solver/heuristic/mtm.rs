//! Defines the [`MtmHeuristic`] heuristic, which wraps an [`Stm`] [`Heuristic`] and turns it into
//! an admissible [`Mtm`] heuristic.

use crate::{
    algorithm::metric::{Mtm, Stm},
    puzzle::sliding_puzzle::SlidingPuzzle,
    solver::heuristic::Heuristic,
};

/// Wrapper around an [`Stm`] [`Heuristic`] that divides the bound by `max(width, height) - 1`,
/// thereby making it admissible in [`Mtm`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MtmHeuristic<H>(pub H);

impl<P, S, H> Heuristic<P, S, Mtm> for MtmHeuristic<H>
where
    P: SlidingPuzzle,
    H: Heuristic<P, S, Stm>,
{
    fn bound(&self, puzzle: &P) -> u64 {
        let (w, h) = puzzle.size().into();
        // On a 1x1 puzzle, `k` would be 0, but the bound is always 0 there anyway.
        let k = (w.max(h) - 1).max(1);
        let h = self.0.bound(puzzle);
        h.div_ceil(k)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::{
        puzzle::{
            label::label::{RowGrids, Trivial},
            puzzle::Puzzle,
        },
        solver::heuristic::manhattan::ManhattanDistance,
    };

    #[test]
    fn test_trivial_mtm_bound() {
        let cases = [
            ("1 2 3/4 5 6/7 8 0", 0),
            ("1 2 3/4 5 6/0 7 8", 1),
            ("1 2 3/4 6 0/7 5 8", 1),
            ("1 0 3/4 2 6/7 5 8", 2),
            ("1 2 3 4/5 6 7 8/9 10 11 12/0 13 14 15", 1),
        ];
        for (state, expected) in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            let bound = MtmHeuristic(ManhattanDistance(Trivial)).bound(&puzzle);
            assert_eq!(bound, expected, "state {state}");
        }
    }

    #[test]
    fn test_mtm_bound() {
        let cases = [
            ("1 2 3/4 5 6/7 8 0", 0),
            ("1 2 3/4 5 6/0 7 8", 1),
            ("8 6 7/2 5 4/3 0 1", 11),
        ];
        for (state, expected) in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            let bound = MtmHeuristic(ManhattanDistance(RowGrids)).bound(&puzzle);
            assert_eq!(bound, expected, "state {state}");
        }
    }

    #[test]
    fn test_mtm_bound_1x1() {
        let puzzle = Puzzle::from_str("0").unwrap();
        assert_eq!(MtmHeuristic(ManhattanDistance(RowGrids)).bound(&puzzle), 0);
        assert_eq!(MtmHeuristic(ManhattanDistance(Trivial)).bound(&puzzle), 0);
    }

    #[test]
    fn test_mtm_bound_1xn() {
        let cases = [
            ("1 2 3 0", 0),
            ("0 1 2 3", 1),
            ("1 0 2 3", 1),
            ("0/1/2/3", 1),
        ];
        for (state, expected) in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            let bound = MtmHeuristic(ManhattanDistance(RowGrids)).bound(&puzzle);
            assert_eq!(bound, expected, "state {state}");
        }
    }
}
