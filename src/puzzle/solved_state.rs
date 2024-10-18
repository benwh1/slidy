//! Defines the [`SolvedState`] trait and implements it on some of the labels defined in
//! [`slidy::label::label`].
//!
//! [`slidy::label::label`]: ../label/label/index.html

use crate::puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle};
use itertools::Itertools as _;

/// Defines a solved state.
pub trait SolvedState {
    /// Checks if `puzzle` is solved.
    #[must_use]
    fn is_solved<Puzzle>(&self, puzzle: &Puzzle) -> bool
    where
        Puzzle: SlidingPuzzle,
        Self: Sized;
}

impl<L: Label> SolvedState for L {
    fn is_solved<Puzzle>(&self, puzzle: &Puzzle) -> bool
    where
        Puzzle: SlidingPuzzle,
    {
        let size = puzzle.size();
        let (w, h) = size.into();
        if puzzle.gap_position_xy() != (w - 1, h - 1) {
            return false;
        }

        (0..w)
            .cartesian_product(0..h)
            .take(size.num_pieces() as usize)
            .all(|(x, y)| {
                // Label of piece in position (x, y)
                let solved_pos = puzzle.solved_pos_xy(puzzle.piece_at_xy((x, y)));
                let piece_label = self.try_position_label(size, solved_pos);

                // Label of piece in position (x, y) on a solved puzzle
                let solved_label = self.try_position_label(size, (x, y));

                piece_label == solved_label
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::puzzle::Puzzle;
    use std::str::FromStr;

    macro_rules! test_solved_state {
        (fn $name:ident, $i:literal, $label:expr, $ok:literal : $pos:literal) => {
            #[test]
            fn $name() {
                let p = Puzzle::from_str($pos).unwrap();
                if $ok {
                    assert!($label.is_solved(&p));
                } else {
                    assert!(!$label.is_solved(&p));
                }
            }
        };

        ($label:ty, $($i:literal : $ok:literal : $pos:literal),+ $(,)?) => {
            ::paste::paste! {
                mod [< $label:snake >] {
                    use super::*;
                    use crate::puzzle::{label::label::$label, solved_state::SolvedState};

                    $(test_solved_state!(
                        fn [< test_ $label:snake _ $i >] , $i, $label, $ok : $pos);
                    )*
                }
            }
        };
    }

    test_solved_state!(
        Trivial,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: false: "1 2 3 4/5 6 7 8/9 0 11 12/13 14 15 10",
        3: true:  "1 2/3 4/5 6/7 8/9 10/11 12/13 14/15 16/17 18/19 0",
        4: false: "10 8/15 3/11 18/5 2/6 19/13 9/12 14/0 4/16 7/1 17",
        5: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
    );

    test_solved_state!(
        RowGrids,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: false: "1 2 3 4/5 6 7 8/9 0 11 12/13 14 15 10",
        3: true:  "1 2/3 4/5 6/7 8/9 10/11 12/13 14/15 16/17 18/19 0",
        4: false: "1 2/3 4/7 8/5 6/9 10/11 12/13 14/15 16/17 18/19 0",
        5: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
    );

    test_solved_state!(
        Rows,
        1: true:  "4 1 3 2/6 7 8 5/9 10 11 12/14 13 15 0",
        2: false: "1 2 3 5/4 6 7 8/9 10 11 12/13 14 15 0",
        3: false: "1 2 3 4/5 6 7 8/9 10 11 12/13 14 0 15",
        4: true:  "1 2 3/6 5 4/9 7 8/12 11 10/14 15 13/16 17 0",
        5: false: "1 2 6/3 5 4/9 7 8/12 11 10/14 15 13/16 17 0",
        6: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
    );

    test_solved_state!(
        Fringe,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "13 4 5 3/1 8 14 7/2 10 15 11/9 6 12 0",
        3: false: "13 8 5 3/1 4 14 7/2 10 15 11/9 6 12 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 12/13 14 0 15",
        5: true:  "9 8 7 1 2 3 6 5 11 4/10 13 12 15 14 17 16 19 18 0",
        6: false: "9 8 7 1 2 3 6 5 12 4/10 13 11 15 14 17 16 19 18 0",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "4 21 17 13/9 8 22 18/5 14 19 23/2 10 11 20/1 7 15 16/3 6 12 0",
    );

    test_solved_state!(
        FringeGrids,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: false: "1 2 3 4/5 6 7 8/9 0 11 12/13 14 15 10",
        3: true:  "1 2/3 4/5 6/7 8/9 10/11 12/13 14/15 16/17 18/19 0",
        4: false: "1 2/3 4/7 8/5 6/9 10/11 12/13 14/15 16/17 18/19 0",
        5: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
    );

    test_solved_state!(
        SquareFringe,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "13 4 5 3/1 8 14 7/2 10 15 11/9 6 12 0",
        3: false: "13 8 5 3/1 4 14 7/2 10 15 11/9 6 12 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 12/13 14 0 15",
        5: true:  "11 2 13 4 15 6 17 8 19 9/1 12 3 14 5 16 7 18 10 0",
        6: false: "2 1 3 4 5 6 7 8 9 10/11 12 13 14 15 16 17 18 19 0",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "4 1 3 2/8 7 6 5/9 13 17 21/10 18 14 22/12 16 23 19/11 15 20 0",
    );

    test_solved_state!(
        SplitFringe,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "4 1 3 2/13 7 6 8/5 14 12 11/9 10 15 0",
        3: false: "13 4 5 3/1 8 14 7/2 10 15 11/9 6 12 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 12/13 14 0 15",
        5: true:  "10 9 8 7 6 5 4 3 2 1/11 19 18 17 16 15 14 13 12 0",
        6: false: "10 9 8 7 6 5 4 3 2 1/12 19 18 17 16 15 14 13 11 0",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "4 3 2 1/17 8 6 7/21 14 12 11/13 10 23 16/9 22 15 20/5 18 19 0",
        9: false: "4 3 2 1/17 8 6 7/21 14 12 11/13 10 23 20/9 22 15 16/5 18 19 0",
    );

    test_solved_state!(
        SplitSquareFringe,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: false: "4 1 3 2/13 7 6 8/5 14 15 11/9 10 12 0",
        3: false: "13 4 5 3/1 8 14 7/2 10 15 11/9 6 12 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 12/13 14 0 15",
        5: true:  "11 2 13 4 15 6 17 8 10 9/1 12 3 14 5 16 7 18 19 0",
        6: false: "11 2 13 4 15 6 17 8 19 9/1 12 3 14 5 16 7 18 10 0",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "4 1 3 2/8 7 6 5/9 12 11 10/17 16 14 15/13 22 20 19/21 18 23 0",
    );

    test_solved_state!(
        Diagonals,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "1 5 6 10/2 9 4 14/3 13 8 15/7 11 12 0",
        3: false: "5 1 6 10/2 9 4 14/3 13 8 15/7 11 12 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 12/13 14 0 15",
        5: true:  "1 11 12 13 14 15 16 17 18 19/2 3 4 5 6 7 8 9 10 0",
        6: false: "1 11 12 13 14 15 16 17 18 19/3 2 4 5 6 7 8 9 10 0",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "1 5 6 10/2 9 4 14/3 13 8 15/7 17 18 19/11 21 16 23/12 22 20 0",
    );

    test_solved_state!(
        LastTwoRows,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "4 3 2 1/6 5 8 7/13 14 15 12/9 10 11 0",
        3: false: "5 1 6 10/2 9 4 14/3 13 8 15/7 11 12 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 0/13 14 15 12",
        5: true:  "11 2 13 4 15 6 17 8 19 10/1 12 3 14 5 16 7 18 9 0",
        6: false: "2 1 3 4 5 6 7 8 9 10/11 12 13 14 15 16 17 18 19 0",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "2 1 4 3/7 8 5 6/10 11 12 9/16 13 14 15/17 22 19 20/21 18 23 0",
    );

    test_solved_state!(
        SplitLastTwoRows,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "9 13 3 4/5 10 14 8/2 7 11 12/1 6 15 0",
        3: false: "5 1 6 10/2 9 4 14/3 13 8 15/7 11 12 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 0/13 14 15 12",
        5: true:  "11 2 13 4 15 6 17 8 19 10/1 12 3 14 5 16 7 18 9 0",
        6: false: "2 1 3 4 5 6 7 8 9 10/11 12 13 14 15 16 17 18 19 0",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "2 1 4 21/22 8 5 6/10 11 12 9/16 13 14 15/17 7 19 20/3 18 23 0",
    );

    test_solved_state!(
        ConcentricRectangles,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "2 5 1 13/12 11 6 15/4 10 7 8/9 14 3 0",
        3: false: "13 4 2 1/9 12 3 7/5 15 11 6/14 8 10 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 0/13 14 15 12",
        5: true:  "2 10 15 1 4 6 19 5 16 13/8 18 14 17 7 9 11 12 3 0",
        6: false: "1 2 3 4 5 6 7 8 9 10/11 12 13 14 15 16 17 18 0 19",
        7: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        8: true:  "9 3 22 23/20 6 11 13/1 7 10 4/21 19 18 17/5 14 15 8/16 2 12 0",
    );

    test_solved_state!(
        Spiral,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "3 1 2 8/13 6 7 4/5 10 11 12/9 15 14 0",
        3: false: "14 10 13 3/5 1 7 15/4 9 8 12/11 6 2 0",
        4: false: "1 2 3 4/5 6 7 8/9 10 11 0/13 14 15 12",
        5: true:  "9 8 7 6 5 4 3 2 1 10/11 19 18 17 16 15 14 13 12 0",
        6: false: "9 8 7 6 5 4 3 2 1 10/19 11 18 17 16 15 14 13 12 0",
        7: false: "9 8 7 6 5 4 3 2 10 1/11 19 18 17 16 15 14 13 12 0",
        8: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
        9: true:  "3 1 2 12/21 6 15 4/5 10 11 16/17 18 7 20/9 14 19 8/13 22 23 0",
    );

    test_solved_state!(
        SpiralGrids,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: false: "1 2 3 4/5 6 7 8/9 0 11 12/13 14 15 10",
        3: true:  "1 2/3 4/5 6/7 8/9 10/11 12/13 14/15 16/17 18/19 0",
        4: false: "1 2/3 4/7 8/5 6/9 10/11 12/13 14/15 16/17 18/19 0",
        5: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
    );

    test_solved_state!(
        Checkerboard,
        1: true:  "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
        2: true:  "11 4 6 12/2 14 15 1/9 5 3 13/7 8 10 0",
        3: false: "11 4 6 12/2 0 5 1/9 15 14 10/7 8 13 3",
        4: true:  "1 15 12 10 3 19 5 2 14 11/4 9 6 7 8 16 13 18 17 0",
        5: false: "3 17 5 1 6 4 18 7 2 13/8 12 11 15 10 14 9 16 19 0",
        6: false: "3 8 9 10/4 15 0 12/6 2 1 5/11 7 14 13",
    );
}
