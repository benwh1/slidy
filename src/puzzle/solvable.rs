//! Defines the [`Solvable`] trait and implements it on some of the labels defined in
//! [`slidy::label::label`].
//!
//! [`slidy::label::label`]: ../label/label/index.html

use num_traits::ToPrimitive;

use crate::puzzle::label::label::Label;

use super::{
    label::label::{
        Checkerboard, ConcentricRectangles, Diagonals, Fringe, LastTwoRows, RowGrids, Rows, Spiral,
        SplitFringe, SplitLastTwoRows, SplitSquareFringe, SquareFringe, Trivial,
    },
    sliding_puzzle::SlidingPuzzle,
};

/// Trait for defining whether a puzzle is solvable with respect to a [`Label`]. Any puzzle that
/// has at least two pieces with the same label is always solvable.
///
/// [`Label`]: ../label/label/trait.Label.html
pub trait Solvable<Puzzle>
where
    Puzzle: SlidingPuzzle,
{
    /// Checks if the puzzle is solvable.
    #[must_use]
    fn is_solvable(puzzle: &Puzzle) -> bool;
}

fn trivial_size_solvable<P: SlidingPuzzle, L: Label>(puzzle: &P, label: &L) -> bool {
    // To check if a 1xn or nx1 puzzle is solvable, we do the following:
    // 1. Ignore the gap piece
    // 2. Enumerate the pieces
    // 3. Check if, for each pair (idx, piece), we have label(idx+1) == label(piece)
    let (w, h) = puzzle.size().into();
    if w == 1 {
        (0..h)
            .map(|i| puzzle.piece_at(i).to_u64().unwrap())
            .filter(|&p| p != 0)
            .enumerate()
            .all(|(i, p)| {
                label.position_label(puzzle.size(), (0, i as u64))
                    == label.position_label(puzzle.size(), (0, p - 1))
            })
    } else {
        (0..w)
            .map(|i| puzzle.piece_at(i).to_u64().unwrap())
            .filter(|&p| p != 0)
            .enumerate()
            .all(|(i, p)| {
                label.position_label(puzzle.size(), (i as u64, 0))
                    == label.position_label(puzzle.size(), (p - 1, 0))
            })
    }
}

impl<Puzzle> Solvable<Puzzle> for RowGrids
where
    Puzzle: SlidingPuzzle,
{
    fn is_solvable(puzzle: &Puzzle) -> bool {
        if puzzle.size().width() == 1 || puzzle.size().height() == 1 {
            return trivial_size_solvable(puzzle, &Self);
        }

        // Closure to get the piece that would be in position (x, y), if we do L* U* to move the
        // gap to the bottom right corner
        let (w, h) = puzzle.size().into();
        let (gx, gy) = puzzle.gap_position_xy();
        let piece_at = |i| {
            let (x, y) = (i % w, i / w);
            if x == w - 1 && y >= gy {
                if y == h - 1 {
                    // Gap piece
                    puzzle.piece_at_xy((gx, gy))
                } else {
                    puzzle.piece_at_xy((x, y + 1))
                }
            } else if y == gy && x >= gx {
                puzzle.piece_at_xy((x + 1, y))
            } else {
                puzzle.piece_at_xy((x, y))
            }
        };

        let n = puzzle.num_pieces() as usize;
        let mut seen = vec![false; n];
        let mut parity = false;
        for i in 0..n {
            if seen[i] {
                continue;
            }

            let mut index = i;
            let mut cycle_len = 0;
            while !seen[index] {
                seen[index] = true;
                cycle_len += 1;
                index = puzzle.solved_pos(piece_at(index as u64)) as usize;
            }

            if cycle_len % 2 == 0 {
                parity = !parity;
            }
        }

        !parity
    }
}

impl<Puzzle> Solvable<Puzzle> for Spiral
where
    Puzzle: SlidingPuzzle,
{
    fn is_solvable(puzzle: &Puzzle) -> bool {
        // Always solvable unless puzzle is 2x2, then equivalent to RowGrids.
        let (w, h) = puzzle.size().into();
        if (w, h) == (2, 2) {
            RowGrids::is_solvable(puzzle)
        } else {
            true
        }
    }
}

macro_rules! always_solvable {
    ($($t:ty),* $(,)?) => {
        $(
            impl<Puzzle> Solvable<Puzzle> for $t
            where
                Puzzle: SlidingPuzzle,
            {
                fn is_solvable(_puzzle: &Puzzle) -> bool {
                    true
                }
            }
        )*
    };
}

macro_rules! always_solvable_except_trivial_size {
    ($($t:ty),* $(,)?) => {
        $(
            impl<Puzzle> Solvable<Puzzle> for $t
            where
                Puzzle: SlidingPuzzle,
            {
                fn is_solvable(puzzle: &Puzzle) -> bool {
                    if puzzle.size().width() == 1 || puzzle.size().height() == 1 {
                        trivial_size_solvable(puzzle, &Self)
                    } else {
                        true
                    }
                }
            }
        )*
    };
}

always_solvable!(Trivial, Fringe, ConcentricRectangles, Checkerboard);

always_solvable_except_trivial_size!(
    Rows,
    SquareFringe,
    SplitFringe,
    SplitSquareFringe,
    Diagonals,
    LastTwoRows,
    SplitLastTwoRows,
);

#[cfg(test)]
mod tests {
    mod row_grids {
        use crate::puzzle::{label::label::RowGrids, puzzle::Puzzle, solvable::Solvable};
        use std::str::FromStr;

        #[test]
        fn test_solvable() {
            let solvable = vec![
                "1 2 3 0",
                "1 0 2 3",
                "1/2/3/0",
                "1/0/2/3",
                "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
                "2 3 1 4/5 6 7 8/9 10 11 12/13 14 15 0",
                "0 8 7/6 5 4/3 2 1",
                "3 1/2 0",
            ];
            let unsolvable = vec![
                "1 3 0 2",
                "0 3 1 2",
                "1/3/0/2",
                "0/3/1/2",
                "2 1 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
                "4 8 12 0/3 7 11 15/2 6 10 14/1 5 9 13",
                "4 5 6/1 2 3/7 8 0",
                "3 1 8/6 2 0/5 4 7",
            ];
            for s in solvable {
                let p = Puzzle::from_str(s).unwrap();
                assert!(RowGrids::is_solvable(&p));
            }
            for s in unsolvable {
                let p = Puzzle::from_str(s).unwrap();
                assert!(!RowGrids::is_solvable(&p));
            }
        }
    }
}
