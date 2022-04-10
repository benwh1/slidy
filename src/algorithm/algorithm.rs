use super::puzzle_move::{DisplayMove, Move, MoveDisplay};
use std::marker::PhantomData;

pub struct Algorithm {
    moves: Vec<Move>,
}

pub struct DisplaySpaced;
pub struct DisplayUnspaced;

pub trait AlgorithmDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

pub struct DisplayAlgorithm<'a, T1, T2> {
    alg: &'a Algorithm,
    phantom_t1: PhantomData<T1>,
    phantom_t2: PhantomData<T2>,
}

impl Algorithm {
    fn new(moves: Vec<Move>) -> Self {
        Self { moves }
    }

    fn length(&self) -> u32 {
        self.moves.iter().map(|m| m.amount).sum()
    }

    fn push(&mut self, m: Move) {
        self.moves.push(m)
    }
}

impl<'a, T> AlgorithmDisplay for DisplayAlgorithm<'a, DisplaySpaced, T>
where
    DisplayMove<'a, T>: MoveDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.alg
                .moves
                .iter()
                .map(|m| m.display::<T>().to_string())
                .intersperse(" ".to_string())
                .collect::<String>()
        )
    }
}
