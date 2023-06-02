//! Defines the [`AsAlgorithmSlice`] trait.

use crate::algorithm::{algorithm::Algorithm, slice::AlgorithmSlice};

/// Conversion to [`AlgorithmSlice`].
pub trait AsAlgorithmSlice<'a> {
    /// Converts `self` to an [`AlgorithmSlice`].
    fn as_slice(&'a self) -> AlgorithmSlice<'a>;
}

impl<'a> AsAlgorithmSlice<'a> for Algorithm {
    /// Returns an [`AlgorithmSlice`] containing the entire algorithm.
    fn as_slice(&'a self) -> AlgorithmSlice<'a> {
        AlgorithmSlice {
            first: None,
            middle: &self.moves,
            last: None,
        }
    }
}

impl<'a> AsAlgorithmSlice<'a> for AlgorithmSlice<'a> {
    fn as_slice(&'a self) -> AlgorithmSlice<'a> {
        *self
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::algorithm::{
        algorithm::Algorithm, as_slice::AsAlgorithmSlice, r#move::r#move::Move,
        slice::AlgorithmSlice,
    };

    #[test]
    fn test_algorithm() {
        let a = Algorithm::from_str("U2LD3R").unwrap();
        let b = a.as_slice();
        assert_eq!(
            b,
            AlgorithmSlice {
                first: None,
                middle: &a.moves,
                last: None
            }
        );
    }

    #[test]
    fn test_algorithm_slice() -> Result<(), Box<dyn std::error::Error>> {
        let a = AlgorithmSlice {
            first: Some(Move::from_str("R3")?),
            middle: &[Move::from_str("D")?, Move::from_str("L2")?],
            last: Some(Move::from_str("U6")?),
        };
        let b = a.as_slice();

        assert_eq!(a, b);

        // Make sure we aren't somehow doing a deep copy
        assert_eq!(a.middle.as_ptr(), b.middle.as_ptr());

        Ok(())
    }
}
