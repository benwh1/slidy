use crate::puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle};
use itertools::Itertools;
use num_traits::PrimInt;

pub trait SolvedState {
    fn is_solved<Piece, Puzzle>(puzzle: &Puzzle) -> bool
    where
        Piece: PrimInt,
        Puzzle: SlidingPuzzle<Piece>,
        Self: Sized;
}

impl<T> SolvedState for T
where
    T: Label,
{
    fn is_solved<Piece, Puzzle>(puzzle: &Puzzle) -> bool
    where
        Piece: PrimInt,
        Puzzle: SlidingPuzzle<Piece>,
    {
        let (w, h) = puzzle.size();
        if puzzle.gap_position_xy() != (w - 1, h - 1) {
            return false;
        }

        (0..w)
            .cartesian_product(0..h)
            .take(w * h - 1)
            .all(|(x, y)| {
                // Label of piece in position (x, y)
                let (sx, sy) = puzzle.solved_pos_xy(puzzle.piece_at_xy(x, y));
                let piece_label = T::position_label(w, h, sx, sy);

                // Label of piece in position (x, y) on a solved puzzle
                let solved_label = T::position_label(w, h, x, y);

                piece_label == solved_label
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::{puzzle::Puzzle, sliding_puzzle::SlidingPuzzle};
    use std::str::FromStr;

    macro_rules! test_solved_state {
        (fn $name:ident, $i:literal, $label:ty, $ok:literal : $pos:literal) => {
            #[test]
            fn $name() {
                let p = Puzzle::from_str($pos).unwrap();
                if $ok {
                    assert!(p.is_solved::<$label>());
                } else {
                    assert!(!p.is_solved::<$label>());
                }
            }
        };

        ($label:ty, $($i:literal : $ok:literal : $pos:literal),+ $(,)?) => {
            ::paste::paste! {
                mod [< $label:snake >] {
                    use super::*;
                    use crate::puzzle::label::label::$label;

                    $(test_solved_state!(
                        fn [< test_ $label:snake _ $i >] , $i, $label, $ok : $pos);
                    )*
                }
            }
        };
    }

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
}
