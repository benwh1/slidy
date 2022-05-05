use super::{
    label::label::{
        ConcentricRectangles, Diagonals, Fringe, LastTwoRows, Rows, SplitFringe, SplitLastTwoRows,
        SplitSquareFringe, SquareFringe,
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
