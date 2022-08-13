use num_traits::PrimInt;

use crate::puzzle::sliding_puzzle::SlidingPuzzle;
use std::{fmt::Display, marker::PhantomData};

macro_rules! define_display {
    ($name:ident) => {
        pub struct $name<'a, Piece, Puzzle>
        where
            Piece: PrimInt,
            Puzzle: SlidingPuzzle<Piece>,
        {
            puzzle: &'a Puzzle,
            phantom_piece: PhantomData<Piece>,
        }

        impl<'a, Piece, Puzzle> $name<'a, Piece, Puzzle>
        where
            Piece: PrimInt,
            Puzzle: SlidingPuzzle<Piece>,
        {
            pub fn new(puzzle: &'a Puzzle) -> Self {
                Self {
                    puzzle,
                    phantom_piece: PhantomData,
                }
            }
        }
    };
}

define_display!(DisplayGrid);
define_display!(DisplayInline);

impl<Piece, Puzzle> Display for DisplayGrid<'_, Piece, Puzzle>
where
    Piece: PrimInt + Display,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let max_number = self.puzzle.num_pieces();
        let num_digits = max_number.ilog10() as usize + 1;
        let (w, h) = self.puzzle.size();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                let n = self.puzzle.piece_at_xy(x, y);
                let a = format!("{n: >num_digits$}");
                s.push_str(&a);
                s.push(' ');
            }
            s.pop();
            s.push('\n');
        }
        s.pop();
        f.write_str(&s)
    }
}

impl<Piece, P> Display for DisplayInline<'_, Piece, P>
where
    Piece: PrimInt + Display,
    P: SlidingPuzzle<Piece>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let (w, h) = self.puzzle.size();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                let n = self.puzzle.piece_at_xy(x, y);
                s.push_str(&n.to_string());
                s.push(' ');
            }
            s.pop();
            s.push('/');
        }
        s.pop();
        write!(f, "{}", &s)
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::puzzle::Puzzle;

    use super::*;

    #[test]
    fn test_display_grid() {
        let p = Puzzle::new(4, 4).unwrap();
        let a = DisplayGrid::new(&p);
        let s = a.to_string();
        assert_eq!(s, " 1  2  3  4\n 5  6  7  8\n 9 10 11 12\n13 14 15  0");
    }

    #[test]
    fn test_display_inline() {
        let p = Puzzle::new(4, 4).unwrap();
        let a = DisplayInline::new(&p);
        let s = a.to_string();
        assert_eq!(s, "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0");
    }
}
