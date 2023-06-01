//! Defines the [`AlgorithmSlice`] type.

use std::iter;

use crate::algorithm::{direction::Direction, moves::MultiTileMoves, r#move::r#move::Move};

/// A slice of an [`Algorithm`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlgorithmSlice<'a> {
    // We might slice in the middle of one move, e.g. D10LU10R[5..15] should be D5LU4. To represent
    // this, we need to store the first and last moves separately.
    pub(super) first: Option<Move>,
    pub(super) middle: &'a [Move],
    pub(super) last: Option<Move>,
}

impl AlgorithmSlice<'_> {
    /// The length of the slice in single tile moves.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.multi_tile_moves().map(|m| m.amount).sum()
    }

    /// Checks if the slice is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.first.is_none() && self.middle.is_empty() && self.last.is_none()
    }

    /// An iterator over the single-tile moves in the slice.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::str::FromStr;
    /// # use slidy::algorithm::{algorithm::Algorithm, direction::Direction};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let alg = Algorithm::from_str("RD3LUR2")?;
    /// let slice = alg.as_slice();
    ///
    /// let mut iter = slice.single_tile_moves();
    /// assert_eq!(iter.next(), Some(Direction::Right));
    /// assert_eq!(iter.next(), Some(Direction::Down));
    /// assert_eq!(iter.next(), Some(Direction::Down));
    /// assert_eq!(iter.next(), Some(Direction::Down));
    /// assert_eq!(iter.next(), Some(Direction::Left));
    /// assert_eq!(iter.next(), Some(Direction::Up));
    /// assert_eq!(iter.next(), Some(Direction::Right));
    /// assert_eq!(iter.next(), Some(Direction::Right));
    /// assert_eq!(iter.next(), None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn single_tile_moves(&self) -> impl Iterator<Item = Direction> + '_ {
        self.multi_tile_moves()
            .flat_map(|m| iter::repeat(m.direction).take(m.amount as usize))
    }

    /// An iterator over the multi-tile moves in the slice.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::str::FromStr;
    /// # use slidy::algorithm::{algorithm::Algorithm, direction::Direction, r#move::r#move::Move};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let alg = Algorithm::from_str("RD3LUR2")?;
    /// let slice = alg.as_slice();
    ///
    /// let mut iter = slice.multi_tile_moves();
    /// assert_eq!(iter.next(), Some(Move::new(Direction::Right, 1)));
    /// assert_eq!(iter.next(), Some(Move::new(Direction::Down, 3)));
    /// assert_eq!(iter.next(), Some(Move::new(Direction::Left, 1)));
    /// assert_eq!(iter.next(), Some(Move::new(Direction::Up, 1)));
    /// assert_eq!(iter.next(), Some(Move::new(Direction::Right, 2)));
    /// assert_eq!(iter.next(), None);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn multi_tile_moves(&self) -> MultiTileMoves {
        MultiTileMoves::new(self)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::algorithm::{algorithm::Algorithm, direction::Direction};

    #[test]
    fn test_len() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;

        for start in 0..34 {
            for end in start..34 {
                let slice = alg.try_slice(start..end)?;
                assert_eq!(slice.len(), end - start);
            }
        }

        Ok(())
    }

    #[test]
    fn test_is_empty() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;

        for start in 0..34 {
            for end in start..34 {
                let slice = alg.try_slice(start..end)?;
                assert_eq!(slice.is_empty(), start == end);
            }
        }

        Ok(())
    }

    #[test]
    fn test_single_tile_moves() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;
        let slice = alg.try_slice(4..19)?;
        let mut moves = slice.single_tile_moves();

        assert_eq!(moves.next(), Some(Direction::Down));
        assert_eq!(moves.next(), Some(Direction::Left));
        assert_eq!(moves.next(), Some(Direction::Down));
        assert_eq!(moves.next(), Some(Direction::Right));
        assert_eq!(moves.next(), Some(Direction::Right));
        assert_eq!(moves.next(), Some(Direction::Right));
        assert_eq!(moves.next(), Some(Direction::Right));
        assert_eq!(moves.next(), Some(Direction::Right));
        assert_eq!(moves.next(), Some(Direction::Up));
        assert_eq!(moves.next(), Some(Direction::Up));
        assert_eq!(moves.next(), Some(Direction::Up));
        assert_eq!(moves.next(), Some(Direction::Up));
        assert_eq!(moves.next(), Some(Direction::Up));
        assert_eq!(moves.next(), Some(Direction::Up));
        assert_eq!(moves.next(), Some(Direction::Up));
        assert_eq!(moves.next(), None);
        assert_eq!(moves.next(), None);

        Ok(())
    }
}
