//! Defines ways in which implementations of [`SlidingPuzzle`] can be displayed.

use crate::puzzle::sliding_puzzle::SlidingPuzzle;
use std::fmt::{Display, Write};

macro_rules! define_display {
    ($($(#[$annot:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$annot])*
            #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name<'a, Puzzle>
            where
                Puzzle: SlidingPuzzle,
            {
                puzzle: &'a Puzzle,
            }

            impl<'a, Puzzle> $name<'a, Puzzle>
            where
                Puzzle: SlidingPuzzle,
            {
                #[doc = concat!("Create a new [`", stringify!($name), "`] for displaying `puzzle`.")]
                #[must_use]
                pub fn new(puzzle: &'a Puzzle) -> Self {
                    Self { puzzle }
                }
            }
        )*
    };
}

define_display!(
    /// Displays the puzzle in a two dimensional grid with the numbers right-aligned.
    ///
    /// # Example
    ///
    /// ```
    /// #![feature(iter_intersperse)]
    ///
    /// use slidy::puzzle::{display::DisplayGrid, puzzle::Puzzle};
    ///
    /// fn main() {
    ///     let p = Puzzle::default();
    ///     let s = DisplayGrid::new(&p).to_string();
    ///     let rows = [
    ///         " 1  2  3  4",
    ///         " 5  6  7  8",
    ///         " 9 10 11 12",
    ///         "13 14 15  0",
    ///     ];
    ///     // Combine `rows` into a `String`, separated by new lines.
    ///     let expected = rows.into_iter().intersperse("\n").collect::<String>();
    ///     assert_eq!(s, expected);
    /// }
    /// ```
    DisplayGrid,
    /// Displays the puzzle on a single line with numbers separated by spaces, and rows separated
    /// by forward slashes.
    /// # Example
    ///
    /// ```
    /// use slidy::puzzle::{display::DisplayInline, puzzle::Puzzle};
    ///
    /// fn main() {
    ///     let p = Puzzle::default();
    ///     let s = DisplayInline::new(&p).to_string();
    ///     let expected = "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0".to_string();
    ///     assert_eq!(s, expected);
    /// }
    /// ```
    DisplayInline,
);

impl<Puzzle> Display for DisplayGrid<'_, Puzzle>
where
    Puzzle: SlidingPuzzle,
    Puzzle::Piece: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let max_number = self.puzzle.num_pieces();
        let num_digits = max_number.ilog10() as usize + 1;
        let (w, h) = self.puzzle.size().into();
        for y in 0..h {
            for x in 0..w {
                let n = self.puzzle.piece_at_xy((x, y));
                let a = format!("{n: >num_digits$}");
                f.write_str(&a)?;
                if x != w - 1 {
                    f.write_char(' ')?;
                }
            }
            if y != h - 1 {
                f.write_char('\n')?;
            }
        }
        Ok(())
    }
}

impl<Puzzle> Display for DisplayInline<'_, Puzzle>
where
    Puzzle: SlidingPuzzle,
    Puzzle::Piece: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let (w, h) = self.puzzle.size().into();
        for y in 0..h {
            for x in 0..w {
                let n = self.puzzle.piece_at_xy((x, y));
                f.write_str(&n.to_string())?;
                if x != w - 1 {
                    f.write_char(' ')?;
                }
            }
            if y != h - 1 {
                f.write_char('/')?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::{puzzle::Puzzle, size::Size};

    use super::*;

    #[test]
    fn test_display_grid() {
        let p = Puzzle::new(Size::new(4, 4).unwrap());
        let s1 = DisplayGrid::new(&p).to_string();
        let s2 = p.display_grid().to_string();
        assert_eq!(s1, " 1  2  3  4\n 5  6  7  8\n 9 10 11 12\n13 14 15  0");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_display_inline() {
        let p = Puzzle::new(Size::new(4, 4).unwrap());
        let s1 = DisplayInline::new(&p).to_string();
        let s2 = p.display_inline().to_string();
        let s3 = p.to_string();
        assert_eq!(s1, "1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0");
        assert_eq!(s1, s2);
        assert_eq!(s1, s3);
    }
}

#[cfg(test)]
mod benchmarks {
    extern crate test;

    use test::Bencher;

    use crate::puzzle::puzzle::Puzzle;

    use super::*;

    #[bench]
    fn bench_display_inline(b: &mut Bencher) {
        let p = Puzzle::default();
        b.iter(|| DisplayInline::new(&p).to_string());
    }

    #[bench]
    fn bench_display_grid(b: &mut Bencher) {
        let p = Puzzle::default();
        b.iter(|| DisplayGrid::new(&p).to_string());
    }
}
