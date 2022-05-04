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
