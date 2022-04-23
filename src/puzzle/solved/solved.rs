use crate::puzzle::traits::SlidingPuzzle;

pub trait SolvedState<Piece, Puzzle>
where
    Piece: Into<u64>,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn is_solved(puzzle: &Puzzle) -> bool;
}

struct Normal;

impl<Piece, Puzzle> SolvedState<Piece, Puzzle> for Normal
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
