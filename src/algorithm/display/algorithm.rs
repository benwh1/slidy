use super::puzzle_move::DisplayMove;
use crate::algorithm::algorithm::Algorithm;
use std::{fmt::Display, marker::PhantomData};

pub struct DisplaySpaced;
pub struct DisplayUnspaced;

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

impl<'a, T> Display for DisplayAlgorithm<'a, DisplaySpaced, T>
where
    DisplayMove<'a, T>: Display,
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

impl<'a, T> Display for DisplayAlgorithm<'a, DisplayUnspaced, T>
where
    DisplayMove<'a, T>: Display,
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

#[cfg(test)]
mod tests {
    use crate::algorithm::{
        algorithm::Algorithm,
        display::{
            algorithm::{DisplayAlgorithm, DisplaySpaced, DisplayUnspaced},
            puzzle_move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
        },
    };
    use std::str::FromStr;

    #[test]
    fn test_display() {
        let a = Algorithm::from_str("U2RDL3").unwrap();
        let d1 = DisplayAlgorithm::<DisplaySpaced, DisplayLongSpaced>::new(&a).to_string();
        let d2 = DisplayAlgorithm::<DisplaySpaced, DisplayLongUnspaced>::new(&a).to_string();
        let d3 = DisplayAlgorithm::<DisplaySpaced, DisplayShort>::new(&a).to_string();
        let d4 = DisplayAlgorithm::<DisplayUnspaced, DisplayLongSpaced>::new(&a).to_string();
        let d5 = DisplayAlgorithm::<DisplayUnspaced, DisplayLongUnspaced>::new(&a).to_string();
        let d6 = DisplayAlgorithm::<DisplayUnspaced, DisplayShort>::new(&a).to_string();
        assert_eq!(d1, "U U R D L L L");
        assert_eq!(d2, "UU R D LLL");
        assert_eq!(d3, "U2 R D L3");
        assert_eq!(d4, "U URDL L L");
        assert_eq!(d5, "UURDLLL");
        assert_eq!(d6, "U2RDL3");
    }
}
