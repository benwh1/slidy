use super::{
    label::label::{
        ConcentricRectangles, Diagonals, Fringe, LastTwoRows, RowGrids, Rows, SplitFringe,
        SplitLastTwoRows, SplitSquareFringe, SquareFringe,
    },
    sliding_puzzle::SlidingPuzzle,
};

pub trait Solvable<Piece, Puzzle>
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn solvable(puzzle: &Puzzle) -> bool;
}

impl<Piece, Puzzle> Solvable<Piece, Puzzle> for RowGrids
where
    Piece: Into<u64>,
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
                    puzzle.piece_at_xy(gx, gy)
                } else {
                    puzzle.piece_at_xy(x, y + 1)
                }
            } else if y == gy && x >= gx {
                puzzle.piece_at_xy(x + 1, y)
            } else {
                puzzle.piece_at_xy(x, y)
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
                index = puzzle.solved_pos(piece_at(index));
            }

            if cycle_len % 2 == 0 {
                parity = !parity;
            }
        }

        !parity
    }
}

macro_rules! always_solvable {
    () => {};
    ($t:ty) => {
        impl<Piece, Puzzle> Solvable<Piece, Puzzle> for $t
        where
            Piece: Into<u64>,
            Puzzle: SlidingPuzzle<Piece>,
        {
            fn solvable(_puzzle: &Puzzle) -> bool {
                true
            }
        }
    };
    ($t:ty, $($t2:ty,)*) => {
        always_solvable!($t);
        always_solvable!($($t2,)*);
    }
}

always_solvable!(
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
