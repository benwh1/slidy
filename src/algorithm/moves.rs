//! Defines iterators over the moves of an [`AlgorithmSlice`].

use crate::algorithm::{r#move::r#move::Move, slice::AlgorithmSlice};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum State {
    #[default]
    First,
    Middle(usize),
    Last,
    Finished,
}

/// Iterator over the moves of an [`AlgorithmSlice`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moves<'a> {
    slice: AlgorithmSlice<'a>,
    iter_state: State,
}

impl<'a> Moves<'a> {
    pub(super) fn new(slice: AlgorithmSlice<'a>) -> Self {
        Self {
            slice,
            iter_state: State::First,
        }
    }
}

impl Iterator for Moves<'_> {
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let f = self.slice.first.is_some() as usize;
        let m = self.slice.middle.len();
        let l = self.slice.last.is_some() as usize;

        let len = match self.iter_state {
            State::First => f + m + l,
            State::Middle(n) => m - n + l,
            State::Last => l,
            State::Finished => 0,
        };

        (len, Some(len))
    }
}

impl ExactSizeIterator for Moves<'_> {}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use crate::algorithm::{algorithm::Algorithm, r#move::r#move::Move};

    #[test]
    fn test_multi_tile_moves() -> Result<(), Box<dyn std::error::Error>> {
        let alg = Algorithm::from_str("R3D2LDR5U12RD3LU4R")?;
        let slice = alg.try_slice(4..19)?;
        let mut moves = slice.moves();

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

        let mut iter = slice.moves();
        for i in 0..7 {
            assert_eq!(iter.len(), 6 - i);
            iter.next();
        }

        Ok(())
    }
}
