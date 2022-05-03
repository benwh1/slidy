use super::sliding_puzzle::SlidingPuzzle;

pub trait Scrambler<P, Piece>
where
    P: SlidingPuzzle<Piece>,
    Piece: Into<u64>,
{
    fn scramble(puzzle: &mut P);
}
