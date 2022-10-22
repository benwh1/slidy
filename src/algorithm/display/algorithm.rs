//! Defines ways in which an [`Algorithm`] can be displayed.

use super::puzzle_move::MoveDisplay;
use crate::algorithm::algorithm::Algorithm;
use std::{fmt::Display, marker::PhantomData};

/// Marker trait for structs that are used to display algorithms
pub trait AlgorithmDisplay<'a> {
    /// Create a new [`AlgorithmDisplay`] for displaying `algorithm`.
    #[must_use]
    fn new(algorithm: &'a Algorithm) -> Self;
}

macro_rules! define_display {
    ($(#[$annot:meta] $name:ident),* $(,)?) => {
        $(
            #[$annot]
            pub struct $name<'a, T: MoveDisplay + Display> {
                algorithm: &'a Algorithm,
                phantom_t: PhantomData<T>,
            }

            impl<'a, T: MoveDisplay + Display> AlgorithmDisplay<'a> for $name<'a, T> {
                fn new(algorithm: &'a Algorithm) -> Self {
                    Self {
                        algorithm,
                        phantom_t: PhantomData,
                    }
                }
            }
        )*
    };
}

define_display!(
    /// Formats each move of the algorithm using `T` and adds a space between moves.
    DisplaySpaced,
    /// Formats each move of the algorithm using `T`.
    DisplayUnspaced
);

impl<'a, T: MoveDisplay + Display> Display for DisplaySpaced<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            &self
                .algorithm
                .iter_moves()
                .map(|m| T::new(m).to_string())
                .intersperse(" ".to_string())
                .collect::<String>(),
        )
    }
}

impl<'a, T: MoveDisplay + Display> Display for DisplayUnspaced<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for m in self.algorithm.iter_moves() {
            T::new(m).fmt(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::algorithm::{
        algorithm::Algorithm,
        display::{
            algorithm::{AlgorithmDisplay, DisplaySpaced, DisplayUnspaced},
            puzzle_move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
        },
    };
    use std::str::FromStr;

    #[test]
    fn test_display() {
        let a = Algorithm::from_str("U2RDL3").unwrap();
        let d1 = DisplaySpaced::<DisplayLongSpaced>::new(&a).to_string();
        let d2 = DisplaySpaced::<DisplayLongUnspaced>::new(&a).to_string();
        let d3 = DisplaySpaced::<DisplayShort>::new(&a).to_string();
        let d4 = DisplayUnspaced::<DisplayLongSpaced>::new(&a).to_string();
        let d5 = DisplayUnspaced::<DisplayLongUnspaced>::new(&a).to_string();
        let d6 = DisplayUnspaced::<DisplayShort>::new(&a).to_string();
        assert_eq!(d1, "U U R D L L L");
        assert_eq!(d2, "UU R D LLL");
        assert_eq!(d3, "U2 R D L3");
        assert_eq!(d4, "U URDL L L");
        assert_eq!(d5, "UURDLLL");
        assert_eq!(d6, "U2RDL3");
    }
}

#[cfg(test)]
mod benchmarks {
    extern crate test;

    use std::str::FromStr;

    use test::Bencher;

    use crate::algorithm::display::puzzle_move::DisplayShort;

    use super::*;

    #[bench]
    fn bench_display_spaced_display_short(b: &mut Bencher) {
        let a = Algorithm::from_str(
            "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2R3D3",
        )
        .unwrap();

        b.iter(|| DisplaySpaced::<DisplayShort>::new(&a).to_string());
    }

    #[bench]
    fn bench_display_unspaced_display_short(b: &mut Bencher) {
        let a = Algorithm::from_str(
            "DR2D2LULURUR2DL2DRU2RD2LDRULULDRDL2URDLU3RDLUR3DLDLU2RD3LU3R2DLD2LULU2R3D3",
        )
        .unwrap();

        b.iter(|| DisplayUnspaced::<DisplayShort>::new(&a).to_string());
    }
}
