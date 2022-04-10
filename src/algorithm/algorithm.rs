use super::puzzle_move::{Move, MoveDisplay};
use std::{fmt::Display, marker::PhantomData};

pub struct Algorithm {
    moves: Vec<Move>,
}

pub struct DisplaySpaced;
pub struct DisplayUnspaced;

pub trait AlgorithmDisplay {}

impl AlgorithmDisplay for DisplaySpaced {}
impl AlgorithmDisplay for DisplayUnspaced {}

pub struct DisplayAlgorithm<'a, T1: AlgorithmDisplay, T2: MoveDisplay> {
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
