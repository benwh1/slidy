use crate::puzzle::{label::label::Label, sliding_puzzle::SlidingPuzzle};
use itertools::Itertools;

pub trait SolvedState<Piece, Puzzle>
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool;
}

impl<Piece, Puzzle, T> SolvedState<Piece, Puzzle> for T
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
    T: Label<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool {
        let (w, h) = puzzle.size();
        if puzzle.gap_position_xy() != (w - 1, h - 1) {
            return false;
        }

        (0..w)
            .cartesian_product(0..h)
            .take(w * h - 1)
            .all(|(x, y)| {
                // Label of piece in position (x, y)
                let piece_label = T::piece_label(w, h, puzzle.piece_at_xy(x, y));

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

        ($label:ty, $($i:literal : $ok:literal : $pos:literal,)*) => {
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
}
