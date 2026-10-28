//! Defines the [`Scrambler`] trait and several implementations.

use rand::{Rng, RngExt as _};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    algorithm::{direction::Direction, r#move::r#move::Move},
    puzzle::{size::Size, sliding_puzzle::SlidingPuzzle},
};

/// Trait defining a scrambling algorithm.
pub trait Scrambler {
    /// Checks if this `Scrambler` can be used with a given puzzle size.
    #[must_use]
    fn is_valid_size(&self, size: Size) -> bool;

    /// Equivalent to [`Scrambler::try_scramble_with_rng`] using [`rand::rng`].
    #[cfg(feature = "thread_rng")]
    fn try_scramble<P>(&self, puzzle: &mut P) -> bool
    where
        P: SlidingPuzzle,
    {
        self.try_scramble_with_rng(puzzle, &mut rand::rng())
    }

    /// Equivalent to [`Scrambler::scramble_with_rng`] using [`rand::rng`].
    #[cfg(feature = "thread_rng")]
    fn scramble<P>(&self, puzzle: &mut P)
    where
        P: SlidingPuzzle,
    {
        self.scramble_with_rng(puzzle, &mut rand::rng());
    }

    /// Scrambles the puzzle using a given [`Rng`]. If the puzzle is not of a valid size for the
    /// scrambler, the function returns false and the puzzle is not modified.
    fn try_scramble_with_rng<P, R>(&self, puzzle: &mut P, rng: &mut R) -> bool
    where
        P: SlidingPuzzle,
        R: Rng,
    {
        if self.is_valid_size(puzzle.size()) {
            self.scramble_with_rng(puzzle, rng);
            true
        } else {
            false
        }
    }

    /// See [`Scrambler::try_scramble_with_rng`].
    ///
    /// This function may not check whether the puzzle is of a valid size for the scrambler. If it
    /// is not, then the function may panic or scramble the puzzle into an unsolvable or invalid
    /// state.
    fn scramble_with_rng<P, R>(&self, puzzle: &mut P, rng: &mut R)
    where
        P: SlidingPuzzle,
        R: Rng;
}

/// Random state scrambler, but leaving the gap in the bottom right corner so that the resulting
/// state is invertible.
///
/// See [`RandomState`].
#[derive(Clone, Default, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RandomInvertibleState;

impl Scrambler for RandomInvertibleState {
    fn is_valid_size(&self, size: Size) -> bool {
        size.width() > 1 && size.height() > 1
    }

    fn scramble_with_rng<P, R>(&self, puzzle: &mut P, rng: &mut R)
    where
        P: SlidingPuzzle,
        R: Rng,
    {
        puzzle.reset();

        let n = puzzle.num_pieces();
        let mut parity = false;
        for i in 0..n - 2 {
            // Pick random element to go in position i
            let j = rng.random_range(i..n);

            // Swap and check if we need to toggle parity
            if i != j {
                puzzle.swap_non_gap_pieces(i, j);
                parity = !parity;
            }
        }

        // Swap the last two pieces if necessary to make it solvable
        if parity {
            puzzle.swap_non_gap_pieces(n - 2, n - 1);
        }
    }
}

/// Random state scrambler. Scrambles the puzzle in such a way that every solvable state is equally
/// likely to occur.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RandomState;

impl Scrambler for RandomState {
    fn is_valid_size(&self, _size: Size) -> bool {
        true
    }

    fn scramble_with_rng<P, R>(&self, puzzle: &mut P, rng: &mut R)
    where
        P: SlidingPuzzle,
        R: Rng,
    {
        let (w, h) = puzzle.size().into();

        if w == 1 {
            puzzle.reset();
            let d = rng.random_range(0..h);
            puzzle.apply_move(Move::new(Direction::Down, d));
            return;
        }

        if h == 1 {
            puzzle.reset();
            let r = rng.random_range(0..w);
            puzzle.apply_move(Move::new(Direction::Right, r));
            return;
        }

        RandomInvertibleState.scramble_with_rng(puzzle, rng);

        // Move blank to a random position
        let (d, r) = (rng.random_range(0..h), rng.random_range(0..w));

        puzzle.apply_move(Move::new(Direction::Down, d));
        puzzle.apply_move(Move::new(Direction::Right, r));
    }
}

