use super::sliding_puzzle::SlidingPuzzle;
use crate::algorithm::{direction::Direction, puzzle_move::Move};
use rand::Rng;

pub trait Scrambler<P, Piece>
where
    P: SlidingPuzzle<Piece>,
    Piece: Into<u64>,
{
    fn scramble(&self, puzzle: &mut P);
}

struct RandomState;

impl<P, Piece> Scrambler<P, Piece> for RandomState
where
    P: SlidingPuzzle<Piece>,
    Piece: Into<u64>,
{
    fn scramble(&self, puzzle: &mut P) {
        puzzle.reset();

        let mut rng = rand::thread_rng();

        let n = puzzle.num_pieces();
        let mut parity = false;
        for i in 0..n - 2 {
            // Pick random element to go in position i
            let j = rng.gen_range(i..n);

            // Swap and check if we need to toggle parity
            if i != j {
                puzzle.swap_pieces(i, j);
                parity = !parity;
            }
        }

        // Swap the last two pieces if necessary to make it solvable
        if parity {
            puzzle.swap_pieces(n - 2, n - 1);
        }

        // Move blank to a random position
        let (w, h) = puzzle.size();
        let (d, r) = (rng.gen_range(0..h), rng.gen_range(0..w));

        puzzle.apply_move(Move::new(Direction::Down, d as u32));
        puzzle.apply_move(Move::new(Direction::Right, r as u32));
    }
}

struct RandomMoves {
    moves: u64,
    allow_cancellation: bool,
    require_applyable: bool,
}

impl<P, Piece> Scrambler<P, Piece> for RandomMoves
where
    P: SlidingPuzzle<Piece>,
    Piece: Into<u64>,
{
    fn scramble(&self, puzzle: &mut P) {
        let mut rng = rand::thread_rng();

        let mut last_dir = None::<Direction>;
        for _ in 0..self.moves {
            let dir = {
                let mut d = rng.gen::<Direction>();
                while (!self.allow_cancellation && last_dir == Some(d.inverse()))
                    || (self.require_applyable && !puzzle.can_move_dir(d))
                {
                    d = rng.gen();
                }
                d
            };

            last_dir = Some(dir);
            puzzle.move_dir(dir);
        }
    }
}
