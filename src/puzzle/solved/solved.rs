use crate::puzzle::traits::SlidingPuzzle;

pub trait SolvedState<Piece, Puzzle>
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool;
}

struct Rows;
struct Columns;
struct RowsSetwise;
struct ColumnsSetwise;

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

impl<Piece, Puzzle> SolvedState<Piece, Puzzle> for Columns
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool {
        if puzzle.gap_position() != puzzle.num_pieces() {
            return false;
        }

        let (w, h) = puzzle.size();
        for y in 0..h {
            for x in 0..w {
                if (x, y) == (w - 1, h - 1) {
                    continue;
                }

                if puzzle.piece_at_xy(x, y).into() != (1 + y + h * x) as u64 {
                    return false;
                }
            }
        }

        true
    }
}

impl<Piece, Puzzle> SolvedState<Piece, Puzzle> for RowsSetwise
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool {
        if puzzle.gap_position() != puzzle.num_pieces() {
            return false;
        }

        let (w, h) = puzzle.size();
        for y in 0..h {
            for x in 0..w {
                if (x, y) == (w - 1, h - 1) {
                    continue;
                }

                let (_, solved_location_y) = {
                    let a = (puzzle.piece_at_xy(x, y).into() - 1) as usize;
                    (a % w, a / w)
                };

                if y != solved_location_y {
                    return false;
                }
            }
        }

        true
    }
}

impl<Piece, Puzzle> SolvedState<Piece, Puzzle> for ColumnsSetwise
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool {
        if puzzle.gap_position() != puzzle.num_pieces() {
            return false;
        }

        let (w, h) = puzzle.size();
        for y in 0..h {
            for x in 0..w {
                if (x, y) == (w - 1, h - 1) {
                    continue;
                }

                let (solved_location_x, _) = {
                    let a = (puzzle.piece_at_xy(x, y).into() - 1) as usize;
                    (a % w, a / w)
                };

                if x != solved_location_x {
                    return false;
                }
            }
        }

        true
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

    mod columns {
        use super::*;
        use crate::puzzle::solved::solved::Columns;

        #[test]
        fn test_columns() {
            let p = Puzzle::from_str("1 5 9 13/2 6 10 14/3 7 11 15/4 8 12 0").unwrap();
            assert!(p.is_solved::<Columns>());
        }

        #[test]
        fn test_columns_2() {
            let p = Puzzle::from_str("1 5 9 13/6 2 10 14/3 7 11 15/8 4 12 0").unwrap();
            assert!(!p.is_solved::<Columns>());
        }

        #[test]
        fn test_columns_3() {
            let p = Puzzle::from_str("1 11/2 12/3 13/4 14/5 15/6 16/7 17/8 18/9 19/10 0").unwrap();
            assert!(p.is_solved::<Columns>());
        }

        #[test]
        fn test_columns_4() {
            let p = Puzzle::from_str("1 11/2 13/3 12/4 14/6 15/5 16/7 17/8 18/9 19/10 0").unwrap();
            assert!(!p.is_solved::<Columns>());
        }
    }

    mod rows_setwise {
        use super::*;
        use crate::puzzle::solved::solved::RowsSetwise;

        #[test]
        fn test_rows_setwise() {
            let p = Puzzle::from_str("4 1 3 2/6 7 8 5/9 10 11 12/14 13 15 0").unwrap();
            assert!(p.is_solved::<RowsSetwise>());
        }

        #[test]
        fn test_rows_setwise_2() {
            let p = Puzzle::from_str("1 2 3 5/4 6 7 8/9 10 11 12/13 14 15 0").unwrap();
            assert!(!p.is_solved::<RowsSetwise>());
        }

        #[test]
        fn test_rows_setwise_3() {
            let p = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 12/13 14 0 15").unwrap();
            assert!(!p.is_solved::<RowsSetwise>());
        }

        #[test]
        fn test_rows_setwise_4() {
            let p = Puzzle::from_str("1 2 3/6 5 4/9 7 8/12 11 10/14 15 13/16 17 0").unwrap();
            assert!(p.is_solved::<RowsSetwise>());
        }

        #[test]
        fn test_rows_setwise_5() {
            let p = Puzzle::from_str("1 2 6/3 5 4/9 7 8/12 11 10/14 15 13/16 17 0").unwrap();
            assert!(!p.is_solved::<RowsSetwise>());
        }
    }

    mod columns_setwise {
        use super::*;
        use crate::puzzle::solved::solved::ColumnsSetwise;

        #[test]
        fn test_columns_setwise() {
            let p = Puzzle::from_str("13 14 15 8/1 10 3 4/5 6 11 12/9 2 7 0").unwrap();
            assert!(p.is_solved::<ColumnsSetwise>());
        }

        #[test]
        fn test_columns_setwise_2() {
            let p = Puzzle::from_str("13 14 15 8/1 3 10 4/5 6 11 12/9 2 7 0").unwrap();
            assert!(!p.is_solved::<ColumnsSetwise>());
        }

        #[test]
        fn test_columns_setwise_3() {
            let p = Puzzle::from_str("1 2 3 4/5 6 7 8/9 10 11 0/13 14 15 12").unwrap();
            assert!(!p.is_solved::<ColumnsSetwise>());
        }

        #[test]
        fn test_columns_setwise_4() {
            let p = Puzzle::from_str("5 8/3 2/17 14/13 12/11 10/9 6/1 18/7 16/15 4/19 0").unwrap();
            assert!(p.is_solved::<ColumnsSetwise>());
        }

        #[test]
        fn test_columns_setwise_5() {
            let p = Puzzle::from_str("1 11/7 17/4 14/2 13/12 3/5 15/9 19/8 16/10 18/6 0").unwrap();
            assert!(!p.is_solved::<ColumnsSetwise>());
        }
    }
}
