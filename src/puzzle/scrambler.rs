//! Defines the [`Scrambler`] trait and several implementations.

use super::sliding_puzzle::SlidingPuzzle;
use crate::algorithm::{direction::Direction, r#move::r#move::Move};
use rand::Rng;

/// Trait defining a scrambling algorithm.
pub trait Scrambler<Puzzle>
where
    Puzzle: SlidingPuzzle,
{
    /// Scrambles the puzzle using [`rand::thread_rng`].
    fn scramble(&self, puzzle: &mut Puzzle) {
        let mut rng = rand::thread_rng();
        self.scramble_with_rng(puzzle, &mut rng);
    }

    /// Scrambles the puzzle using a given [`Rng`].
    fn scramble_with_rng<R: Rng>(&self, puzzle: &mut Puzzle, rng: &mut R);
}

/// Random state scrambler. Scrambles the puzzle in such a way that every solvable state is equally
/// likely to occur.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RandomState;

impl<Puzzle> Scrambler<Puzzle> for RandomState
where
    Puzzle: SlidingPuzzle,
{
    fn scramble_with_rng<R: Rng>(&self, puzzle: &mut Puzzle, rng: &mut R) {
        puzzle.reset();

        let n = puzzle.num_pieces();
        let mut parity = false;
        for i in 0..n - 2 {
            // Pick random element to go in position i
            let j = rng.gen_range(i..n);

            // Swap and check if we need to toggle parity
            if i != j {
                puzzle.try_swap_pieces(i, j);
                parity = !parity;
            }
        }

        // Swap the last two pieces if necessary to make it solvable
        if parity {
            puzzle.try_swap_pieces(n - 2, n - 1);
        }

        // Move blank to a random position
        let (w, h) = puzzle.size();
        let (d, r) = (rng.gen_range(0..h), rng.gen_range(0..w));

        puzzle.apply_move(Move::new(Direction::Down, d as u32));
        puzzle.apply_move(Move::new(Direction::Right, r as u32));
    }
}

/// Scrambles the puzzle by applying a fixed number of random single-tile moves.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RandomMoves {
    /// Number of random moves to apply.
    pub moves: u64,
    /// Are backtracking moves allowed? E.g. If one move of the scramble is R, is the next move
    /// allowed to be L? If this is false, the L move will not be allowed and a differentmove will
    /// be generated.
    pub allow_backtracking: bool,
    /// Are illegal moves counted? E.g. If the first generated move of the scramble is L (which
    /// can not be applied to the puzzle), should this be counted towards the total move count? If
    /// this is false, the L move will not be counted and a different move will be generated.
    pub allow_illegal_moves: bool,
}

impl<Puzzle> Scrambler<Puzzle> for RandomMoves
where
    Puzzle: SlidingPuzzle,
{
    fn scramble_with_rng<R: Rng>(&self, puzzle: &mut Puzzle, rng: &mut R) {
        let mut last_dir = None::<Direction>;
        for _ in 0..self.moves {
            let dir = {
                let mut d = rng.gen::<Direction>();
                while (!self.allow_backtracking && last_dir == Some(d.inverse()))
                    || (!self.allow_illegal_moves && !puzzle.can_move_dir(d))
                {
                    d = rng.gen();
                }
                d
            };

            last_dir = Some(dir);
            puzzle.try_move_dir(dir);
        }
    }
}

/// Scrambler that applies a single cycle of pieces to the puzzle. If `length` is even, the last
/// two pieces in the puzzle will also be swapped to make it solvable.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cycle {
    /// Length of the cycle.
    pub length: u64,
}

impl<Puzzle> Scrambler<Puzzle> for Cycle
where
    Puzzle: SlidingPuzzle,
{
    fn scramble_with_rng<R: Rng>(&self, puzzle: &mut Puzzle, rng: &mut R) {
        let n = puzzle.num_pieces();
        let cycle_len = (self.length as usize).min(if n % 2 == 0 { n - 1 } else { n });
        let max = if cycle_len % 2 == 0 { n - 2 } else { n };
        let pieces = rand::seq::index::sample(rng, max, cycle_len);

        for i in 1..cycle_len {
            puzzle.try_swap_pieces(pieces.index(0), pieces.index(i));
        }

        if self.length % 2 == 0 {
            puzzle.try_swap_pieces(n - 2, n - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzle::puzzle::Puzzle;

    mod random_state {
        use super::*;
        use crate::puzzle::{label::label::RowGrids, solvable::Solvable};

        #[test]
        fn test_solvable() {
            for (w, h) in [(2, 2), (4, 4), (10, 2), (20, 20)] {
                let mut p = Puzzle::new(w, h).unwrap();
                let x = RandomState;
                for _ in 0..100 {
                    p.reset();
                    x.scramble(&mut p);
                    assert!(RowGrids::is_solvable(&p));
                }
            }
        }
    }
}
