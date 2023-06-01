//! Defines iterators over the moves of an [`AlgorithmSlice`].

use crate::algorithm::{r#move::r#move::Move, slice::AlgorithmSlice};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum State {
    #[default]
    First,
    Middle(usize),
    Last,
    Finished,
}

/// Iterator over the moves of an [`AlgorithmSlice`]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MultiTileMoves<'a> {
    slice: AlgorithmSlice<'a>,
    iter_state: State,
}

impl<'a> MultiTileMoves<'a> {
    pub(super) fn new(slice: AlgorithmSlice<'a>) -> Self {
        Self {
            slice,
            iter_state: State::First,
        }
    }
}

impl Iterator for MultiTileMoves<'_> {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter_state {
            State::First => {
                self.iter_state = State::Middle(0);
                self.slice.first.or_else(|| self.next())
            }
            State::Middle(n) => {
                if self.slice.middle.is_empty() {
                    self.iter_state = State::Last;
                    self.next()
                } else {
                    self.iter_state = if n + 1 == self.slice.middle.len() {
                        State::Last
                    } else {
                        State::Middle(n + 1)
                    };
                    Some(self.slice.middle[n])
                }
            }
            State::Last => {
                self.iter_state = State::Finished;
                self.slice.last
            }
            State::Finished => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::algorithm::{algorithm::Algorithm, r#move::r#move::Move};

    #[test]
    fn test_multi_tile_moves() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;
        let slice = alg.try_slice(4..19)?;
        let mut moves = slice.multi_tile_moves();

        assert_eq!(moves.next(), Some(Move::from_str("D")?));
        assert_eq!(moves.next(), Some(Move::from_str("L")?));
        assert_eq!(moves.next(), Some(Move::from_str("D")?));
        assert_eq!(moves.next(), Some(Move::from_str("R5")?));
        assert_eq!(moves.next(), Some(Move::from_str("U7")?));
        assert_eq!(moves.next(), None);
        assert_eq!(moves.next(), None);

        Ok(())
    }
}
