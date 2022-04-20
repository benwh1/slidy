use crate::puzzle::{puzzle::Puzzle, traits::SlidingPuzzle};
use std::{fmt::Display, marker::PhantomData};

pub struct DisplayPuzzle<'a, T> {
    puzzle: &'a Puzzle,
    phantom: PhantomData<T>,
}

impl<'a, T> DisplayPuzzle<'a, T> {
    pub fn new(puzzle: &'a Puzzle) -> Self {
        Self {
            puzzle,
            phantom: PhantomData,
        }
    }
}

pub struct DisplayGrid;

impl Display for DisplayPuzzle<'_, DisplayGrid> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let max_number = self.puzzle.num_pieces();
        let num_digits = max_number.log10() + 1;
        let (w, h) = self.puzzle.size();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                let n = self.puzzle.piece_at_xy(x, y);
                let a = format!("{: >length$}", n, length = num_digits as usize);
                s.push_str(&a);
                s.push(' ');
            }
            s.pop();
            s.push('\n');
        }
        s.pop();
        write!(f, "{}", &s)
    }
}

#[cfg(test)]
mod tests {
    use super::{DisplayGrid, DisplayPuzzle};
    use crate::puzzle::puzzle::Puzzle;

    #[test]
    fn test_display_puzzle() {
        let p = Puzzle::new(4, 4);
        let a = DisplayPuzzle::<DisplayGrid>::new(&p);
        let s = a.to_string();
        assert_eq!(s, " 1  2  3  4\n 5  6  7  8\n 9 10 11 12\n13 14 15  0");
    }
}