/// Scrambles the puzzle by applying a fixed number of random single-tile moves.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RandomMoves {
    /// Number of random moves to apply.
    pub moves: u64,
    /// Are backtracking moves allowed? E.g. If one move of the scramble is R, is the next move
    /// allowed to be L? If this is false, the L move will not be allowed and a different move will
    /// be generated.
    pub allow_backtracking: bool,
    /// Are illegal moves counted? E.g. If the first generated move of the scramble is L (which
    /// can not be applied to the puzzle), should this be counted toward the total move count? If
    /// this is false, the L move will not be counted and a different move will be generated.
    pub allow_illegal_moves: bool,
}

impl Scrambler for RandomMoves {
    fn is_valid_size(&self, size: Size) -> bool {
        // If the puzzle is 1xn or nx1 and we don't allow backtracking or illegal moves, then after
        // n-1 moves, there will be no legal moves, and the `while` loop in `scramble_with_rng`
        // would loop forever.
        if (size.width() == 1 || size.height() == 1)
            && !self.allow_backtracking
            && !self.allow_illegal_moves
        {
            self.moves < size.area()
        } else {
            true
        }
    }

    fn scramble_with_rng<P, R>(&self, puzzle: &mut P, rng: &mut R)
    where
        P: SlidingPuzzle,
        R: Rng,
    {
        puzzle.reset();

        let mut last_dir = None::<Direction>;
        for _ in 0..self.moves {
            let dir = {
                let mut d = rng.random::<Direction>();
                while (!self.allow_backtracking && last_dir == Some(d.inverse()))
                    || (!self.allow_illegal_moves && !puzzle.can_move_dir(d))
                {
                    d = rng.random();
                }
                d
            };

            // An illegal move is counted but not applied, so it doesn't count as the last move for
            // the backtracking check.
            if puzzle.try_move_dir(dir) {
                last_dir = Some(dir);
            }
        }
    }
}

/// Scrambler that applies a single cycle of pieces to the puzzle. If the cycle length is even, the
/// last two pieces in the puzzle will also be swapped to make it solvable.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Cycle {
    /// Length of the cycle.
    ///
    /// If the cycle doesn't fit on the puzzle (including the two extra pieces needed when the
    /// length is even), the length is reduced to the largest length that does fit. Lengths 0 and
    /// 1 leave the puzzle solved.
    pub length: u64,
}

impl Scrambler for Cycle {
    fn is_valid_size(&self, size: Size) -> bool {
        // We can't do any cycles on a 1xn or nx1 puzzle.
        size.width() > 1 && size.height() > 1
    }

