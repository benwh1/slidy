use super::puzzle_move::{DisplayMove, MoveDisplay};
use crate::algorithm::algorithm::Algorithm;
use std::{fmt::Display, marker::PhantomData};

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

impl<'a, T1, T2> DisplayAlgorithm<'a, T1, T2> {
    pub fn new(alg: &'a Algorithm) -> Self {
        Self {
            alg,
            phantom_t1: PhantomData,
            phantom_t2: PhantomData,
        }
    }
}

impl<'a, T1, T2> Display for DisplayAlgorithm<'a, T1, T2>
where
    DisplayAlgorithm<'a, T1, T2>: AlgorithmDisplay,
    DisplayMove<'a, T2>: MoveDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        AlgorithmDisplay::fmt(self, f)
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

impl<'a, T> AlgorithmDisplay for DisplayAlgorithm<'a, DisplayUnspaced, T>
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
                .collect::<String>()
        )
    }
}
