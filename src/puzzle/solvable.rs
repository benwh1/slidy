//! Defines the [`Solvable`] trait and implements it on some of the labels defined in
//! [`crate::label::label`].

use num_traits::PrimInt;

use super::{
    label::label::{
        ConcentricRectangles, Diagonals, Fringe, LastTwoRows, RowGrids, Rows, Spiral, SplitFringe,
        SplitLastTwoRows, SplitSquareFringe, SquareFringe, Trivial,
    },
    sliding_puzzle::SlidingPuzzle,
};

/// Trait for defining whether a puzzle is solvable with respect to a [`Label`]. Any puzzle that
/// has at least two pieces with the same label is always solvable.
pub trait Solvable<Piece, Puzzle>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
{
    /// Checks if the puzzle is solvable.
    #[must_use]
    fn solvable(puzzle: &Puzzle) -> bool;
}

impl<Piece, Puzzle> Solvable<Piece, Puzzle> for RowGrids
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn solvable(puzzle: &Puzzle) -> bool {
        // Closure to get the piece that would be in position (x, y), if we do L* U* to move the
        // gap to the bottom right corner
        let (w, h) = puzzle.size();
        let (gx, gy) = puzzle.gap_position_xy();
        let piece_at = |i| {
            let (x, y) = (i % w, i / w);
            if x == w - 1 && y >= gy {
                if y == h - 1 {
                    // Gap piece
                    puzzle.piece_at_xy_unchecked(gx, gy)
                } else {
                    puzzle.piece_at_xy_unchecked(x, y + 1)
                }
            } else if y == gy && x >= gx {
                puzzle.piece_at_xy_unchecked(x + 1, y)
            } else {
                puzzle.piece_at_xy_unchecked(x, y)
            }
        };

        let n = puzzle.num_pieces();
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
                index = puzzle.solved_pos_unchecked(piece_at(index));
            }

            if cycle_len % 2 == 0 {
                parity = !parity;
            }
        }

        !parity
    }
}

macro_rules! always_solvable {
    ($($t:ty),* $(,)?) => {
        $(
            impl<Piece, Puzzle> Solvable<Piece, Puzzle> for $t
            where
                Piece: PrimInt,
                Puzzle: SlidingPuzzle<Piece>,
            {
                fn solvable(_puzzle: &Puzzle) -> bool {
                    true
                }
            }
        )*
    };
}

always_solvable!(
    Trivial,
    Rows,
    Fringe,
    SquareFringe,
    SplitFringe,
    SplitSquareFringe,
    Diagonals,
    LastTwoRows,
    SplitLastTwoRows,
    ConcentricRectangles,
);

impl<Piece, Puzzle> Solvable<Piece, Puzzle> for Spiral
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn solvable(puzzle: &Puzzle) -> bool {
        // Always solvable unless puzzle is 2x2, then equivalent to RowGrids.
        if puzzle.size() == (2, 2) {
            RowGrids::solvable(puzzle)
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    mod row_grids {
        use crate::puzzle::{label::label::RowGrids, puzzle::Puzzle, solvable::Solvable};
        use std::str::FromStr;

        #[test]
        fn test_solvable() {
            let solvable = vec![
                "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
                "2 3 1 4/5 6 7 8/9 10 11 12/13 14 15 0",
                "0 8 7/6 5 4/3 2 1",
                "3 1/2 0",
            ];
            let unsolvable = vec![
                "2 1 3 4/5 6 7 8/9 10 11 12/13 14 15 0",
                "4 8 12 0/3 7 11 15/2 6 10 14/1 5 9 13",
                "4 5 6/1 2 3/7 8 0",
                "3 1 8/6 2 0/5 4 7",
            ];
            for s in solvable {
                let p = Puzzle::from_str(s).unwrap();
                assert!(RowGrids::solvable(&p));
            }
            for s in unsolvable {
                let p = Puzzle::from_str(s).unwrap();
                assert!(!RowGrids::solvable(&p));
            }
        }
    }
}