    fn scramble_with_rng<P, R>(&self, puzzle: &mut P, rng: &mut R)
    where
        P: SlidingPuzzle,
        R: Rng,
    {
        puzzle.reset();

        let n = puzzle.num_pieces();

        // An even length cycle is an odd permutation, so it needs the last two pieces to be
        // swapped as well, and the cycle must leave those two pieces alone. If there is no room
        // for that, use the next shorter (odd) length instead.
        let mut cycle_len = self.length.min(n);
        if cycle_len.is_multiple_of(2) && cycle_len > n - 2 {
            cycle_len -= 1;
        }

        let needs_swap = cycle_len >= 2 && cycle_len.is_multiple_of(2);
        let max = if needs_swap { n - 2 } else { n };
        let pieces = rand::seq::index::sample(rng, max as usize, cycle_len as usize);

        for i in 1..cycle_len as usize {
            puzzle.try_swap_pieces(pieces.index(0) as u64, pieces.index(i) as u64);
        }

        if needs_swap {
            puzzle.try_swap_pieces(n - 2, n - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzle::puzzle::Puzzle;

    mod random_invertible_state {
        use rand::SeedableRng as _;
        use rand_xoshiro::Xoroshiro128StarStar;

        use super::*;

        const SEED: [u8; 16] = [
            160, 108, 126, 255, 147, 210, 122, 252, 71, 77, 144, 13, 167, 11, 225, 93,
        ];

        #[test]
        fn test_gap_in_bottom_right() {
            let mut rng = Xoroshiro128StarStar::from_seed(SEED);

            for s in 2..10 {
                let mut p = Puzzle::new(Size::new(s, s).unwrap());

                for _ in 0..100 {
                    RandomInvertibleState.scramble_with_rng(&mut p, &mut rng);
                    assert_eq!(p.gap_position_xy(), (s - 1, s - 1));
                }
            }
        }
    }

    mod random_state {
        use rand::SeedableRng as _;
        use rand_xoshiro::Xoroshiro128StarStar;

        use super::*;
        use crate::puzzle::{label::label::RowGrids, solvable::Solvable as _};

        const SEED: [u8; 16] = [
            160, 108, 126, 255, 147, 210, 122, 252, 71, 77, 144, 13, 167, 11, 225, 93,
        ];

        #[test]
        fn test_solvable() {
            let mut rng = Xoroshiro128StarStar::from_seed(SEED);

            for (w, h) in [(1, 1), (1, 4), (4, 1), (2, 2), (4, 4), (10, 2), (20, 20)] {
                let mut p = Puzzle::new(Size::new(w, h).unwrap());

                for _ in 0..100 {
                    RandomState.scramble_with_rng(&mut p, &mut rng);
                    assert!(RowGrids.is_solvable(&p));
                }
            }
        }
    }

    mod random_moves {
        use rand::SeedableRng as _;
        use rand_xoshiro::Xoroshiro128StarStar;

        use super::*;

        const SEED: [u8; 16] = [
            160, 108, 126, 255, 147, 210, 122, 252, 71, 77, 144, 13, 167, 11, 225, 93,
        ];

        #[test]
        fn test_illegal_moves_do_not_reset_backtracking() {
            let mut rng = Xoroshiro128StarStar::from_seed(SEED);
            let mut p = Puzzle::new(Size::new(2, 1).unwrap());
            let scrambler = RandomMoves {
                moves: 50,
                allow_backtracking: false,
                allow_illegal_moves: true,
            };

            // The only legal move on the solved 2x1 puzzle is R, and after that the only legal move
            // is L, which would be backtracking. So once R has been applied, the puzzle must stay
            // in the state "0 1", even if illegal moves are generated in between.
            for _ in 0..200 {
                scrambler.scramble_with_rng(&mut p, &mut rng);
                assert_eq!(p.to_string(), "0 1");
            }
        }
    }

    mod cycle {
        use rand::SeedableRng as _;
        use rand_xoshiro::Xoroshiro128StarStar;

        use super::*;
        use crate::puzzle::{label::label::RowGrids, solvable::Solvable as _};

        const SEED: [u8; 16] = [
            160, 108, 126, 255, 147, 210, 122, 252, 71, 77, 144, 13, 167, 11, 225, 93,
        ];

        fn num_unsolved_pieces(p: &Puzzle) -> u64 {
            (0..p.num_pieces())
                .filter(|&i| p.solved_pos(p.piece_at(i)) != i)
                .count() as u64
        }

        #[test]
        fn test_solvable() {
            let mut rng = Xoroshiro128StarStar::from_seed(SEED);

            for (w, h) in [(2, 2), (3, 2), (2, 3), (3, 3), (4, 3), (4, 4), (5, 5)] {
                let mut p = Puzzle::new(Size::new(w, h).unwrap());
                let n = p.num_pieces();

                for length in 0..n + 5 {
                    for _ in 0..20 {
                        Cycle { length }.scramble_with_rng(&mut p, &mut rng);
                        assert!(
                            RowGrids.is_solvable(&p),
                            "{w}x{h} puzzle, length {length}: {p} is unsolvable"
                        );
                        assert_eq!(p.gap_position(), n);
                    }
                }
            }
        }

        #[test]
        fn test_num_unsolved_pieces() {
            let mut rng = Xoroshiro128StarStar::from_seed(SEED);
            let mut p = Puzzle::new(Size::new(4, 4).unwrap());

            // (length, number of pieces moved by the cycle and the extra swap)
            let cases = [
                (0, 0),
                (1, 0),
                (2, 4),
                (3, 3),
                (12, 14),
                (13, 13),
                (14, 13),
                (15, 15),
                (16, 15),
                (100, 15),
            ];

            for (length, expected) in cases {
                for _ in 0..20 {
                    Cycle { length }.scramble_with_rng(&mut p, &mut rng);
                    assert_eq!(num_unsolved_pieces(&p), expected, "length {length}: {p}");
                }
            }
        }
    }
}
