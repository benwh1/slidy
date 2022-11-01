//! Defines the [`Solver`] struct for computing optimal solutions.

use std::marker::PhantomData;

use num_traits::PrimInt;

use crate::{
    algorithm::{algorithm::Algorithm, direction::Direction, puzzle_move::Move},
    puzzle::sliding_puzzle::SlidingPuzzle,
};

use super::heuristic::{Heuristic, ManhattanDistance};

/// An optimal puzzle solver.
pub struct Solver<'a, Piece, Puzzle>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
{
    puzzle: &'a mut Puzzle,
    stack: Stack,
    phantom_piece: PhantomData<Piece>,
}

impl<'a, Piece, Puzzle> Solver<'a, Piece, Puzzle>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
{
    /// Constructs a new [`Solver`] for solving `puzzle`.
    pub fn new(puzzle: &'a mut Puzzle) -> Self {
        Self {
            puzzle,
            stack: Stack::default(),
            phantom_piece: PhantomData,
        }
    }
}

struct Stack {
    stack: [Direction; 256],
    idx: usize,
}

impl Stack {
    fn push(&mut self, direction: Direction) {
        self.stack[self.idx] = direction;
        self.idx += 1;
    }

    fn top(&self) -> Option<Direction> {
        if self.idx == 0 {
            None
        } else {
            Some(self.stack[self.idx - 1])
        }
    }

    fn pop(&mut self) -> Direction {
        self.idx -= 1;
        self.stack[self.idx]
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            stack: [Direction::Up; 256],
            idx: 0,
        }
    }
}

impl From<&Stack> for Algorithm {
    fn from(stack: &Stack) -> Self {
        Self::new(
            stack.stack[..stack.idx]
                .iter()
                .map(|d| Move::from(*d))
                .collect(),
        )
    }
}

impl<'a, Piece, Puzzle> Solver<'a, Piece, Puzzle>
where
    Piece: PrimInt,
    Puzzle: SlidingPuzzle<Piece>,
{
    fn dfs(&mut self, depth: u8) -> bool {
        if depth == 0 {
            if self.puzzle.is_solved() {
                println!("{}", Algorithm::from(&self.stack));
                return true;
            }
            return false;
        }

        let bound: u8 = ManhattanDistance.bound(self.puzzle);

        if bound > depth {
            return false;
        }

        use Direction::*;
        for d in [Up, Left, Down, Right] {
            if self.stack.top() == Some(d.inverse()) {
                continue;
            }

            if !self.puzzle.move_dir(d) {
                continue;
            }

            self.stack.push(d);

            if self.dfs(depth - 1) {
                return true;
            }

            self.stack.pop();
            self.puzzle.move_dir(d.inverse());
        }

        false
    }

    /// Solves the puzzle.
    pub fn solve(&mut self) {
        let bound: u8 = ManhattanDistance.bound(self.puzzle);
        for b in (bound..u8::MAX).step_by(2) {
            if self.dfs(b) {
                break;
            }
        }
    }
}
