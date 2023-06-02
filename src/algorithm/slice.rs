//! Defines the [`AlgorithmSlice`] type.

use std::{fmt::Display, iter};

use crate::algorithm::{
    algorithm::Algorithm,
    direction::Direction,
    display::{
        algorithm::{AlgorithmDisplay, DisplaySpaced, DisplayUnspaced},
        r#move::{DisplayLongSpaced, DisplayLongUnspaced, DisplayShort},
    },
    moves::MultiTileMoves,
    r#move::r#move::{Move, MoveSum},
};

/// A slice of an [`Algorithm`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// Combines all consecutive moves along the same axis into a single move, and removes any moves
    /// that cancel completely. Returns the result as a new [`Algorithm`].
    #[must_use]
    pub fn simplified(&self) -> Algorithm {
        if self.multi_tile_moves().count() < 2 {
            return Algorithm::from(*self);
        }

        // List of simplified moves
        let mut moves = Vec::new();

        // Current move that we are accumulating into. This will be pushed to `moves` when we
        // reach a move that can't be added to it.
        let mut acc_move = None;

        for next in self.multi_tile_moves() {
            match acc_move {
                Some(sum) => match sum + next {
                    MoveSum::Ok(m) => {
                        // Moves completely cancel.
                        acc_move = if m.amount == 0 {
                            // Try and pop a move off `moves`, because the next move might cancel.
                            // e.g. consider URLD where `next` is the L move. We pop the U move
                            // from `moves` so that the following D move can cancel with it.
                            moves.pop()
                        }
                        // Moves can be added but don't fully cancel, keep accumulating into mv.
                        else {
                            Some(m)
                        };
                    }
                    // Moves can't be added, there is no more simplification at this point.
                    MoveSum::Invalid => {
                        // Push mv and go to the next move.
                        moves.push(sum);
                        acc_move = Some(next);
                    }
                },
                None => acc_move = Some(next),
            }
        }

        if let Some(m) = acc_move && m.amount != 0 {
            moves.push(m);
        }

        Algorithm::with_moves(moves)
    }

    /// Returns the result of reflecting the algorithm through the main diagonal as a new
    /// [`Algorithm`].
    #[must_use]
    pub fn transpose(&self) -> Algorithm {
        Algorithm::with_moves(self.multi_tile_moves().map(|m| m.transpose()).collect())
    }

    /// Concatenates `n` copies of `self` and returns the result as a new [`Algorithm`].
    #[must_use]
    pub fn repeat(&self, n: usize) -> Algorithm {
        let len = self.multi_tile_moves().len();
        Algorithm::with_moves(
            self.multi_tile_moves()
                .cycle()
                .take(len * n)
                .collect::<Vec<_>>(),
        )
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
        MultiTileMoves::new(*self)
    }

    /// Helper function for creating a [`DisplaySpaced<DisplayLongSpaced>`] around `self`.
    #[must_use]
    pub fn display_long_spaced(&self) -> DisplaySpaced<DisplayLongSpaced> {
        DisplaySpaced::<DisplayLongSpaced>::new(self)
    }

    /// Helper function for creating a [`DisplayUnspaced<DisplayLongUnspaced>`] around `self`.
    #[must_use]
    pub fn display_long_unspaced(&self) -> DisplayUnspaced<DisplayLongUnspaced> {
        DisplayUnspaced::<DisplayLongUnspaced>::new(self)
    }

    /// Helper function for creating a [`DisplaySpaced<DisplayShort>`] around `self`.
    #[must_use]
    pub fn display_short_spaced(&self) -> DisplaySpaced<DisplayShort> {
        DisplaySpaced::<DisplayShort>::new(self)
    }

    /// Helper function for creating a [`DisplayUnspaced<DisplayShort>`] around `self`.
    #[must_use]
    pub fn display_short_unspaced(&self) -> DisplayUnspaced<DisplayShort> {
        DisplayUnspaced::<DisplayShort>::new(self)
    }
}

impl Display for AlgorithmSlice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Default formatting is short, unspaced.
        self.display_short_unspaced().fmt(f)
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
