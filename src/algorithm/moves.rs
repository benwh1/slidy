//! Defines iterators over the moves of an [`AlgorithmSlice`].

use std::{
    iter::{Chain, Copied, FusedIterator},
    option, slice,
};

use crate::algorithm::{r#move::r#move::Move, slice::AlgorithmSlice};

type Inner<'a> =
    Chain<Chain<option::IntoIter<Move>, Copied<slice::Iter<'a, Move>>>, option::IntoIter<Move>>;

/// Iterator over the moves of an [`AlgorithmSlice`].
#[derive(Clone, Debug)]
pub struct Moves<'a>(Inner<'a>);

impl<'a> Moves<'a> {
    /// Creates a new [`Moves`] iterator over the given [`AlgorithmSlice`].
    #[must_use]
    pub fn new(slice: AlgorithmSlice<'a>) -> Self {
        Self(
            slice
                .first
                .into_iter()
                .chain(slice.middle.iter().copied())
                .chain(slice.last),
        )
    }
}

impl Iterator for Moves<'_> {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl DoubleEndedIterator for Moves<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

// The length is at most `middle.len() + 2`, which can't overflow because `middle` is a slice of
// `Move`s, so the size hint is always exact.
impl ExactSizeIterator for Moves<'_> {}

impl FusedIterator for Moves<'_> {}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use crate::algorithm::{algorithm::Algorithm, r#move::r#move::Move};

    #[test]
    fn test_multi_tile_moves() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;
        let slice = alg.try_slice(4..19)?;
        let mut moves = slice.moves_mtm();

        assert_eq!(moves.next(), Some(Move::from_str("D")?));
        assert_eq!(moves.next(), Some(Move::from_str("L")?));
        assert_eq!(moves.next(), Some(Move::from_str("D")?));
        assert_eq!(moves.next(), Some(Move::from_str("R5")?));
        assert_eq!(moves.next(), Some(Move::from_str("U7")?));
        assert_eq!(moves.next(), None);
        assert_eq!(moves.next(), None);

        Ok(())
    }

    #[test]
    fn test_exact_size_iterator() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;
        let slice = alg.try_slice(2..20)?;

        let mut iter = slice.moves_mtm();
        for i in 0..7 {
            assert_eq!(iter.len(), 6 - i);
            iter.next();
        }

        Ok(())
    }

    #[test]
    fn test_double_ended() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;
        let slice = alg.try_slice(4..19)?;

        let forward = slice.moves_mtm().collect::<Vec<_>>();
        let mut backward = slice.moves_mtm().rev().collect::<Vec<_>>();
        backward.reverse();
        assert_eq!(forward, backward);

        // Take from both ends; the first and last moves are partial moves.
        let mut moves = slice.moves_mtm();
        assert_eq!(moves.next(), Some(Move::from_str("D")?));
        assert_eq!(moves.next_back(), Some(Move::from_str("U7")?));
        assert_eq!(moves.len(), 3);
        assert_eq!(moves.next_back(), Some(Move::from_str("R5")?));
        assert_eq!(moves.next(), Some(Move::from_str("L")?));
        assert_eq!(moves.next(), Some(Move::from_str("D")?));
        assert_eq!(moves.next(), None);
        assert_eq!(moves.next_back(), None);
        assert_eq!(moves.len(), 0);

        Ok(())
    }
}
