use crate::puzzle::traits::SlidingPuzzle;

pub trait SolvedState<Piece, Puzzle>
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool;
}

struct Rows;

impl<Piece, Puzzle> SolvedState<Piece, Puzzle> for Rows
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool {
        if puzzle.gap_position() != puzzle.num_pieces() {
            return false;
        }

        (0..puzzle.num_pieces()).all(|i| puzzle.piece_at(i).into() == (i + 1) as u64)
    }
}

#[cfg(test)]
mod tests {
    use crate::puzzle::{puzzle::Puzzle, traits::SlidingPuzzle};
    use std::str::FromStr;

    mod rows {
        use super::*;
        use crate::puzzle::solved::solved::Rows;

        #[test]
        fn test_rows() {
            let p = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 12/13 14 15 0").unwrap();
            assert!(p.is_solved::<Rows>());
        }

        #[test]
        fn test_rows_2() {
            let p = Puzzle::from_str("1 2 3 4/5 6 7 8/9 0 11 12/13 14 15 10").unwrap();
            assert!(!p.is_solved::<Rows>());
        }

        #[test]
        fn test_rows_3() {
            let p = Puzzle::from_str("1 2/3 4/5 6/7 8/9 10/11 12/13 14/15 16/17 18/19 0").unwrap();
            assert!(p.is_solved::<Rows>());
        }

        #[test]
        fn test_rows_4() {
            let p = Puzzle::from_str("1 2/3 4/7 8/5 6/9 10/11 12/13 14/15 16/17 18/19 0").unwrap();
            assert!(!p.is_solved::<Rows>());
        }
    }
}
