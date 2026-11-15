//! Defines the [`Max`] heuristic, which is the maximum of a set of heuristics.

use crate::solver::heuristic::Heuristic;

/// The maximum of the bounds given by a set of heuristics, which is a lower bound because each of
/// them is.
///
/// The heuristics can be given as a tuple of up to 8 heuristics of any types, or as a [`Vec`] of
/// heuristics of the same type. The maximum of an empty [`Vec`] is 0.
///
/// # Example
///
/// ```
/// use std::str::FromStr as _;
///
/// use slidy::{
///     algorithm::metric::Stm,
///     puzzle::{label::label::RowGrids, puzzle::Puzzle},
///     solver::heuristic::{
///         Heuristic as _, inversion::InversionCount, manhattan::ManhattanDistance, max::Max,
///     },
/// };
///
/// let heuristic = Max((ManhattanDistance(RowGrids), InversionCount));
///
/// // Manhattan distance 24, inversion count 18.
/// let puzzle = Puzzle::from_str("0 1 2 3/4 5 6 7/8 9 10 11/12 13 15 14").unwrap();
/// assert_eq!(heuristic.bound(&puzzle), 24);
///
/// // Manhattan distance 5, inversion count 9.
/// let puzzle = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 12/0 15 13 14").unwrap();
/// assert_eq!(heuristic.bound(&puzzle), 9);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Max<H>(pub H);

/// Implements [`Heuristic`] for [`Max`] of a tuple, given the tuple's type parameters and
/// indices.
macro_rules! impl_max_tuple {
    ($($h:ident $i:tt),+) => {
        impl<P, S, M, $($h),+> Heuristic<P, S, M> for Max<($($h,)+)>
        where
            $($h: Heuristic<P, S, M>,)+
        {
            type State = ($(<$h as Heuristic<P, S, M>>::State,)+);

            fn bound(&self, puzzle: &P) -> u64 {
                0 $(.max(<$h as Heuristic<P, S, M>>::bound(&self.0.$i, puzzle)))+
            }

            fn init(&self, puzzle: &P) -> Self::State {
                ($(<$h as Heuristic<P, S, M>>::init(&self.0.$i, puzzle),)+)
            }

            fn bound_after_move(
                &self,
                state: &mut Self::State,
                puzzle: &P,
                from: (u64, u64),
                to: (u64, u64),
            ) -> u64 {
                // Every state must be updated, so this doesn't stop early.
                0 $(.max(<$h as Heuristic<P, S, M>>::bound_after_move(
                    &self.0.$i,
                    &mut state.$i,
                    puzzle,
                    from,
                    to,
                )))+
            }

            fn undo_move(
                &self,
                state: &mut Self::State,
                puzzle: &P,
                from: (u64, u64),
                to: (u64, u64),
            ) {
                $(<$h as Heuristic<P, S, M>>::undo_move(&self.0.$i, &mut state.$i, puzzle, from, to);)+
            }
        }
    };
}

impl_max_tuple!(A 0);
impl_max_tuple!(A 0, B 1);
impl_max_tuple!(A 0, B 1, C 2);
impl_max_tuple!(A 0, B 1, C 2, D 3);
impl_max_tuple!(A 0, B 1, C 2, D 3, E 4);
impl_max_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
impl_max_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
impl_max_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);

impl<P, S, M, H> Heuristic<P, S, M> for Max<Vec<H>>
where
    H: Heuristic<P, S, M>,
{
    type State = Vec<H::State>;

    fn bound(&self, puzzle: &P) -> u64 {
        self.0.iter().map(|h| h.bound(puzzle)).max().unwrap_or(0)
    }

    fn init(&self, puzzle: &P) -> Self::State {
        self.0.iter().map(|h| h.init(puzzle)).collect()
    }

    fn bound_after_move(
        &self,
        state: &mut Self::State,
        puzzle: &P,
        from: (u64, u64),
        to: (u64, u64),
    ) -> u64 {
        self.0
            .iter()
            .zip(state)
            .map(|(h, s)| h.bound_after_move(s, puzzle, from, to))
            .max()
            .unwrap_or(0)
    }

    fn undo_move(&self, state: &mut Self::State, puzzle: &P, from: (u64, u64), to: (u64, u64)) {
        for (h, s) in self.0.iter().zip(state) {
            h.undo_move(s, puzzle, from, to);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::{
        algorithm::metric::Stm,
        puzzle::{
            label::label::{RowGrids, Rows},
            puzzle::Puzzle,
            sliding_puzzle::SlidingPuzzle as _,
        },
        solver::heuristic::{inversion::InversionCount, manhattan::ManhattanDistance},
    };

    #[test]
    fn test_max_bound() {
        let cases = [
            "1 2 3/4 5 6/7 8 0",
            "8 6 7/2 5 4/3 0 1",
            "1 4 5/0 7 8/2 3 6",
            "0 1 2 3/4 5 6 7/8 9 10 11/12 13 15 14",
            "1 2 3 4/5 6 7 8/9 10 11 12/0 15 13 14",
            "11 1 9 10/0 2 13 15/4 5 3 14/8 6 12 7",
        ];
        for state in cases {
            let puzzle = Puzzle::from_str(state).unwrap();
            assert!(puzzle.is_solvable(), "state {state}");
            let md = Heuristic::<_, RowGrids, Stm>::bound(&ManhattanDistance(RowGrids), &puzzle);
            let inv = Heuristic::<_, RowGrids, Stm>::bound(&InversionCount, &puzzle);
            let expected = md.max(inv);

            let one = Max((ManhattanDistance(RowGrids),));
            let two = Max((ManhattanDistance(RowGrids), InversionCount));
            let three = Max(((), InversionCount, ManhattanDistance(RowGrids)));
            assert_eq!(Heuristic::<_, RowGrids, Stm>::bound(&one, &puzzle), md);
            assert_eq!(
                Heuristic::<_, RowGrids, Stm>::bound(&two, &puzzle),
                expected
            );
            assert_eq!(
                Heuristic::<_, RowGrids, Stm>::bound(&three, &puzzle),
                expected
            );

            let rows = Heuristic::<_, Rows, Stm>::bound(&ManhattanDistance(Rows), &puzzle);
            let empty = Max(Vec::<ManhattanDistance<Rows>>::new());
            let vec = Max(vec![ManhattanDistance(Rows), ManhattanDistance(Rows)]);
            assert_eq!(Heuristic::<_, Rows, Stm>::bound(&empty, &puzzle), 0);
            assert_eq!(Heuristic::<_, Rows, Stm>::bound(&vec, &puzzle), rows);
        }
    }
}
